//! `ca tui` main application runner and Ratatui rendering engine.

use crate::options::TuiOptions;
use crate::terminal::{init_terminal, restore_terminal};
use anyhow::{bail, Result};
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use hub::HubStore;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Tabs, Wrap},
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
}

impl AppState {
    pub fn new(options: &TuiOptions, home_dir: PathBuf, effective: &hub::EffectiveSettings) -> Self {
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

        let mut status_message = String::from("Ready. Press Tab to switch tabs, q to exit.");
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
        }
    }
}

/// Main entrypoint for running the `ca tui` application loop.
pub fn run(options: TuiOptions) -> Result<()> {
    // Validate invocation options
    if options.set_as_default_workspace_settings && options.workspace.is_none() {
        bail!("--set-as-default-workspace-settings requires an explicit --workspace <path> selector");
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
    let mut settings_store = hub::SettingsStore::open(&home_dir);

    if options.set_as_default_workspace_settings {
        if let Some(ref ws) = options.workspace {
            let ws_str = ws.display().to_string();
            settings_store
                .set_default_workspace(Some(&ws_str))
                .map_err(|e| anyhow::anyhow!(e))?;
            settings_store.save().map_err(|e| anyhow::anyhow!(e))?;
            let _ = store.record_settings_audit_event(
                "general.default_workspace",
                "global",
                "set_default",
            );
        }
    }

    if options.set_as_default_session_settings {
        if let Some(ref sess) = options.session {
            if let Some(ref ws) = options.workspace {
                let ws_str = ws.display().to_string();
                settings_store
                    .set_workspace_default_session(&ws_str, Some(sess))
                    .map_err(|e| anyhow::anyhow!(e))?;
                settings_store.save().map_err(|e| anyhow::anyhow!(e))?;
                let _ = store.record_settings_audit_event(
                    "workspace.default_session",
                    &ws_str,
                    "set_default",
                );
            } else {
                settings_store
                    .set_default_session(Some(sess))
                    .map_err(|e| anyhow::anyhow!(e))?;
                settings_store.save().map_err(|e| anyhow::anyhow!(e))?;
                let _ = store.record_settings_audit_event(
                    "general.default_session",
                    "global",
                    "set_default",
                );
            }
        }
    }

    let ws_str_opt = options.workspace.as_ref().map(|p| p.display().to_string());
    let effective = settings_store.effective(ws_str_opt.as_deref());

    let mut app = AppState::new(&options, home_dir, &effective);
    let mut terminal = init_terminal()?;

    let loop_result = run_loop(&mut terminal, &mut app);

    // Always restore terminal regardless of exit outcome
    restore_terminal(terminal)?;

    loop_result
}

fn run_loop(terminal: &mut crate::terminal::TuiTerminal, app: &mut AppState) -> Result<()> {
    while !app.should_quit {
        terminal.draw(|frame| draw_ui(frame, app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match (key.code, key.modifiers) {
                    (KeyCode::Char('q'), _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                        app.should_quit = true;
                    }
                    (KeyCode::Tab, KeyModifiers::NONE) => {
                        app.active_tab = app.active_tab.next();
                    }
                    (KeyCode::BackTab, _) | (KeyCode::Tab, KeyModifiers::SHIFT) => {
                        app.active_tab = app.active_tab.prev();
                    }
                    (KeyCode::Char('1'), _) => app.active_tab = TabIndex::Orchestrate,
                    (KeyCode::Char('2'), _) => app.active_tab = TabIndex::ChatAndMemory,
                    (KeyCode::Char('3'), _) => app.active_tab = TabIndex::SharedHub,
                    (KeyCode::Char('4'), _) => app.active_tab = TabIndex::Settings,
                    (KeyCode::Esc, _) => {
                        app.should_quit = true;
                    }
                    _ => {}
                }
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
}

fn draw_header(frame: &mut Frame, area: Rect) {
    let title = Line::from(vec![
        Span::styled("⚡ Coding-Assistants ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled("Ratatui TUI Client ", Style::default().fg(Color::Yellow)),
        Span::styled("(ca tui v0.1.0)", Style::default().fg(Color::DarkGray)),
    ]);
    let block = Block::default().borders(Borders::ALL).style(Style::default().bg(Color::Reset));
    let paragraph = Paragraph::new(title).block(block);
    frame.render_widget(paragraph, area);
}

fn draw_tabs(frame: &mut Frame, area: Rect, app: &AppState) {
    let titles = vec!["1: Orchestrate", "2: Chat & Memory", "3: Shared Hub", "4: Settings"];
    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title(" Views "))
        .select(app.active_tab as usize)
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
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
    let ws = app.workspace_path.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "None".to_string());
    let sess = app.session_id.as_deref().unwrap_or("None");

    let text = vec![
        Line::from(vec![
            Span::styled("Workspace Root: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(ws),
            if app.is_workspace_overridden {
                Span::styled(" [Invocation Override]", Style::default().fg(Color::Magenta))
            } else {
                Span::styled(" [Default]", Style::default().fg(Color::Green))
            },
        ]),
        Line::from(vec![
            Span::styled("Active Session: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(sess),
            if app.is_session_overridden {
                Span::styled(" [Invocation Override]", Style::default().fg(Color::Magenta))
            } else {
                Span::styled(" [Default]", Style::default().fg(Color::Green))
            },
        ]),
        Line::from(""),
        Line::from(Span::styled("Team Roster & Orchestration Controls:", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Line::from("• Codex, Claude Code, Grok Build, Gemini/Antigravity harnesses bound."),
        Line::from("• Create Team Chat / Load Team Chat session controls integrated."),
    ];

    let block = Block::default().borders(Borders::ALL).title(" Orchestrate Panel ");
    let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: true });
    frame.render_widget(paragraph, area);
}

fn draw_chat_view(frame: &mut Frame, area: Rect, app: &AppState) {
    let sess = app.session_id.as_deref().unwrap_or("general");
    let text = vec![
        Line::from(vec![
            Span::styled("Active Channel/Session: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled(format!("#{}", sess), Style::default().fg(Color::Green)),
        ]),
        Line::from(""),
        Line::from(Span::styled("Message Stream:", Style::default().fg(Color::Yellow))),
        Line::from(" [System] Chat & Memory session initialized."),
        Line::from(" [Local Hub] Address all / subset / one agent routing ready."),
    ];

    let block = Block::default().borders(Borders::ALL).title(" Chat & Memory Panel ");
    let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: true });
    frame.render_widget(paragraph, area);
}

fn draw_shared_hub_view(frame: &mut Frame, area: Rect, app: &AppState) {
    let home = app.home_dir.display().to_string();
    let text = vec![
        Line::from(vec![
            Span::styled("Hub Data Location: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw(home),
        ]),
        Line::from(""),
        Line::from(Span::styled("Active Tasks & Audit Stream:", Style::default().fg(Color::Yellow))),
        Line::from("• Durable Hub tasks: 0 active"),
        Line::from("• Audit log stream: Normal"),
    ];

    let block = Block::default().borders(Borders::ALL).title(" Shared Hub Panel ");
    let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: true });
    frame.render_widget(paragraph, area);
}

fn draw_settings_view(frame: &mut Frame, area: Rect, app: &AppState) {
    let text = vec![
        Line::from(Span::styled("Persistent Settings (toml) Configuration:", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Line::from(vec![
            Span::raw("• Workspace override mode: "),
            Span::styled(
                if app.is_default_workspace_persisted { "Persisted Default" } else if app.is_workspace_overridden { "Invocation Override" } else { "Global Default" },
                Style::default().fg(Color::Yellow)
            ),
        ]),
        Line::from(vec![
            Span::raw("• Session override mode: "),
            Span::styled(
                if app.is_default_session_persisted { "Persisted Default" } else if app.is_session_overridden { "Invocation Override" } else { "Global Default" },
                Style::default().fg(Color::Yellow)
            ),
        ]),
        Line::from("• Policy controls: Standing approvals, auto-enrollment, tool/sandbox policy"),
    ];

    let block = Block::default().borders(Borders::ALL).title(" Settings Panel ");
    let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: true });
    frame.render_widget(paragraph, area);
}

fn draw_footer(frame: &mut Frame, area: Rect, app: &AppState) {
    let status = Line::from(vec![
        Span::styled("Nav: ", Style::default().fg(Color::DarkGray)),
        Span::styled("[Tab] Switch Tab | [1-4] Jump Tab | [q/Esc] Quit  │  ", Style::default().fg(Color::White)),
        Span::styled(&app.status_message, Style::default().fg(Color::Green)),
    ]);

    let block = Block::default().borders(Borders::ALL).title(" Controls & Status ");
    let paragraph = Paragraph::new(status).block(block);
    frame.render_widget(paragraph, area);
}
