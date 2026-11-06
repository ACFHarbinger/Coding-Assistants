use std::path::PathBuf;

pub(super) fn run(
    home: Option<PathBuf>,
    workspace: Option<PathBuf>,
    session: Option<String>,
    set_as_default_workspace_settings: bool,
    set_as_default_session_settings: bool,
) -> anyhow::Result<()> {
    tui::run(tui::TuiOptions {
        home,
        workspace,
        session,
        set_as_default_workspace_settings,
        set_as_default_session_settings,
    })
}
