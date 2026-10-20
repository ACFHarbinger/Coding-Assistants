//! Install / status / remove lifecycle for the user timer pair.

use super::units::{render_service, render_timer, systemd_user_unit_dir, SERVICE_NAME, TIMER_NAME};
use super::{CommandOutput, CommandRunner, RealRunner, TriggerConfig, TriggerStatus};
use std::fs;
use std::path::Path;

const DIAGNOSTIC_CHARS: usize = 300;

fn argv(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|part| (*part).to_string()).collect()
}

fn bounded(output: &CommandOutput) -> String {
    let raw = if output.stderr.trim().is_empty() {
        output.stdout.trim()
    } else {
        output.stderr.trim()
    };
    let single_line = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    if single_line.chars().count() <= DIAGNOSTIC_CHARS {
        single_line
    } else {
        let truncated: String = single_line.chars().take(DIAGNOSTIC_CHARS).collect();
        format!("{truncated}…")
    }
}

fn probe(runner: &dyn CommandRunner) -> (bool, Option<String>) {
    if !cfg!(target_os = "linux") {
        return (
            false,
            Some("The systemd trigger is Linux-only; this build targets another OS.".to_string()),
        );
    }
    match runner.run("systemctl", &argv(&["--user", "show-environment"])) {
        Ok(output) if output.success => (true, None),
        Ok(output) => (
            false,
            Some(format!(
                "systemd user manager is unavailable (systemctl --user failed: {}). \
                 Start a user session or enable lingering for this account \
                 (`loginctl enable-linger`).",
                bounded(&output)
            )),
        ),
        Err(error) => (
            false,
            Some(format!(
                "systemd user manager is unavailable: {error}. \
                 This feature needs a Linux systemd user session."
            )),
        ),
    }
}

fn run_checked(runner: &dyn CommandRunner, parts: &[&str]) -> Result<(), String> {
    let (program, args) = parts
        .split_first()
        .ok_or_else(|| "empty systemctl invocation".to_string())?;
    let output = runner.run(program, &argv(args))?;
    if output.success {
        Ok(())
    } else {
        Err(bounded(&output))
    }
}

fn read_exec_start(service_path: &Path) -> Option<String> {
    let contents = fs::read_to_string(service_path).ok()?;
    contents
        .lines()
        .find_map(|line| line.strip_prefix("ExecStart=").map(str::to_string))
}

/// Snapshot the trigger state without changing anything.
pub fn status_at(unit_dir: &Path, runner: &dyn CommandRunner) -> TriggerStatus {
    let service_path = unit_dir.join(SERVICE_NAME);
    let timer_path = unit_dir.join(TIMER_NAME);
    let installed = service_path.is_file() && timer_path.is_file();
    let (available, detail) = probe(runner);
    let enabled = available
        && installed
        && runner
            .run("systemctl", &argv(&["--user", "is-enabled", TIMER_NAME]))
            .map(|output| output.success)
            .unwrap_or(false);
    let active = available
        && installed
        && runner
            .run("systemctl", &argv(&["--user", "is-active", TIMER_NAME]))
            .map(|output| output.success)
            .unwrap_or(false);
    TriggerStatus {
        available,
        installed,
        enabled,
        active,
        service_path: service_path.to_string_lossy().into_owned(),
        timer_path: timer_path.to_string_lossy().into_owned(),
        exec_start: read_exec_start(&service_path),
        detail,
    }
}

fn validate(config: &TriggerConfig) -> Result<(), String> {
    if config.interval_minutes == 0 {
        return Err("interval_minutes must be at least 1".to_string());
    }
    if !config.ca_bin.is_absolute() {
        return Err("ca_bin must be an absolute path".to_string());
    }
    if !config.ca_home.is_absolute() {
        return Err("ca_home must be an absolute path".to_string());
    }
    if !config.workspace.is_absolute() {
        return Err("workspace must be an absolute path".to_string());
    }
    if !config.ca_bin.is_file() {
        return Err(format!(
            "ca binary `{}` does not exist",
            config.ca_bin.display()
        ));
    }
    Ok(())
}

fn write_atomic(path: &Path, contents: &str) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("`{}` has no parent directory", path.display()))?;
    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "could not create systemd user unit directory `{}`: {error}",
            parent.display()
        )
    })?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("ca-schedule");
    let temporary = parent.join(format!(".{file_name}.tmp-{}", std::process::id()));
    fs::write(&temporary, contents)
        .map_err(|error| format!("could not write `{}`: {error}", temporary.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&temporary, fs::Permissions::from_mode(0o644)).map_err(|error| {
            let _ = fs::remove_file(&temporary);
            format!(
                "could not set permissions on `{}`: {error}",
                temporary.display()
            )
        })?;
    }
    if let Err(error) = fs::rename(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(format!("could not replace `{}`: {error}", path.display()));
    }
    Ok(())
}

fn install_reports_error(unit_dir: &Path, step: &str, error: String) -> String {
    format!(
        "unit files were written under `{}` but {step} failed: {error}. \
         Retry the install or remove the trigger from Settings.",
        unit_dir.display()
    )
}

/// Install or update both user units, then reload and enable the timer.
/// Repeated installs are idempotent. If enabling fails, the unit files are
/// left in place so the status is truthful and remove/retry still works.
pub fn install_at(
    unit_dir: &Path,
    config: &TriggerConfig,
    runner: &dyn CommandRunner,
) -> Result<TriggerStatus, String> {
    validate(config)?;
    let (available, detail) = probe(runner);
    if !available {
        return Err(detail.unwrap_or_else(|| "systemd user manager is unavailable".to_string()));
    }
    write_atomic(&unit_dir.join(SERVICE_NAME), &render_service(config))?;
    write_atomic(
        &unit_dir.join(TIMER_NAME),
        &render_timer(config.interval_minutes),
    )?;
    run_checked(runner, &["systemctl", "--user", "daemon-reload"])
        .map_err(|error| install_reports_error(unit_dir, "daemon-reload", error))?;
    run_checked(
        runner,
        &["systemctl", "--user", "enable", "--now", TIMER_NAME],
    )
    .map_err(|error| install_reports_error(unit_dir, "enabling the timer", error))?;
    Ok(status_at(unit_dir, runner))
}

/// Disable and remove both units. Safe when nothing is installed and when
/// the user manager has already disappeared; only this pair is touched.
pub fn remove_at(unit_dir: &Path, runner: &dyn CommandRunner) -> Result<TriggerStatus, String> {
    let service_path = unit_dir.join(SERVICE_NAME);
    let timer_path = unit_dir.join(TIMER_NAME);
    let installed = service_path.exists() || timer_path.exists();
    if installed {
        let _ = runner.run(
            "systemctl",
            &argv(&["--user", "disable", "--now", TIMER_NAME]),
        );
        let _ = runner.run("systemctl", &argv(&["--user", "daemon-reload"]));
        for path in [&timer_path, &service_path] {
            if path.exists() {
                fs::remove_file(path)
                    .map_err(|error| format!("could not remove `{}`: {error}", path.display()))?;
            }
        }
    }
    Ok(status_at(unit_dir, runner))
}

fn unavailable_status(detail: String) -> TriggerStatus {
    TriggerStatus {
        available: false,
        installed: false,
        enabled: false,
        active: false,
        service_path: String::new(),
        timer_path: String::new(),
        exec_start: None,
        detail: Some(detail),
    }
}

/// Default-path, default-runner variants used by the desktop commands.
pub fn status() -> TriggerStatus {
    match systemd_user_unit_dir() {
        Ok(unit_dir) => status_at(&unit_dir, &RealRunner),
        Err(detail) => unavailable_status(detail),
    }
}

pub fn install(config: &TriggerConfig) -> Result<TriggerStatus, String> {
    install_at(&systemd_user_unit_dir()?, config, &RealRunner)
}

pub fn remove() -> Result<TriggerStatus, String> {
    remove_at(&systemd_user_unit_dir()?, &RealRunner)
}
