use super::runner::is_ascii_terminal;
use super::state::{AppState, TabIndex};
use super::views::{
    draw_chat_view, draw_orchestrate_view, draw_settings_view, draw_shared_hub_view,
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Tabs, Wrap},
    Frame,
};

/// Carves a centered `percent_x` x `percent_y` rectangle out of `r`. Shared
/// by the help/command-palette modals and the empty-view idle splash.
pub(crate) fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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

pub fn draw_ui(frame: &mut Frame, app: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(frame.area());

    draw_header(frame, chunks[0], app);
    draw_tabs(frame, chunks[1], app);
    draw_body(frame, chunks[2], app);
    draw_footer(frame, chunks[3], app);

    if app.is_help_open {
        draw_help_modal(frame, frame.area(), app);
    }

    if app.is_command_palette_open {
        draw_command_palette_modal(frame, frame.area(), app);
    }
}

fn draw_header(frame: &mut Frame, area: Rect, app: &AppState) {
    let icon = if app.read_model.effective_settings.tui.unicode_fallback || is_ascii_terminal() {
        "[*] "
    } else {
        "⚡ "
    };
    let theme = &app.theme;
    let title = Line::from(vec![
        Span::styled(
            format!("{icon}Coding-Assistants "),
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("Ratatui TUI Client ", Style::default().fg(theme.accent2)),
        Span::styled("(ca tui v0.1.0) ", Style::default().fg(theme.muted)),
        Span::styled(
            format!("· theme: {} (press T to cycle)", app.theme_name.label()),
            Style::default().fg(theme.muted),
        ),
    ]);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
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
    let theme = &app.theme;
    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border))
                .title(" Views "),
        )
        .select(app.active_tab as usize)
        .style(Style::default().fg(theme.tab_inactive_fg))
        .highlight_style(
            Style::default()
                .fg(theme.accent)
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

fn draw_footer(frame: &mut Frame, area: Rect, app: &AppState) {
    let theme = &app.theme;
    let status = Line::from(vec![
        Span::styled("Nav: ", Style::default().fg(theme.muted)),
        Span::styled(
            "[Tab/h/l] Tabs | [Shift+Tab] Prev | [j/k] Scroll | [/] Palette | [T] Theme | [?] Help | [q] Quit │ ",
            Style::default().fg(theme.fg),
        ),
        Span::styled(&app.status_message, Style::default().fg(theme.success)),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .title(" Controls & Status ");
    let paragraph = Paragraph::new(status).block(block);
    frame.render_widget(paragraph, area);
}

fn draw_help_modal(frame: &mut Frame, area: Rect, app: &AppState) {
    let popup_area = centered_rect(65, 60, area);
    frame.render_widget(Clear, popup_area);
    let theme = &app.theme;

    let help_text = vec![
        Line::from(Span::styled(
            "⚡ Navigation & Keybindings Cheat-Sheet",
            Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("  Tab / l / Right      : Switch to Next Tab"),
        Line::from("  Shift+Tab / h / Left : Switch to Previous Tab"),
        Line::from(
            "  1 .. 4               : Direct Jump to Tab (1:Orchestrate, 2:Chat, 3:Hub, 4:Settings)",
        ),
        Line::from("  j / Down             : Scroll Down"),
        Line::from("  k / Up               : Scroll Up"),
        Line::from("  g / Home             : Scroll to Top"),
        Line::from("  G / End              : Scroll to Bottom"),
        Line::from("  / or Ctrl+P          : Open Command Palette Overlay"),
        Line::from("  T                    : Cycle Color Theme (Grok, Dracula, Solarized, Dark)"),
        Line::from("  r                    : Refresh Hub Read Model"),
        Line::from("  ? or F1              : Toggle Help Modal"),
        Line::from("  q or Esc             : Close Modal / Exit Application"),
        Line::from(""),
        Line::from(Span::styled(
            "The mouse is intentionally left uncaptured, so click-and-drag",
            Style::default().fg(theme.muted),
        )),
        Line::from(Span::styled(
            "text selection/copy works natively in your terminal emulator.",
            Style::default().fg(theme.muted),
        )),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_focus))
        .title(" Help Modal (Press Esc or ? to Close) ")
        .style(Style::default().bg(Color::Reset).fg(theme.accent2));
    let paragraph = Paragraph::new(help_text)
        .block(block)
        .wrap(Wrap { trim: true });
    frame.render_widget(paragraph, popup_area);
}

fn draw_command_palette_modal(frame: &mut Frame, area: Rect, app: &AppState) {
    let popup_area = centered_rect(70, 25, area);
    frame.render_widget(Clear, popup_area);
    let theme = &app.theme;

    let text = vec![
        Line::from(Span::styled(
            "Command Palette — type a command and press Enter:",
            Style::default().fg(theme.accent2).add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled(
                "> ",
                Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
            ),
            Span::raw(&app.command_input),
            Span::styled("█", Style::default().fg(theme.success)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Commands: 1:orchestrate | 2:chat | 3:hub | 4:settings | refresh | theme [name] | help | quit",
            Style::default().fg(theme.muted),
        )),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_focus))
        .title(" Command Palette (Press Esc to Cancel) ")
        .style(Style::default().bg(Color::Reset).fg(theme.accent));
    let paragraph = Paragraph::new(text).block(block);
    frame.render_widget(paragraph, popup_area);
}
