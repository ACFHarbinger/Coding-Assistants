//! Non-blocking conflict and stale-write status banner (T7).

use crate::app::state::AppState;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn draw_conflict_banner(frame: &mut Frame, area: Rect, app: &AppState) {
    let theme = &app.theme;
    let conflict = &app.conflict;

    if !conflict.is_conflict_active {
        return;
    }

    let line = Line::from(vec![
        Span::styled(
            " ⚠ Stale Write Rejected: ",
            Style::default()
                .fg(theme.error)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(&conflict.conflict_message, Style::default().fg(theme.fg)),
        Span::raw("   "),
        Span::styled(
            " [r] Refresh & Retry ",
            Style::default()
                .fg(theme.success)
                .add_modifier(Modifier::BOLD | Modifier::REVERSED),
        ),
        Span::raw(" "),
        Span::styled(
            " [Esc] Dismiss ",
            Style::default()
                .fg(theme.muted)
                .add_modifier(Modifier::UNDERLINED),
        ),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.error));

    let p = Paragraph::new(line).block(block);
    frame.render_widget(p, area);
}
