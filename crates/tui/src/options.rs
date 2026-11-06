//! Invocation options and selector flags for `ca tui`.

use std::path::PathBuf;

/// Invocation options passed to the `ca tui` application runner.
#[derive(Debug, Clone, Default)]
pub struct TuiOptions {
    /// Hub data directory override (defaults to $CA_HOME or ~/.coding-assistants).
    pub home: Option<PathBuf>,
    /// Override the active workspace path for this invocation.
    pub workspace: Option<PathBuf>,
    /// Override the active session ID for this invocation.
    pub session: Option<String>,
    /// Persist the specified invocation workspace as the default workspace setting.
    pub set_as_default_workspace_settings: bool,
    /// Persist the specified invocation session as the default session setting.
    pub set_as_default_session_settings: bool,
}
