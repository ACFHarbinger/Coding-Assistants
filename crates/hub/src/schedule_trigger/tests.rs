use super::*;
use std::cell::RefCell;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::Mutex;
use tempfile::tempdir;

static ENV_LOCK: Mutex<()> = Mutex::new(());

#[derive(Default)]
struct FakeRunner {
    available: bool,
    enabled: RefCell<bool>,
    active: RefCell<bool>,
    fail_enable: bool,
    calls: RefCell<Vec<String>>,
}

impl FakeRunner {
    fn available() -> Self {
        Self {
            available: true,
            ..Self::default()
        }
    }

    fn calls(&self) -> Vec<String> {
        self.calls.borrow().clone()
    }
}

impl CommandRunner for FakeRunner {
    fn run(&self, program: &str, args: &[String]) -> Result<CommandOutput, String> {
        self.calls
            .borrow_mut()
            .push(format!("{program} {}", args.join(" ")));
        let ok = |stdout: &str| CommandOutput {
            success: true,
            stdout: stdout.to_string(),
            stderr: String::new(),
        };
        let fail = |stderr: &str| CommandOutput {
            success: false,
            stdout: String::new(),
            stderr: stderr.to_string(),
        };
        if program != "systemctl" {
            return Ok(fail("unexpected program"));
        }
        match (
            args.first().map(String::as_str),
            args.get(1).map(String::as_str),
        ) {
            (Some("--user"), Some("show-environment")) => Ok(if self.available {
                ok("")
            } else {
                fail("Failed to connect to bus: No such file or directory")
            }),
            (Some("--user"), Some("is-enabled")) => Ok(if *self.enabled.borrow() {
                ok("enabled")
            } else {
                fail("disabled")
            }),
            (Some("--user"), Some("is-active")) => Ok(if *self.active.borrow() {
                ok("active")
            } else {
                fail("inactive")
            }),
            (Some("--user"), Some("daemon-reload")) => Ok(ok("")),
            (Some("--user"), Some("enable")) => {
                if self.fail_enable {
                    Ok(fail("Access denied"))
                } else {
                    *self.enabled.borrow_mut() = true;
                    *self.active.borrow_mut() = true;
                    Ok(ok(""))
                }
            }
            (Some("--user"), Some("disable")) => {
                *self.enabled.borrow_mut() = false;
                *self.active.borrow_mut() = false;
                Ok(ok(""))
            }
            _ => Ok(fail("unexpected systemctl invocation")),
        }
    }
}

struct Fixture {
    _home: tempfile::TempDir,
    unit_dir: PathBuf,
    config: TriggerConfig,
}

fn fixture() -> Fixture {
    let home = tempdir().unwrap();
    let ca_bin = home.path().join("ca");
    fs::write(&ca_bin, "#!/bin/sh\n").unwrap();
    fs::set_permissions(&ca_bin, fs::Permissions::from_mode(0o755)).unwrap();
    let unit_dir = home.path().join("systemd/user");
    let config = TriggerConfig {
        ca_bin,
        ca_home: home.path().join("hub home"),
        workspace: home.path().join("work space"),
        interval_minutes: 5,
    };
    Fixture {
        _home: home,
        unit_dir,
        config,
    }
}

#[test]
fn service_unit_quotes_paths_and_pins_home_and_workspace() {
    let config = TriggerConfig {
        ca_bin: PathBuf::from("/opt/My Apps/ca"),
        ca_home: PathBuf::from("/home/u/CA%$Home"),
        workspace: PathBuf::from("/tmp/work space"),
        interval_minutes: 5,
    };
    let unit = render_service(&config);
    assert!(
        unit.contains("ExecStart=\"/opt/My Apps/ca\" --home \"/home/u/CA%%$$Home\" schedule tick")
    );
    assert!(unit.contains("Environment=\"CA_HOME=/home/u/CA%%$Home\""));
    assert!(unit.contains("WorkingDirectory=\"/tmp/work space\""));
    assert!(unit.contains("Type=oneshot"));
    assert!(!unit.contains("sh -c"));
}

#[test]
fn service_unit_escapes_quotes_and_backslashes() {
    let config = TriggerConfig {
        ca_bin: PathBuf::from("/opt/a\"b\\c/ca"),
        ca_home: PathBuf::from("/tmp/h"),
        workspace: PathBuf::from("/tmp/w"),
        interval_minutes: 5,
    };
    let unit = render_service(&config);
    assert!(unit.contains("ExecStart=\"/opt/a\\\"b\\\\c/ca\""));
}

#[test]
fn timer_unit_uses_the_requested_cadence_and_service() {
    let unit = render_timer(15);
    assert!(unit.contains("OnUnitActiveSec=15min"));
    assert!(unit.contains("OnBootSec=1min"));
    assert!(unit.contains("Unit=ca-schedule.service"));
    assert!(unit.contains("WantedBy=timers.target"));
}

#[test]
fn unit_dir_honors_xdg_config_home() {
    let _guard = ENV_LOCK.lock().unwrap();
    let previous = std::env::var_os("XDG_CONFIG_HOME");
    std::env::set_var("XDG_CONFIG_HOME", "/tmp/xdg-test");
    let resolved = systemd_user_unit_dir().unwrap();
    match previous {
        Some(value) => std::env::set_var("XDG_CONFIG_HOME", value),
        None => std::env::remove_var("XDG_CONFIG_HOME"),
    }
    assert_eq!(resolved, PathBuf::from("/tmp/xdg-test/systemd/user"));
}

#[test]
fn resolve_ca_binary_accepts_an_absolute_executable() {
    let dir = tempdir().unwrap();
    let binary = dir.path().join("ca");
    fs::write(&binary, "").unwrap();
    fs::set_permissions(&binary, fs::Permissions::from_mode(0o755)).unwrap();
    let resolved = resolve_ca_binary(Some(binary.to_str().unwrap())).unwrap();
    assert_eq!(resolved, binary);
}

#[test]
fn resolve_ca_binary_rejects_a_relative_path() {
    let error = resolve_ca_binary(Some("bin/ca")).unwrap_err();
    assert!(error.contains("must be absolute"), "{error}");
}

#[test]
fn resolve_ca_binary_rejects_a_non_executable_file() {
    let dir = tempdir().unwrap();
    let binary = dir.path().join("ca");
    fs::write(&binary, "").unwrap();
    fs::set_permissions(&binary, fs::Permissions::from_mode(0o644)).unwrap();
    let error = resolve_ca_binary(Some(binary.to_str().unwrap())).unwrap_err();
    assert!(error.contains("not an executable"), "{error}");
}

#[test]
fn install_writes_both_units_and_enables_the_timer() {
    let fixture = fixture();
    let runner = FakeRunner::available();
    let status = install_at(&fixture.unit_dir, &fixture.config, &runner).unwrap();

    assert!(status.installed);
    assert!(status.enabled);
    assert!(status.active);
    assert!(status.available);
    let service = fs::read_to_string(fixture.unit_dir.join(SERVICE_NAME)).unwrap();
    assert!(service.contains("schedule tick"));
    assert_eq!(
        status.exec_start.as_deref(),
        service
            .lines()
            .find(|line| line.starts_with("ExecStart="))
            .map(|line| &line["ExecStart=".len()..])
    );
    let timer = fs::read_to_string(fixture.unit_dir.join(TIMER_NAME)).unwrap();
    assert!(timer.contains("OnUnitActiveSec=5min"));

    let calls = runner.calls();
    let reload = calls
        .iter()
        .position(|call| call == "systemctl --user daemon-reload")
        .unwrap();
    let enable = calls
        .iter()
        .position(|call| call == "systemctl --user enable --now ca-schedule.timer")
        .unwrap();
    assert!(reload < enable, "{calls:?}");
}

#[test]
fn repeated_install_is_idempotent() {
    let fixture = fixture();
    let runner = FakeRunner::available();
    install_at(&fixture.unit_dir, &fixture.config, &runner).unwrap();
    let first = fs::read_to_string(fixture.unit_dir.join(SERVICE_NAME)).unwrap();
    let second_status = install_at(&fixture.unit_dir, &fixture.config, &runner).unwrap();
    let second = fs::read_to_string(fixture.unit_dir.join(SERVICE_NAME)).unwrap();
    assert_eq!(first, second);
    assert!(second_status.installed);
    assert_eq!(fixture.unit_dir.read_dir().unwrap().count(), 2);
}

#[test]
fn install_fails_with_actionable_diagnostics_without_a_user_manager() {
    let fixture = fixture();
    let mut runner = FakeRunner::available();
    runner.available = false;
    let error = install_at(&fixture.unit_dir, &fixture.config, &runner).unwrap_err();
    assert!(
        error.contains("systemd user manager is unavailable"),
        "{error}"
    );
    assert!(error.contains("loginctl enable-linger"), "{error}");
    assert!(!fixture.unit_dir.join(SERVICE_NAME).exists());
}

#[test]
fn install_rejects_invalid_input_before_contacting_systemd() {
    let fixture = fixture();
    let runner = FakeRunner::available();
    let mut config = fixture.config.clone();
    config.interval_minutes = 0;
    let error = install_at(&fixture.unit_dir, &config, &runner).unwrap_err();
    assert!(error.contains("at least 1"), "{error}");
    assert!(runner.calls().is_empty());
}

#[test]
fn install_reports_a_failed_enable_but_keeps_recoverable_units() {
    let fixture = fixture();
    let mut runner = FakeRunner::available();
    runner.fail_enable = true;
    let error = install_at(&fixture.unit_dir, &fixture.config, &runner).unwrap_err();
    assert!(error.contains("enabling the timer failed"), "{error}");
    assert!(error.contains("Retry the install"), "{error}");
    assert!(fixture.unit_dir.join(SERVICE_NAME).exists());
    assert!(fixture.unit_dir.join(TIMER_NAME).exists());
}

#[test]
fn remove_disables_and_deletes_both_units() {
    let fixture = fixture();
    let runner = FakeRunner::available();
    install_at(&fixture.unit_dir, &fixture.config, &runner).unwrap();

    let status = remove_at(&fixture.unit_dir, &runner).unwrap();
    assert!(!status.installed);
    assert!(!status.enabled);
    assert!(!status.active);
    assert!(!fixture.unit_dir.join(SERVICE_NAME).exists());
    assert!(!fixture.unit_dir.join(TIMER_NAME).exists());
    assert!(runner
        .calls()
        .iter()
        .any(|call| call == "systemctl --user disable --now ca-schedule.timer"));
}

#[test]
fn remove_is_safe_when_nothing_is_installed() {
    let fixture = fixture();
    let runner = FakeRunner::available();
    let status = remove_at(&fixture.unit_dir, &runner).unwrap();
    assert!(!status.installed);
    assert!(!fixture.unit_dir.join(SERVICE_NAME).exists());
}

#[test]
fn status_reports_installed_enabled_and_active_state() {
    let fixture = fixture();
    let runner = FakeRunner::available();
    fs::create_dir_all(&fixture.unit_dir).unwrap();
    fs::write(
        fixture.unit_dir.join(SERVICE_NAME),
        render_service(&fixture.config),
    )
    .unwrap();
    fs::write(
        fixture.unit_dir.join(TIMER_NAME),
        render_timer(fixture.config.interval_minutes),
    )
    .unwrap();
    *runner.enabled.borrow_mut() = true;
    *runner.active.borrow_mut() = true;

    let status = status_at(&fixture.unit_dir, &runner);
    assert!(status.available);
    assert!(status.installed);
    assert!(status.enabled);
    assert!(status.active);
    assert!(status.exec_start.unwrap().contains("schedule tick"));
}

#[test]
fn status_is_honest_when_the_user_manager_is_missing() {
    let fixture = fixture();
    let mut runner = FakeRunner::available();
    runner.available = false;
    let status = status_at(&fixture.unit_dir, &runner);
    assert!(!status.available);
    assert!(!status.installed);
    assert!(status
        .detail
        .unwrap()
        .contains("systemd user manager is unavailable"));
}

#[test]
fn status_only_probes_enablement_for_present_units() {
    let fixture = fixture();
    let runner = FakeRunner::available();
    let _ = status_at(&fixture.unit_dir, &runner);
    assert!(!runner
        .calls()
        .iter()
        .any(|call| call.contains("is-enabled")));
}
