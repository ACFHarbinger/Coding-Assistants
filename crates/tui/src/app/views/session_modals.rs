//! Work session modal dialogs for ca tui (T4 / #138).

use crate::app::state::AppState;
use crate::app::ui::centered_rect;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

pub fn draw_session_switcher_modal(frame: &mut Frame, area: Rect, app: &AppState) {
    let popup_area = centered_rect(65, 50, area);
    frame.render_widget(Clear, popup_area);
    let theme = &app.theme;

    let sessions = &app.read_model.work_sessions;
    let mut lines = vec![
        Line::from(Span::styled(
            "Select a Work Session to Load:",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    if sessions.is_empty() {
        lines.push(Line::from(
            "  • No work sessions found. Press [n] to create one.",
        ));
    } else {
        for (i, session) in sessions.iter().enumerate() {
            let is_sel = i == app.session_switcher.selected_index;
            let is_current = app.session_id.as_deref() == Some(&session.id);
            let marker = if is_sel { "▶ " } else { "  " };
            let current_tag = if is_current { " [Active]" } else { "" };
            let style = if is_sel {
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg)
            };
            lines.push(Line::from(vec![
                Span::styled(marker, style),
                Span::styled(
                    format!("{} ({} members)", session.name, session.member_ids.len()),
                    style,
                ),
                Span::styled(current_tag, Style::default().fg(theme.success)),
                Span::styled(
                    format!(" · id: {}", session.id),
                    Style::default().fg(theme.muted),
                ),
            ]));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "[↑/↓] Select Session │ [Enter] Load │ [n] New Session │ [Esc] Cancel",
        Style::default().fg(theme.muted),
    )));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_focus))
        .title(" Switch Work Session ")
        .style(Style::default().bg(Color::Reset));
    frame.render_widget(
        Paragraph::new(lines).block(block).wrap(Wrap { trim: true }),
        popup_area,
    );
}

pub fn draw_create_session_modal(frame: &mut Frame, area: Rect, app: &AppState) {
    let popup_area = centered_rect(70, 55, area);
    frame.render_widget(Clear, popup_area);
    let theme = &app.theme;

    let mut lines = vec![
        Line::from(Span::styled(
            "Create New Work Session:",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "Session Name: ",
                Style::default()
                    .fg(theme.accent2)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(&app.create_session.name_input),
            Span::styled("█", Style::default().fg(theme.accent)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Initial Member Roster:",
            Style::default()
                .fg(theme.accent2)
                .add_modifier(Modifier::BOLD),
        )),
    ];

    let member_keys: Vec<String> = app
        .create_session
        .selected_members
        .keys()
        .cloned()
        .collect();
    let mut member_spans = vec![Span::raw("  ")];
    for (i, key) in member_keys.iter().enumerate() {
        let selected = *app
            .create_session
            .selected_members
            .get(key)
            .unwrap_or(&true);
        let is_cursor = i == app.create_session.member_cursor;
        let box_str = if selected { "[x]" } else { "[ ]" };
        let style = if is_cursor {
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else if selected {
            Style::default().fg(theme.success)
        } else {
            Style::default().fg(theme.muted)
        };
        member_spans.push(Span::styled(format!("{box_str} {key}  "), style));
    }
    lines.push(Line::from(member_spans));

    if let Some(err) = &app.create_session.error_message {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("Error: {err}"),
            Style::default()
                .fg(theme.error)
                .add_modifier(Modifier::BOLD),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Navigation: [Tab] Switch Field │ [Space] Toggle Member │ [Enter] Create │ [Esc] Cancel",
        Style::default().fg(theme.muted),
    )));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_focus))
        .title(" New Work Session ")
        .style(Style::default().bg(Color::Reset));
    frame.render_widget(
        Paragraph::new(lines).block(block).wrap(Wrap { trim: true }),
        popup_area,
    );
}
