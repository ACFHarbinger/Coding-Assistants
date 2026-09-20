//! Malformed settings recovery modal rendering (T5 / #139).

use crate::app::state::AppState;
use crate::app::ui::centered_rect;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

pub fn draw_recovery_modal(frame: &mut Frame, area: Rect, app: &AppState) {
    if !app.recovery.is_open {
        return;
    }
    let popup_area = centered_rect(75, 60, area);
    frame.render_widget(Clear, popup_area);
    let theme = &app.theme;

    let mut lines = vec![
        Line::from(Span::styled(
            "⚠️  MALFORMED SETTINGS FILE DETECTED",
            Style::default()
                .fg(theme.error)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("Reason: {}", app.recovery.reason),
            Style::default().fg(theme.accent2),
        )),
        Line::from(
            "Running on safe defaults. You may restore a verified backup or quarantine this file.",
        ),
        Line::from(""),
        Line::from(Span::styled(
            "Available Last-Known-Good Backups:",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )),
    ];

    if app.read_model.backups.is_empty() {
        lines.push(Line::from("  • No settings backups found on disk."));
    } else {
        for (i, backup) in app.read_model.backups.iter().enumerate().take(6) {
            let is_sel = i == app.recovery.selected_backup_index;
            let marker = if is_sel { "▶ " } else { "  " };
            let fname = backup
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");
            let style = if is_sel {
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg)
            };
            lines.push(Line::from(vec![
                Span::styled(marker, style),
                Span::styled(fname, style),
            ]));
        }
    }

    if let Some(err) = &app.recovery.error_message {
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
        "Actions: [↑/↓] Select Backup │ [r] Restore Backup │ [q] Quarantine & Reset │ [Esc] Dismiss",
        Style::default().fg(theme.muted),
    )));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.error))
        .title(" Settings Diagnostic & Recovery ")
        .style(Style::default().bg(Color::Reset));
    frame.render_widget(
        Paragraph::new(lines).block(block).wrap(Wrap { trim: true }),
        popup_area,
    );
}
