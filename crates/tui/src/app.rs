//! `ca tui` main application runner and Ratatui rendering engine.

use crate::model::HubReadModel;
use crate::options::TuiOptions;
use crate::terminal::{init_terminal, restore_terminal};
use anyhow::{bail, Result};
use crossterm::event::{self, Event, KeyCode, KeyModifiers, MouseButton, MouseEventKind};
use hub::HubStore;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Tabs, Wrap},
    Frame,
};
use std::path::PathBuf;
use std::time::Duration;

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
        Err(_) => (
            HubReadModel {
                work_sessions: vec![],
                team_members: vec![],
                channel_messages: vec![],
                tasks: vec![],
                audit_events: vec![],
                effective_settings: effective.clone(),
            },
            true,
        ),
    };

    let mut app = AppState::new(&options, home_dir, &effective, read_model);
    if initial_read_failed {
        app.status_message = String::from("Hub data is temporarily unavailable; press r to retry.");
    }
    let mut terminal = init_terminal()?;

    let loop_result = run_loop(&mut terminal, &mut app);

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

fn run_loop(terminal: &mut crate::terminal::TuiTerminal, app: &mut AppState) -> Result<()> {
    while !app.should_quit {
        terminal.draw(|frame| draw_ui(frame, app))?;

        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) => {
                    if app.is_command_palette_open {
                        match key.code {
                            KeyCode::Esc => {
                                app.is_command_palette_open = false;
                                app.command_input.clear();
                            }
                            KeyCode::Enter => {
                                app.execute_command();
                            }
                            KeyCode::Backspace => {
                                app.command_input.pop();
                            }
                            KeyCode::Char(c) => {
                                app.command_input.push(c);
                            }
                            _ => {}
                        }
                    } else if app.is_help_open {
                        match key.code {
                            KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') => {
                                app.is_help_open = false;
                            }
                            _ => {}
                        }
                    } else {
                        match (key.code, key.modifiers) {
                            (KeyCode::Char('q'), _)
                            | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                                app.should_quit = true;
                            }
                            (KeyCode::Char('/'), _)
                            | (KeyCode::Char('p'), KeyModifiers::CONTROL) => {
                                app.is_command_palette_open = true;
                                app.command_input.clear();
                            }
                            (KeyCode::Char('?'), _) | (KeyCode::F(1), _) => {
                                app.is_help_open = !app.is_help_open;
                            }
                            (KeyCode::Char('r'), _) => {
                                app.refresh();
                            }
                            (KeyCode::Tab, KeyModifiers::NONE)
                            | (KeyCode::Char('l'), KeyModifiers::NONE)
                            | (KeyCode::Right, KeyModifiers::NONE) => {
                                app.active_tab = app.active_tab.next();
                                app.scroll_offset = 0;
                            }
                            (KeyCode::BackTab, _)
                            | (KeyCode::Tab, KeyModifiers::SHIFT)
                            | (KeyCode::Char('h'), KeyModifiers::NONE)
                            | (KeyCode::Left, KeyModifiers::NONE) => {
                                app.active_tab = app.active_tab.prev();
                                app.scroll_offset = 0;
                            }
                            (KeyCode::Char('j'), KeyModifiers::NONE)
                            | (KeyCode::Down, KeyModifiers::NONE) => {
                                app.scroll_offset = app.scroll_offset.saturating_add(1);
                                app.selected_index = app.selected_index.saturating_add(1);
                            }
                            (KeyCode::Char('k'), KeyModifiers::NONE)
                            | (KeyCode::Up, KeyModifiers::NONE) => {
                                app.scroll_offset = app.scroll_offset.saturating_sub(1);
                                app.selected_index = app.selected_index.saturating_sub(1);
                            }
                            (KeyCode::Char('g'), KeyModifiers::NONE)
                            | (KeyCode::Home, KeyModifiers::NONE) => {
                                app.scroll_offset = 0;
                                app.selected_index = 0;
                            }
                            (KeyCode::Char('G'), KeyModifiers::NONE)
                            | (KeyCode::End, KeyModifiers::NONE) => {
                                app.scroll_offset = 100;
                            }
                            (KeyCode::Char('1'), _) => {
                                app.active_tab = TabIndex::Orchestrate;
                                app.scroll_offset = 0;
                            }
                            (KeyCode::Char('2'), _) => {
                                app.active_tab = TabIndex::ChatAndMemory;
                                app.scroll_offset = 0;
                            }
                            (KeyCode::Char('3'), _) => {
                                app.active_tab = TabIndex::SharedHub;
                                app.scroll_offset = 0;
                            }
                            (KeyCode::Char('4'), _) => {
                                app.active_tab = TabIndex::Settings;
                                app.scroll_offset = 0;
                            }
                            (KeyCode::Esc, _) => {
                                app.should_quit = true;
                            }
                            _ => {}
                        }
                    }
                }
                Event::Mouse(mouse_event) => match mouse_event.kind {
                    MouseEventKind::Down(MouseButton::Left) => {
                        if mouse_event.row == 3 || mouse_event.row == 4 {
                            if mouse_event.column < 16 {
                                app.active_tab = TabIndex::Orchestrate;
                            } else if mouse_event.column < 34 {
                                app.active_tab = TabIndex::ChatAndMemory;
                            } else if mouse_event.column < 50 {
                                app.active_tab = TabIndex::SharedHub;
                            } else {
                                app.active_tab = TabIndex::Settings;
                            }
                            app.scroll_offset = 0;
                        }
                    }
                    MouseEventKind::ScrollDown => {
                        app.scroll_offset = app.scroll_offset.saturating_add(1);
                    }
                    MouseEventKind::ScrollUp => {
                        app.scroll_offset = app.scroll_offset.saturating_sub(1);
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }
    Ok(())
}

fn draw_ui(frame: &mut Frame, app: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(frame.area());

    draw_header(frame, chunks[0]);
    draw_tabs(frame, chunks[1], app);
    draw_body(frame, chunks[2], app);
    draw_footer(frame, chunks[3], app);

    if app.is_help_open {
        draw_help_modal(frame, frame.area());
    }

    if app.is_command_palette_open {
        draw_command_palette_modal(frame, frame.area(), app);
    }
}

fn draw_header(frame: &mut Frame, area: Rect) {
    let title = Line::from(vec![
        Span::styled(
            "⚡ Coding-Assistants ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("Ratatui TUI Client ", Style::default().fg(Color::Yellow)),
        Span::styled("(ca tui v0.1.0)", Style::default().fg(Color::DarkGray)),
    ]);
    let block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Reset));
    let paragraph = Paragraph::new(title).block(block);
    frame.render_widget(paragraph, area);
}

fn draw_tabs(frame: &mut Frame, area: Rect, app: &AppState) {
    let titles = vec![
        "1: Orchestrate",
        "2: Chat & Memory",
        "3: Shared Hub",
        "4: Settings",
    ];
    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title(" Views "))
        .select(app.active_tab as usize)
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_widget(tabs, area);
}

fn draw_body(frame: &mut Frame, area: Rect, app: &AppState) {
    match app.active_tab {
        TabIndex::Orchestrate => draw_orchestrate_view(frame, area, app),
        TabIndex::ChatAndMemory => draw_chat_view(frame, area, app),
        TabIndex::SharedHub => draw_shared_hub_view(frame, area, app),
        TabIndex::Settings => draw_settings_view(frame, area, app),
    }
}

fn draw_orchestrate_view(frame: &mut Frame, area: Rect, app: &AppState) {
    let ws = app
        .workspace_path
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "None".to_string());
    let sess = app.session_id.as_deref().unwrap_or("None");

    let team_roster = if app.read_model.team_members.is_empty() {
        "• Default roster: human, claude, grok, gemini, chat".to_string()
    } else {
        let members: Vec<String> = app
            .read_model
            .team_members
            .iter()
            .map(|agent| format!("{} ({})", agent.display_name, agent.id))
            .collect();
        format!("• Enrolled roster: {}", members.join(", "))
    };

    let session_count = app.read_model.work_sessions.len();
    let sessions_summary = if session_count == 0 {
        "• Work Sessions: None active".to_string()
    } else {
        let session_names: Vec<String> = app
            .read_model
            .work_sessions
            .iter()
            .take(5)
            .map(|s| format!("{} ({} members)", s.name, s.member_ids.len()))
            .collect();
        format!(
            "• Work Sessions ({}): {}",
            session_count,
            session_names.join(" | ")
        )
    };

    let text = vec![
        Line::from(vec![
            Span::styled(
                "Workspace Root: ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(ws),
            if app.is_workspace_overridden {
                Span::styled(
                    " [Invocation Override]",
                    Style::default().fg(Color::Magenta),
                )
            } else {
                Span::styled(" [Default]", Style::default().fg(Color::Green))
            },
        ]),
        Line::from(vec![
            Span::styled(
                "Active Session: ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(sess),
            if app.is_session_overridden {
                Span::styled(
                    " [Invocation Override]",
                    Style::default().fg(Color::Magenta),
                )
            } else {
                Span::styled(" [Default]", Style::default().fg(Color::Green))
            },
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Team Roster & Orchestration Controls:",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(team_roster),
        Line::from(sessions_summary),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Orchestrate Panel ");
    let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: true });
    frame.render_widget(paragraph, area);
}

fn draw_chat_view(frame: &mut Frame, area: Rect, app: &AppState) {
    let sess = app.session_id.as_deref().unwrap_or("general");

    let mut text = vec![
        Line::from(vec![
            Span::styled(
                "Active Channel/Session: ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!("#{}", sess), Style::default().fg(Color::Green)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Message Stream:",
            Style::default().fg(Color::Yellow),
        )),
    ];

    if app.read_model.channel_messages.is_empty() {
        text.push(Line::from(" [System] No messages in this channel yet. Send a message via CLI or Desktop to start."));
    } else {
        for msg in app
            .read_model
            .channel_messages
            .iter()
            .skip(app.scroll_offset)
            .take(15)
        {
            let sender = if msg.from_agent.is_empty() {
                "system"
            } else {
                &msg.from_agent
            };
            let body_preview: String = msg.body.chars().take(80).collect();
            text.push(Line::from(vec![
                Span::styled(format!(" [{}] ", sender), Style::default().fg(Color::Cyan)),
                Span::raw(body_preview),
            ]));
        }
    }

    let block = Block::default().borders(Borders::ALL).title(format!(
        " Chat & Memory Panel (Scroll: {}) ",
        app.scroll_offset
    ));
    let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: true });
    frame.render_widget(paragraph, area);
}

fn draw_shared_hub_view(frame: &mut Frame, area: Rect, app: &AppState) {
    let home = app.home_dir.display().to_string();
    let task_count = app.read_model.tasks.len();
    let audit_count = app.read_model.audit_events.len();

    let mut text = vec![
        Line::from(vec![
            Span::styled(
                "Hub Data Location: ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(home),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Active Tasks & Audit Stream:",
            Style::default().fg(Color::Yellow),
        )),
        Line::from(format!("• Durable Hub tasks: {} tasks", task_count)),
        Line::from(format!("• Settings audit events: {} recorded", audit_count)),
    ];

    if !app.read_model.tasks.is_empty() {
        text.push(Line::from(""));
        text.push(Line::from(Span::styled(
            "Recent Tasks:",
            Style::default().fg(Color::Cyan),
        )));
        for task in app.read_model.tasks.iter().skip(app.scroll_offset).take(5) {
            text.push(Line::from(format!("  [{:?}] {}", task.status, task.id)));
        }
    }

    if !app.read_model.audit_events.is_empty() {
        text.push(Line::from(""));
        text.push(Line::from(Span::styled(
            "Recent Settings Audit Events:",
            Style::default().fg(Color::Cyan),
        )));
        for event in app
            .read_model
            .audit_events
            .iter()
            .skip(app.scroll_offset)
            .take(5)
        {
            text.push(Line::from(format!(
                "  [{}] {} ({})",
                event.operation, event.path, event.status
            )));
        }
    }

    let block = Block::default().borders(Borders::ALL).title(format!(
        " Shared Hub Panel (Scroll: {}) ",
        app.scroll_offset
    ));
    let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: true });
    frame.render_widget(paragraph, area);
}

fn draw_settings_view(frame: &mut Frame, area: Rect, app: &AppState) {
    let eff = &app.read_model.effective_settings;

    let text = vec![
        Line::from(Span::styled(
            "Persistent Settings (toml) Configuration:",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(format!(
            "• Backup Retention: {} backups",
            eff.backup_retention
        )),
        Line::from(format!(
            "• Default Workspace: {}",
            eff.default_workspace.as_deref().unwrap_or("None (Global)")
        )),
        Line::from(format!(
            "• Default Session: {}",
            eff.default_session.as_deref().unwrap_or("None (Global)")
        )),
        Line::from(vec![
            Span::raw("• Workspace override mode: "),
            Span::styled(
                if app.is_default_workspace_persisted {
                    "Persisted Default"
                } else if app.is_workspace_overridden {
                    "Invocation Override"
                } else {
                    "Global Default"
                },
                Style::default().fg(Color::Yellow),
            ),
        ]),
        Line::from(vec![
            Span::raw("• Session override mode: "),
            Span::styled(
                if app.is_default_session_persisted {
                    "Persisted Default"
                } else if app.is_session_overridden {
                    "Invocation Override"
                } else {
                    "Global Default"
                },
                Style::default().fg(Color::Yellow),
            ),
        ]),
        Line::from(format!(
            "• Profiles configured: {} global profiles",
            eff.profiles.len()
        )),
        Line::from(format!(
            "• Harnesses configured: {} harnesses",
            eff.harnesses.len()
        )),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Settings Panel ");
    let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: true });
    frame.render_widget(paragraph, area);
}

fn draw_footer(frame: &mut Frame, area: Rect, app: &AppState) {
    let status = Line::from(vec![
        Span::styled("Nav: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            "[Tab/h/l] Tabs | [j/k] Scroll | [/] Palette | [?] Help | [r] Refresh | [q] Quit │ ",
            Style::default().fg(Color::White),
        ),
        Span::styled(&app.status_message, Style::default().fg(Color::Green)),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Controls & Status ");
    let paragraph = Paragraph::new(status).block(block);
    frame.render_widget(paragraph, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn draw_help_modal(frame: &mut Frame, area: Rect) {
    let popup_area = centered_rect(65, 55, area);
    frame.render_widget(Clear, popup_area);

    let help_text = vec![
        Line::from(Span::styled(
            "⚡ Navigation & Keybindings Cheat-Sheet",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("  Tab / l / Right    : Switch to Next Tab"),
        Line::from("  Shift+Tab / h / Left: Switch to Previous Tab"),
        Line::from(
            "  1 .. 4             : Direct Jump to Tab (1:Orchestrate, 2:Chat, 3:Hub, 4:Settings)",
        ),
        Line::from("  j / Down           : Scroll Down"),
        Line::from("  k / Up             : Scroll Up"),
        Line::from("  g / Home           : Scroll to Top"),
        Line::from("  G / End            : Scroll to Bottom"),
        Line::from("  / or Ctrl+P        : Open Command Palette Overlay"),
        Line::from("  r                  : Refresh Hub Read Model"),
        Line::from("  ? or F1            : Toggle Help Modal"),
        Line::from("  q or Esc           : Close Modal / Exit Application"),
        Line::from("  Mouse Left Click   : Click Tab Header to select"),
        Line::from("  Mouse Scroll       : Scroll view content up/down"),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Help Modal (Press Esc or ? to Close) ")
        .style(Style::default().bg(Color::Reset).fg(Color::Yellow));
    let paragraph = Paragraph::new(help_text)
        .block(block)
        .wrap(Wrap { trim: true });
    frame.render_widget(paragraph, popup_area);
}

fn draw_command_palette_modal(frame: &mut Frame, area: Rect, app: &AppState) {
    let popup_area = centered_rect(70, 25, area);
    frame.render_widget(Clear, popup_area);

    let text = vec![
        Line::from(Span::styled(
            "Command Palette — type a command and press Enter:",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled(
                "> ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(&app.command_input),
            Span::styled("█", Style::default().fg(Color::Green)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Commands: 1:orchestrate | 2:chat | 3:hub | 4:settings | refresh | help | quit",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Command Palette (Press Esc to Cancel) ")
        .style(Style::default().bg(Color::Reset).fg(Color::Cyan));
    let paragraph = Paragraph::new(text).block(block);
    frame.render_widget(paragraph, popup_area);
}
