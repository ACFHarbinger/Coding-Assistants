use crate::model::HubReadModel;
use crate::options::TuiOptions;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabIndex {
    Orchestrate = 0,
    ChatAndMemory = 1,
    SharedHub = 2,
    Settings = 3,
}

impl TabIndex {
    pub fn from_index(index: usize) -> Self {
        match index {
            0 => TabIndex::Orchestrate,
            1 => TabIndex::ChatAndMemory,
            2 => TabIndex::SharedHub,
            3 => TabIndex::Settings,
            _ => TabIndex::Orchestrate,
        }
    }

    pub fn next(self) -> Self {
        match self {
            TabIndex::Orchestrate => TabIndex::ChatAndMemory,
            TabIndex::ChatAndMemory => TabIndex::SharedHub,
            TabIndex::SharedHub => TabIndex::Settings,
            TabIndex::Settings => TabIndex::Orchestrate,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            TabIndex::Orchestrate => TabIndex::Settings,
            TabIndex::ChatAndMemory => TabIndex::Orchestrate,
            TabIndex::SharedHub => TabIndex::ChatAndMemory,
            TabIndex::Settings => TabIndex::SharedHub,
        }
    }
}

pub struct AppState {
    pub active_tab: TabIndex,
    pub home_dir: PathBuf,
    pub workspace_path: Option<PathBuf>,
    pub session_id: Option<String>,
    pub is_workspace_overridden: bool,
    pub is_session_overridden: bool,
    pub is_default_workspace_persisted: bool,
    pub is_default_session_persisted: bool,
    pub status_message: String,
    pub should_quit: bool,
    pub read_model: HubReadModel,
    pub is_help_open: bool,
    pub is_command_palette_open: bool,
    pub command_input: String,
    pub scroll_offset: usize,
    pub selected_index: usize,
    pub is_prefix_mode_active: bool,
}

impl AppState {
    pub fn new(
        options: &TuiOptions,
        home_dir: PathBuf,
        effective: &hub::EffectiveSettings,
        read_model: HubReadModel,
    ) -> Self {
        let is_workspace_overridden = options.workspace.is_some();
        let is_session_overridden = options.session.is_some();

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

        let mut status_message = String::from(
            "Ready. Press [Tab] to switch, [/] palette, [?] help, [r] refresh, [q] exit.",
        );
        if options.set_as_default_workspace_settings {
            status_message = format!("Persisted default workspace setting: {:?}", workspace_path);
        }
        if options.set_as_default_session_settings {
            status_message = format!("Persisted default session setting: {:?}", session_id);
        }

        Self {
            active_tab: TabIndex::Orchestrate,
            home_dir,
            workspace_path,
            session_id,
            is_workspace_overridden,
            is_session_overridden,
            is_default_workspace_persisted: options.set_as_default_workspace_settings,
            is_default_session_persisted: options.set_as_default_session_settings,
            status_message,
            should_quit: false,
            read_model,
            is_help_open: false,
            is_command_palette_open: false,
            command_input: String::new(),
            scroll_offset: 0,
            selected_index: 0,
            is_prefix_mode_active: false,
        }
    }

    pub fn refresh(&mut self) {
        match HubReadModel::load(
            &self.home_dir,
            self.workspace_path.as_deref(),
            self.session_id.as_deref(),
        ) {
            Ok(model) => {
                self.read_model = model;
                self.status_message = String::from("Refreshed Hub read model.");
                if self.read_model.effective_settings.tui.bell_notification {
                    use std::io::Write;
                    print!("\x07");
                    let _ = std::io::stdout().flush();
                }
            }
            Err(_) => {
                self.status_message =
                    String::from("Hub data is temporarily unavailable; press r to retry.");
            }
        }
    }

    pub fn execute_command(&mut self) {
        let input = self.command_input.trim().to_lowercase();
        self.command_input.clear();
        self.is_command_palette_open = false;

        match input.as_str() {
            "1" | "orchestrate" => {
                self.active_tab = TabIndex::Orchestrate;
                self.status_message = String::from("Navigated to Orchestrate panel.");
            }
            "2" | "chat" | "chat & memory" => {
                self.active_tab = TabIndex::ChatAndMemory;
                self.status_message = String::from("Navigated to Chat & Memory panel.");
            }
            "3" | "hub" | "shared hub" => {
                self.active_tab = TabIndex::SharedHub;
                self.status_message = String::from("Navigated to Shared Hub panel.");
            }
            "4" | "settings" => {
                self.active_tab = TabIndex::Settings;
                self.status_message = String::from("Navigated to Settings panel.");
            }
            "r" | "refresh" => {
                self.refresh();
            }
            "?" | "help" => {
                self.is_help_open = true;
            }
            "q" | "quit" | "exit" => {
                self.should_quit = true;
            }
            "" => {}
            other => {
                self.status_message = format!("Unknown command: '{other}'. Press [?] for help.");
            }
        }
    }
}
