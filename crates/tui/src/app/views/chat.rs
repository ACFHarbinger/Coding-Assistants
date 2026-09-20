use crate::app::approvals::ChatViewMode;
use crate::app::state::AppState;
use crate::theme::{logo_lines, spinner_frame, wordmark_lines};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

pub fn draw_chat_view(frame: &mut Frame, area: Rect, app: &AppState) {
    match app.approvals.chat_view_mode {
        ChatViewMode::SessionMessages => draw_session_messages(frame, area, app),
        ChatViewMode::HumanInbox => draw_human_inbox(frame, area, app),
        ChatViewMode::PendingApprovals => draw_approvals_view(frame, area, app),
        ChatViewMode::MemorySearch => draw_memory_search(frame, area, app),
    }
}

fn draw_session_messages(frame: &mut Frame, area: Rect, app: &AppState) {
    if app.read_model.channel_messages.is_empty() {
        draw_idle_splash(frame, area, app);
        return;
    }

    let theme = &app.theme;
    let sess = app.session_id.as_deref().unwrap_or("general");

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(3)])
        .split(area);

    let mut text = vec![
        Line::from(vec![
            Span::styled(
                "Active Channel/Session: ",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!("#{sess}"), Style::default().fg(theme.success)),
            Span::styled(" │ Mode: ", Style::default().fg(theme.muted)),
            Span::styled(
                "[Messages]",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " [Inbox] [Gates] (Press [I] to cycle)",
                Style::default().fg(theme.muted),
            ),
        ]),
        Line::from(""),
    ];

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
            Span::styled(format!(" [{}] ", sender), Style::default().fg(theme.accent)),
            Span::raw(body_preview),
        ]));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .title(format!(
            " Chat & Memory Panel (Scroll: {}) ",
            app.scroll_offset
        ));
    let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: true });
    frame.render_widget(paragraph, chunks[0]);

    // Bottom action hint bar
    let action_bar = Line::from(vec![
        Span::styled(
            "Actions: ",
            Style::default()
                .fg(theme.accent2)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "[c] Open Composer",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " │ [s] Switch Session │ [n] New Session │ [I] Toggle Inbox/Gates",
            Style::default().fg(theme.fg),
        ),
    ]);
    let action_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border));
    frame.render_widget(Paragraph::new(action_bar).block(action_block), chunks[1]);
}

fn draw_human_inbox(frame: &mut Frame, area: Rect, app: &AppState) {
    let theme = &app.theme;
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(3)])
        .split(area);

    let inbox = &app.read_model.inbox_messages;
    let mut text = vec![
        Line::from(vec![
            Span::styled(
                "Human Direct Inbox: ",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("({} messages)", inbox.len()),
                Style::default().fg(theme.accent2),
            ),
            Span::styled(" │ Mode: [Messages] ", Style::default().fg(theme.muted)),
            Span::styled(
                "[Inbox]",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " [Gates] (Press [I] to cycle)",
                Style::default().fg(theme.muted),
            ),
        ]),
        Line::from(""),
    ];

    if inbox.is_empty() {
        text.push(Line::from("  • No direct messages addressed to human."));
    } else {
        for (i, msg) in inbox.iter().enumerate().take(12) {
            let is_sel = i == app.approvals.selected_inbox_index;
            let marker = if is_sel { "▶ " } else { "  " };
            let style = if is_sel {
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg)
            };
            let body_preview: String = msg.body.chars().take(70).collect();
            text.push(Line::from(vec![
                Span::styled(marker, style),
                Span::styled(format!("[{}] From {}: ", msg.status, msg.from_agent), style),
                Span::raw(body_preview),
            ]));
        }
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .title(" Direct Inbox ");
    frame.render_widget(
        Paragraph::new(text).block(block).wrap(Wrap { trim: true }),
        chunks[0],
    );

    let action_bar = Line::from(vec![
        Span::styled(
            "Actions: ",
            Style::default()
                .fg(theme.accent2)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "[a] Acknowledge/Mark Read │ [c] Open Composer │ [I] Cycle View",
            Style::default().fg(theme.fg),
        ),
    ]);
    let action_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border));
    frame.render_widget(Paragraph::new(action_bar).block(action_block), chunks[1]);
}

fn draw_approvals_view(frame: &mut Frame, area: Rect, app: &AppState) {
    let theme = &app.theme;
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(3)])
        .split(area);

    let gates = &app.read_model.pending_gates;
    let mut text = vec![
        Line::from(vec![
            Span::styled(
                "Pending Wake Gate Approvals (C12): ",
                Style::default()
                    .fg(theme.error)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("({} pending)", gates.len()),
                Style::default().fg(theme.accent2),
            ),
            Span::styled(
                " │ Mode: [Messages] [Inbox] ",
                Style::default().fg(theme.muted),
            ),
            Span::styled(
                "[Gates]",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" (Press [I] to cycle)", Style::default().fg(theme.muted)),
        ]),
        Line::from(""),
    ];

    if gates.is_empty() {
        text.push(Line::from("  • All wake gate requests have been resolved."));
    } else {
        for (i, gate) in gates.iter().enumerate().take(12) {
            let is_sel = i == app.approvals.selected_approval_index;
            let marker = if is_sel { "▶ " } else { "  " };
            let style = if is_sel {
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg)
            };
            let body_preview: String = gate.body.chars().take(60).collect();
            text.push(Line::from(vec![
                Span::styled(marker, style),
                Span::styled(
                    format!(
                        "[{}] Target: {} | Sender: {} | Body: \"{}\"",
                        gate.status,
                        gate.to_agents.join(", "),
                        gate.from_agent,
                        body_preview
                    ),
                    style,
                ),
            ]));
        }
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .title(" Wake Gate Approvals ");
    frame.render_widget(
        Paragraph::new(text).block(block).wrap(Wrap { trim: true }),
        chunks[0],
    );

    let action_bar = Line::from(vec![
        Span::styled(
            "Actions: ",
            Style::default()
                .fg(theme.accent2)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "[a] Approve Wake Gate",
            Style::default()
                .fg(theme.success)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" │ ", Style::default().fg(theme.muted)),
        Span::styled(
            "[d] Deny Wake Gate",
            Style::default()
                .fg(theme.error)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " │ [↑/↓] Select │ [I] Cycle View",
            Style::default().fg(theme.fg),
        ),
    ]);
    let action_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border));
    frame.render_widget(Paragraph::new(action_bar).block(action_block), chunks[1]);
}

fn draw_idle_splash(frame: &mut Frame, area: Rect, app: &AppState) {
    let theme = &app.theme;
    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .title(" Chat & Memory Panel ");
    frame.render_widget(&outer, area);
    let inner = outer.inner(area);

    let phase = (app.tick as f32) / 120.0;
    let mut lines = logo_lines(theme, phase);
    lines.extend(wordmark_lines(theme, "Ratatui TUI Client · v0.1.0"));
    lines.push(Line::from(""));
    let sess = app.session_id.as_deref().unwrap_or("general");
    lines.push(Line::from(vec![
        Span::styled(spinner_frame(app.tick), Style::default().fg(theme.accent)),
        Span::styled(
            format!(" waiting on #{sess} — nothing here yet"),
            Style::default().fg(theme.muted),
        ),
    ]));
    lines.push(Line::from(Span::styled(
        "Press [c] to Compose, [s] to Switch Session, [n] for New Session.",
        Style::default().fg(theme.muted),
    )));

    let logo_width = 24;
    let popup = centered_rect_in(logo_width, lines.len() as u16, inner);
    frame.render_widget(Clear, popup);
    frame.render_widget(Paragraph::new(lines), popup);
}

fn draw_memory_search(frame: &mut Frame, area: Rect, app: &AppState) {
    let theme = &app.theme;
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(3)])
        .split(area);

    let scope_filter = app.memory.scope_filter.as_deref().unwrap_or("All Scopes");

    let mut text = vec![
        Line::from(vec![
            Span::styled(
                "Memory Search: ",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                if app.memory.query.is_empty() {
                    "(empty query — showing recent memories)"
                } else {
                    &app.memory.query
                },
                Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
            ),
            Span::styled("█", Style::default().fg(theme.accent)),
            Span::styled(
                format!(" │ Scope Filter: [{scope_filter}] (press [w] to toggle)"),
                Style::default().fg(theme.muted),
            ),
        ]),
        Line::from(""),
    ];

    if app.memory.results.is_empty() {
        text.push(Line::from("  • No memory records matching query."));
    } else {
        for (i, mem) in app
            .memory
            .results
            .iter()
            .skip(app.scroll_offset)
            .take(8)
            .enumerate()
        {
            let actual_idx = app.scroll_offset + i;
            let is_sel = actual_idx == app.memory.selected_index;
            let marker = if is_sel { "▶ " } else { "  " };
            let style = if is_sel {
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg)
            };
            let body_preview: String = mem.body.chars().take(60).collect();
            let title = mem.title.as_deref().unwrap_or("(untitled)");
            text.push(Line::from(vec![
                Span::styled(marker, style),
                Span::styled(
                    format!("[{}] ", mem.tier),
                    Style::default().fg(theme.accent2),
                ),
                Span::styled(
                    format!("[{}] ", mem.scope),
                    Style::default().fg(theme.muted),
                ),
                Span::styled(format!("{title} "), style),
                Span::styled(
                    format!("\"{body_preview}...\""),
                    Style::default().fg(theme.muted),
                ),
            ]));
        }
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .title(" Shared Agentic Memory Review ");
    frame.render_widget(
        Paragraph::new(text).block(block).wrap(Wrap { trim: true }),
        chunks[0],
    );

    let action_bar = Line::from(vec![
        Span::styled(
            "Actions: ",
            Style::default()
                .fg(theme.accent2)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "[↑/↓] Select │ [/] Type Query │ [Enter] Search │ [w] Filter Scope │ [I] Cycle View",
            Style::default().fg(theme.fg),
        ),
    ]);
    let action_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border));
    frame.render_widget(Paragraph::new(action_bar).block(action_block), chunks[1]);
}

fn centered_rect_in(width: u16, height: u16, r: Rect) -> Rect {
    let width = width.min(r.width);
    let height = height.min(r.height);
    let x = r.x + (r.width.saturating_sub(width)) / 2;
    let y = r.y + (r.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width, height)
}
