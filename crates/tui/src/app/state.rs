use super::approvals::{ApprovalsState, ChatViewMode};
use super::composer::ComposerState;
use super::danger_ops::DangerModalState;
use super::memory_ops::MemorySearchState;
use super::recovery_ops::RecoveryModalState;
use super::session_ops::{CreateSessionState, SessionSwitcherState};
use super::settings_ops::{SettingsSection, SettingsState};
use super::views::HubViewMode;
use crate::model::HubReadModel;
use crate::options::TuiOptions;
use crate::theme::{Theme, ThemeName};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabIndex {
    Orchestrate = 0,
    ChatAndMemory = 1,
    SharedHub = 2,
    Settings = 3,
    HarnessPanes = 4,
}

impl TabIndex {
    pub fn from_index(index: usize) -> Self {
        match index {
            0 => TabIndex::Orchestrate,
            1 => TabIndex::ChatAndMemory,
            2 => TabIndex::SharedHub,
            3 => TabIndex::Settings,
            4 => TabIndex::HarnessPanes,
            _ => TabIndex::Orchestrate,
        }
    }

    pub fn next(self) -> Self {
        match self {
            TabIndex::Orchestrate => TabIndex::ChatAndMemory,
            TabIndex::ChatAndMemory => TabIndex::SharedHub,
            TabIndex::SharedHub => TabIndex::Settings,
            TabIndex::Settings => TabIndex::HarnessPanes,
            TabIndex::HarnessPanes => TabIndex::Orchestrate,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            TabIndex::Orchestrate => TabIndex::HarnessPanes,
            TabIndex::ChatAndMemory => TabIndex::Orchestrate,
            TabIndex::SharedHub => TabIndex::ChatAndMemory,
            TabIndex::Settings => TabIndex::SharedHub,
            TabIndex::HarnessPanes => TabIndex::Settings,
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
    pub theme_name: ThemeName,
    pub theme: Theme,
    /// Incremented once per draw loop iteration (~every 100ms); drives the
    /// idle splash's animated gradient sweep and spinner glyph. Never
    /// persisted — purely a render-time animation clock.
    pub tick: u64,
    // T4 Orchestration & Collaboration State
    pub composer: ComposerState,
    pub session_switcher: SessionSwitcherState,
    pub create_session: CreateSessionState,
    pub approvals: ApprovalsState,
    // T5 Settings, Memory, Audit, Recovery & Budgets State
    pub settings: SettingsState,
    pub danger: DangerModalState,
    pub recovery: RecoveryModalState,
    pub memory: MemorySearchState,
    pub hub_view_mode: HubViewMode,
    // T6 & T7 Harness Panes & Multi-Instance Coherence State
    pub harnesses: super::pane_ops::HarnessWorkspaceState,
    pub is_pane_focused: bool,
    pub conflict: super::conflict_ops::ConflictState,
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

        let mut recovery = RecoveryModalState::default();
        recovery.check_load_status(&read_model.settings_load.status);
        let conflict = super::conflict_ops::ConflictState::new(&home_dir);

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
            theme_name: ThemeName::Grok,
            theme: Theme::from_name(ThemeName::Grok),
            tick: 0,
            composer: ComposerState::default(),
            session_switcher: SessionSwitcherState::default(),
            create_session: CreateSessionState::default(),
            approvals: ApprovalsState::default(),
            settings: SettingsState::default(),
            danger: DangerModalState::default(),
            recovery,
            memory: MemorySearchState::default(),
            hub_view_mode: HubViewMode::Tasks,
            harnesses: super::pane_ops::HarnessWorkspaceState::new(),
            is_pane_focused: false,
            conflict,
        }
    }

    /// Advances to the next color theme, wrapping around. Used by the `T`
    /// keybinding and the `theme` command palette entry.
    pub fn cycle_theme(&mut self) {
        self.theme_name = self.theme_name.next();
        self.theme = Theme::from_name(self.theme_name);
        self.status_message = format!("Theme: {}", self.theme_name.label());
    }

    pub fn open_composer(&mut self) {
        self.composer.open(&self.read_model.team_members);
    }

    pub fn open_session_switcher(&mut self) {
        self.session_switcher.open();
    }

    pub fn open_create_session(&mut self) {
        self.create_session.open(&self.read_model.team_members);
    }

    pub fn load_session(&mut self, session_id: String) {
        self.session_id = Some(session_id.clone());
        self.session_switcher.close();
        self.refresh();
        let name = self
            .read_model
            .work_sessions
            .iter()
            .find(|s| s.id == session_id)
            .map(|s| s.name.clone())
            .unwrap_or_else(|| session_id.clone());
        self.status_message = format!("Loaded work session '{name}' ({session_id}).");
    }

    pub fn refresh(&mut self) {
        self.conflict.update_stamp(&self.home_dir);
        self.harnesses.drain_all();
        match HubReadModel::load(
            &self.home_dir,
            self.workspace_path.as_deref(),
            self.session_id.as_deref(),
        ) {
            Ok(model) => {
                self.recovery.check_load_status(&model.settings_load.status);
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
            "5" | "harness" | "panes" | "term" | "terminal" => {
                self.active_tab = TabIndex::HarnessPanes;
                self.status_message = String::from("Navigated to Harness Workspace Panes.");
            }
            "launch" => {
                self.active_tab = TabIndex::HarnessPanes;
                self.harnesses.is_launcher_open = true;
                self.status_message = String::from("Opened Harness Launcher dialog.");
            }
            "split" => {
                self.harnesses.toggle_split();
                let mode = if self.harnesses.is_split_view {
                    "split tiles"
                } else {
                    "single tab"
                };
                self.status_message = format!("Harness layout: {mode}.");
            }
            "danger" => {
                self.active_tab = TabIndex::Settings;
                self.settings.active_section = SettingsSection::DangerZone;
                self.status_message = String::from("Viewing Settings Danger Zone.");
            }
            "profiles" => {
                self.active_tab = TabIndex::Settings;
                self.settings.active_section = SettingsSection::Profiles;
                self.status_message = String::from("Viewing Provider Profiles.");
            }
            "memory" | "search memory" | "memories" => {
                self.active_tab = TabIndex::ChatAndMemory;
                self.approvals.chat_view_mode = ChatViewMode::MemorySearch;
                self.status_message = String::from("Viewing Memory Search.");
            }
            "audit" | "audit journal" | "journal" => {
                self.active_tab = TabIndex::SharedHub;
                self.hub_view_mode = HubViewMode::AuditJournal;
                self.status_message =
                    String::from("Viewing Audit Journal with Hash Chain Verification.");
            }
            "budgets" | "budget" | "telemetry" => {
                self.active_tab = TabIndex::SharedHub;
                self.hub_view_mode = HubViewMode::Budgets;
                self.status_message = String::from("Viewing Provider & Local Budgets.");
            }
            "tasks" => {
                self.active_tab = TabIndex::SharedHub;
                self.hub_view_mode = HubViewMode::Tasks;
                self.status_message = String::from("Viewing Shared Hub Tasks.");
            }
            "restore" | "recovery" => {
                if !self.read_model.backups.is_empty() {
                    self.recovery.is_open = true;
                    self.status_message = String::from("Opened Settings Backup Recovery dialog.");
                } else {
                    self.status_message = String::from("No settings backups available to restore.");
                }
            }
            "c" | "compose" => {
                self.open_composer();
            }
            "session" | "sessions" | "session switch" | "switch session" => {
                self.open_session_switcher();
            }
            "session new" | "new session" => {
                self.open_create_session();
            }
            "inbox" => {
                self.active_tab = TabIndex::ChatAndMemory;
                self.approvals.chat_view_mode = ChatViewMode::HumanInbox;
                self.status_message = String::from("Viewing Human Direct Inbox.");
            }
            "approvals" | "wakes" | "gates" => {
                self.active_tab = TabIndex::ChatAndMemory;
                self.approvals.chat_view_mode = ChatViewMode::PendingApprovals;
                self.status_message = String::from("Viewing Pending Wake Gate Approvals.");
            }
            "r" | "refresh" => {
                self.refresh();
            }
            "theme" => {
                self.cycle_theme();
            }
            other if other.starts_with("theme ") => {
                let requested = other["theme ".len()..].trim();
                match ThemeName::from_label(requested) {
                    Some(name) => {
                        self.theme_name = name;
                        self.theme = Theme::from_name(name);
                        self.status_message = format!("Theme: {}", name.label());
                    }
                    None => {
                        self.status_message = format!(
                            "Unknown theme '{requested}'. Try: grok, dracula, solarized, dark."
                        );
                    }
                }
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
