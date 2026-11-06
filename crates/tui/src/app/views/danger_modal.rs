//! Danger-zone confirmation modal rendering (T5 / #139).
//!
//! Enforces the cancel-first confirmation contract and typed target matching.

use crate::app::danger_ops::DangerButton;
use crate::app::state::AppState;
use crate::app::ui::centered_rect;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

pub fn draw_danger_modal(frame: &mut Frame, area: Rect, app: &AppState) {
    if !app.danger.is_open {
        return;
    }
    let popup_area = centered_rect(75, 60, area);
    frame.render_widget(Clear, popup_area);
    let theme = &app.theme;

    let action = match &app.danger.action {
        Some(a) => a,
        None => return,
    };

    let is_red = action.is_red();
    let border_color = if is_red { theme.error } else { theme.accent2 };

    let target_label = if app.danger.target_name.is_empty() {
        app.workspace_path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "current workspace".to_string())
    } else {
        app.danger.target_name.clone()
    };

    let matched = app.danger.is_matched();

    let mut lines = vec![
        Line::from(Span::styled(
            if is_red {
                "⚠️  CAUTION: DESTRUCTIVE IRREVERSIBLE OPERATION"
            } else {
                "⚠️  CAUTION: RECOVERABLE CONFIGURATION RESET"
            },
            Style::default()
                .fg(border_color)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            action.description(&target_label),
            Style::default().fg(theme.fg),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "Required Confirmation Target: ",
                Style::default().fg(theme.muted),
            ),
            Span::styled(
                format!("\"{}\"", app.danger.target_name),
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Type Target Name: ", Style::default().fg(border_color)),
            Span::styled(
                if app.danger.input_text.is_empty() {
                    " (type exact name to unlock confirm)"
                } else {
                    &app.danger.input_text
                },
                if app.danger.input_text.is_empty() {
                    Style::default().fg(theme.muted)
                } else {
                    Style::default().fg(theme.fg).add_modifier(Modifier::BOLD)
                },
            ),
            Span::styled("█", Style::default().fg(theme.accent)),
        ]),
    ];

    if let Some(err) = &app.danger.error_message {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("Error: {err}"),
            Style::default()
                .fg(theme.error)
                .add_modifier(Modifier::BOLD),
        )));
    }

    // Cancel-first button bar
    let cancel_focused = app.danger.focused_button == DangerButton::Cancel;
    let confirm_focused = app.danger.focused_button == DangerButton::Confirm;

    let cancel_style = if cancel_focused {
        Style::default()
            .fg(Color::Black)
            .bg(theme.accent)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD)
    };

    let confirm_style = if !matched {
        Style::default().fg(theme.muted)
    } else if confirm_focused {
        Style::default()
            .fg(Color::White)
            .bg(border_color)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(border_color)
            .add_modifier(Modifier::BOLD)
    };

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(format!("  [ {} ]  ", action.cancel_label()), cancel_style),
        Span::raw("    "),
        Span::styled(format!("  [ {} ]  ", action.confirm_label()), confirm_style),
    ]));

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Navigation: [Tab] Switch Cancel/Confirm │ [Enter] Execute │ [Esc] Cancel (No Data Changed)",
        Style::default().fg(theme.muted),
    )));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(format!(" {} ", action.title()))
        .style(Style::default().bg(Color::Reset));
    frame.render_widget(
        Paragraph::new(lines).block(block).wrap(Wrap { trim: true }),
        popup_area,
    );
}
