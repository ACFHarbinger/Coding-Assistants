//! Linux systemd **user** trigger for headless Scheduled Goals ticks (S5a / #356).
//!
//! The trigger is deliberately narrow: it installs a user-scoped
//! `.service`/`.timer` pair whose only job is to run `ca schedule tick` with
//! explicit argv. It never installs a privileged (system) unit, never
//! bypasses the P5 approval path, and is not a general-purpose action
//! scheduler. The headless tick still enforces only the in-hub credit
//! allowance; live provider-quota pauses remain an in-app (S6) capability.
//!
//! Rendering is pure and the lifecycle accepts an injected unit directory
//! plus [`CommandRunner`], so tests exercise generated units and the full
//! install/status/remove flow without touching the real user timer.

mod lifecycle;
mod runner;
mod units;

pub use lifecycle::{install, install_at, remove, remove_at, status, status_at};
pub use runner::{CommandOutput, CommandRunner, RealRunner};
pub use units::{
    render_service, render_timer, resolve_ca_binary, systemd_user_unit_dir, SERVICE_NAME,
    TIMER_NAME,
};

use std::path::PathBuf;

/// Inputs baked into the generated unit pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TriggerConfig {
    /// Absolute path to the `ca` CLI binary.
    pub ca_bin: PathBuf,
    /// Hub data directory (`CA_HOME` / `--home`).
    pub ca_home: PathBuf,
    /// Workspace the service starts in.
    pub workspace: PathBuf,
    /// Timer cadence, in minutes.
    pub interval_minutes: u32,
}

/// Current state of the user-scoped trigger, safe to surface in Settings.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct TriggerStatus {
    /// systemd user manager is present and reachable.
    pub available: bool,
    /// Both unit files exist on disk.
    pub installed: bool,
    /// `systemctl --user is-enabled ca-schedule.timer` succeeded.
    pub enabled: bool,
    /// `systemctl --user is-active ca-schedule.timer` succeeded.
    pub active: bool,
    pub service_path: String,
    pub timer_path: String,
    pub exec_start: Option<String>,
    pub detail: Option<String>,
}

#[cfg(test)]
mod tests;
