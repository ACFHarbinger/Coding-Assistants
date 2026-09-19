use crate::app::state::AppState;
use crate::theme::{lerp_accent, sparkline_string, spinner_frame, task_sparkline_buckets};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn draw_shared_hub_view(frame: &mut Frame, area: Rect, app: &AppState) {
    let theme = &app.theme;
    let home = app.home_dir.display().to_string();
    let task_count = app.read_model.tasks.len();
    let audit_count = app.read_model.audit_events.len();

    let mut text = vec![
        Line::from(vec![
            Span::styled(
                "Hub Data Location: ",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(home),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Active Tasks & Audit Stream:",
            Style::default().fg(theme.accent2),
        )),
        Line::from(format!("• Durable Hub tasks: {} tasks", task_count)),
        Line::from(format!("• Settings audit events: {} recorded", audit_count)),
    ];

    if !app.read_model.tasks.is_empty() {
        text.push(Line::from(""));
        text.push(Line::from(Span::styled(
            "Recent Tasks:",
            Style::default().fg(theme.accent),
        )));
        for task in app.read_model.tasks.iter().skip(app.scroll_offset).take(5) {
            text.push(Line::from(format!("  [{:?}] {}", task.status, task.id)));
        }
    }

    if !app.read_model.audit_events.is_empty() {
        text.push(Line::from(""));
        text.push(Line::from(Span::styled(
            "Recent Settings Audit Events:",
            Style::default().fg(theme.accent),
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

    // ── Ambient sparkline footer ──────────────────────────────────────────────
    let n_buckets = (area.width.saturating_sub(22) as usize).clamp(1, 40);
    let phase = (app.tick as f32 / 80.0).rem_euclid(1.0);

    let task_ts: Vec<&str> = app
        .read_model
        .tasks
        .iter()
        .map(|t| t.updated_at.as_str())
        .collect();
    let task_bar = sparkline_string(&task_sparkline_buckets(&task_ts, n_buckets));

    let audit_ts: Vec<&str> = app
        .read_model
        .audit_events
        .iter()
        .map(|e| e.observed_at.as_str())
        .collect();
    let audit_bar = sparkline_string(&task_sparkline_buckets(&audit_ts, n_buckets));

    text.push(Line::from(""));
    text.push(Line::from(Span::styled(
        "─── Activity Sparklines ─────────────────────────",
        Style::default().fg(theme.border),
    )));
    text.push(Line::from(vec![
        Span::styled(
            format!("  {} Tasks:  ", spinner_frame(app.tick)),
            Style::default().fg(theme.accent),
        ),
        Span::styled(task_bar, Style::default().fg(lerp_accent(theme, phase))),
    ]));
    text.push(Line::from(vec![
        Span::styled("     Audit:  ", Style::default().fg(theme.accent2)),
        Span::styled(
            audit_bar,
            Style::default().fg(lerp_accent(theme, (phase + 0.5).rem_euclid(1.0))),
        ),
    ]));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .title(format!(
            " Shared Hub Panel (Scroll: {}) ",
            app.scroll_offset
        ));
    let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: true });
    frame.render_widget(paragraph, area);
}
