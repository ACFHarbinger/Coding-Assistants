use super::keymap::handle_key;
pub use super::keymap::is_ascii_terminal;
use super::state::AppState;
use super::ui::draw_ui;
use crate::model::HubReadModel;
use crate::options::TuiOptions;
use crate::terminal::{init_terminal, restore_terminal};
use anyhow::{bail, Result};
use crossterm::event::{self, Event};
use hub::HubStore;
use std::path::PathBuf;
use std::time::Duration;

/// Main entrypoint for running the `ca tui` application loop.
pub fn run(options: TuiOptions) -> Result<()> {
    // Validate invocation options
    if options.set_as_default_workspace_settings && options.workspace.is_none() {
        bail!(
            "--set-as-default-workspace-settings requires an explicit --workspace <path> selector"
        );
    }
    if options.set_as_default_session_settings && options.session.is_none() {
        bail!("--set-as-default-session-settings requires an explicit --session <id> selector");
    }

    let default_home_dir = if let Ok(ca_home) = std::env::var("CA_HOME") {
        PathBuf::from(ca_home)
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".coding-assistants")
    };
    let home_dir = options.home.clone().unwrap_or(default_home_dir);

    // Verify or open Hub store & Settings store
    let store = HubStore::open(&home_dir)?;
    persist_requested_defaults(&options, &home_dir, &store)?;
    let settings_store = hub::SettingsStore::open(&home_dir);
    let ws_str_opt = options.workspace.as_ref().map(|p| p.display().to_string());
    let effective = settings_store.effective(ws_str_opt.as_deref());

    let workspace_path = options
        .workspace
        .clone()
        .or_else(|| effective.default_workspace.as_ref().map(PathBuf::from))
        .or_else(|| std::env::current_dir().ok());

    let session_id = options
        .session
        .clone()
        .or_else(|| effective.default_session.clone())
        .or_else(|| Some("general".to_string()));

    let initial_read =
        HubReadModel::load(&home_dir, workspace_path.as_deref(), session_id.as_deref());
    let (read_model, initial_read_failed) = match initial_read {
        Ok(model) => (model, false),
        Err(_) => (HubReadModel::empty(effective.clone()), true),
    };

    let mut app = AppState::new(&options, home_dir, &effective, read_model);
    if initial_read_failed {
        app.status_message = String::from("Hub data is temporarily unavailable; press r to retry.");
    }
    let mut terminal = init_terminal()?;

    let loop_result = run_loop(&mut terminal, &mut app, &store);

    // Always restore terminal regardless of exit outcome
    restore_terminal(terminal)?;

    loop_result
}

/// Persist only the explicit `--set-as-default-…-settings` requests. Plain
/// `--workspace` and `--session` selectors remain invocation-only. Keeping
/// this outside the terminal loop makes the real persistence/audit path
/// testable without opening a terminal.
pub fn persist_requested_defaults(
    options: &TuiOptions,
    home_dir: &std::path::Path,
    hub_store: &HubStore,
) -> Result<()> {
    let mut settings_store = hub::SettingsStore::open(home_dir);
    let workspace = options
        .workspace
        .as_ref()
        .map(|path| path.display().to_string());
    let mut changed = false;

    if options.set_as_default_workspace_settings {
        let workspace = workspace.as_deref().ok_or_else(|| {
            anyhow::anyhow!("--set-as-default-workspace-settings requires --workspace")
        })?;
        settings_store
            .set_default_workspace(Some(workspace))
            .map_err(|error| anyhow::anyhow!(error))?;
        changed = true;
    }

    if options.set_as_default_session_settings {
        let session = options.session.as_deref().ok_or_else(|| {
            anyhow::anyhow!("--set-as-default-session-settings requires --session")
        })?;
        match workspace.as_deref() {
            Some(workspace) => settings_store
                .set_workspace_default_session(workspace, Some(session))
                .map_err(|error| anyhow::anyhow!(error))?,
            None => settings_store
                .set_default_session(Some(session))
                .map_err(|error| anyhow::anyhow!(error))?,
        }
        changed = true;
    }

    if !changed {
        return Ok(());
    }

    settings_store
        .save()
        .map_err(|error| anyhow::anyhow!(error))?;
    if options.set_as_default_workspace_settings {
        hub_store
            .record_settings_audit_event("general.default_workspace", "global", "set_default")
            .map_err(|error| anyhow::anyhow!(error))?;
    }
    if options.set_as_default_session_settings {
        let scope = workspace.as_deref().unwrap_or("global");
        let field = if workspace.is_some() {
            "workspace.default_session"
        } else {
            "general.default_session"
        };
        hub_store
            .record_settings_audit_event(field, scope, "set_default")
            .map_err(|error| anyhow::anyhow!(error))?;
    }
    Ok(())
}

fn run_loop(
    terminal: &mut crate::terminal::TuiTerminal,
    app: &mut AppState,
    store: &HubStore,
) -> Result<()> {
    while !app.should_quit {
        terminal.draw(|frame| draw_ui(frame, app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                handle_key(app, store, key);
            }
        }

        // Advances the idle splash's animated gradient sweep and spinner
        // glyph once per loop iteration (~every 100ms, bounded by the
        // event::poll timeout above), independent of whether an event fired.
        app.tick = app.tick.wrapping_add(1);
    }
    Ok(())
}
