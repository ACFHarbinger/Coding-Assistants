//! Pure unit rendering, path resolution, and `ca` binary lookup.
//!
//! systemd's unit syntax has two escaping layers this module cares about:
//! double-quoted arguments (backslash/quote escaping) and specifiers (`%`,
//! and `$` in `ExecStart`). Paths are always quoted so spaces and shell-like
//! characters stay data; `$$`/`%%` keep literal `$`/`%` after expansion.

use super::TriggerConfig;
use std::path::{Path, PathBuf};

pub const SERVICE_NAME: &str = "ca-schedule.service";
pub const TIMER_NAME: &str = "ca-schedule.timer";

/// `$XDG_CONFIG_HOME/systemd/user` when set, else `~/.config/systemd/user`.
pub fn systemd_user_unit_dir() -> Result<PathBuf, String> {
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        if !xdg.is_empty() {
            return Ok(PathBuf::from(xdg).join("systemd/user"));
        }
    }
    let home = std::env::var_os("HOME")
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            "HOME is not set, so ~/.config/systemd/user cannot be located".to_string()
        })?;
    Ok(PathBuf::from(home).join(".config/systemd/user"))
}

/// Resolve the `ca` CLI to an absolute executable path.
///
/// An explicit absolute path wins; otherwise `ca` is searched on `PATH`.
/// A relative path is rejected rather than resolved against the Settings
/// process's current directory, which would make the generated unit depend
/// on launch context.
pub fn resolve_ca_binary(explicit: Option<&str>) -> Result<PathBuf, String> {
    if let Some(raw) = explicit.map(str::trim).filter(|value| !value.is_empty()) {
        let path = PathBuf::from(raw);
        if !path.is_absolute() {
            return Err(format!(
                "ca binary path must be absolute: `{}`",
                path.display()
            ));
        }
        if !is_executable(&path) {
            return Err(format!("`{}` is not an executable file", path.display()));
        }
        return Ok(path);
    }
    let path_var = std::env::var_os("PATH").unwrap_or_default();
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join("ca");
        if is_executable(&candidate) {
            return Ok(candidate);
        }
    }
    Err(
        "could not find the `ca` CLI on PATH; install it or pass an absolute ca_bin path"
            .to_string(),
    )
}

fn is_executable(path: &Path) -> bool {
    let Ok(metadata) = std::fs::metadata(path) else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

fn quote_exec_arg(value: &str) -> String {
    let escaped = value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('%', "%%")
        .replace('$', "$$");
    format!("\"{escaped}\"")
}

fn quote_path(value: &str) -> String {
    let escaped = value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('%', "%%");
    format!("\"{escaped}\"")
}

fn quote_environment(key: &str, value: &str) -> String {
    quote_path(&format!("{key}={value}"))
}

/// Render `ca-schedule.service`. The `ExecStart` argv is explicit — no
/// shell, no user-controlled action string — and pins both `--home` and
/// `CA_HOME` so the headless tick opens the same Hub database as the app.
pub fn render_service(config: &TriggerConfig) -> String {
    let ca_home = config.ca_home.to_string_lossy();
    let workspace = config.workspace.to_string_lossy();
    let ca_bin = config.ca_bin.to_string_lossy();
    format!(
        "[Unit]\n\
         Description=Coding Assistants scheduled task tick\n\
         After=default.target\n\
         \n\
         [Service]\n\
         Type=oneshot\n\
         Environment={environment}\n\
         WorkingDirectory={working_directory}\n\
         ExecStart={ca_bin} --home {ca_home} schedule tick\n",
        environment = quote_environment("CA_HOME", &ca_home),
        working_directory = quote_path(&workspace),
        ca_bin = quote_exec_arg(&ca_bin),
        ca_home = quote_exec_arg(&ca_home),
    )
}

/// Render `ca-schedule.timer`. `OnBootSec` gives a first tick shortly after
/// login/boot; `OnUnitActiveSec` then repeats at the requested cadence.
pub fn render_timer(interval_minutes: u32) -> String {
    format!(
        "[Unit]\n\
         Description=Coding Assistants scheduled task tick timer\n\
         \n\
         [Timer]\n\
         OnBootSec=1min\n\
         OnUnitActiveSec={interval_minutes}min\n\
         AccuracySec=1min\n\
         Persistent=true\n\
         Unit={SERVICE_NAME}\n\
         \n\
         [Install]\n\
         WantedBy=timers.target\n"
    )
}
