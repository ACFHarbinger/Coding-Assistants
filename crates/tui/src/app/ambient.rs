//! Ambient fill for panels whose real content is much shorter than the
//! terminal's available height — an animated agent-presence strip and a
//! Hub-activity sparkline, both driven by `AppState.tick` and `AppState.theme`,
//! so the space reads as alive rather than as dead padding.

use super::state::AppState;
use crate::theme::{
    agent_pulse_phase, lerp_accent, sparkline_string, spinner_frame, task_sparkline_buckets,
};
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

/// Fills the unused vertical space in the Orchestrate panel with two ambient
/// elements, both driven by `app.tick` and `app.theme`:
///
/// 1. **Agent presence strip** — one row per enrolled team member (or a
///    default roster when empty), each showing an animated pulse-dot
///    (brightness waves staggered per-agent via `agent_pulse_phase`) and the
///    agent's display name in a muted style.
///
/// 2. **Task activity sparkline** — a single row of Unicode block characters
///    bucketed from `app.read_model.tasks`' `updated_at` timestamps by real
///    elapsed time (oldest to newest), prefixed with a live spinner glyph,
///    giving a glanceable "has there been recent task churn?" signal without
///    any scrolling.
pub(super) fn draw_orchestrate_ambient(frame: &mut Frame, area: Rect, app: &AppState) {
    let theme = &app.theme;
    let mut lines: Vec<Line> = vec![Line::from(Span::styled(
        "─── Agent Presence ─────────────────────────────",
        Style::default().fg(theme.border),
    ))];

    let default_agents = ["human", "claude", "grok", "gemini", "chat"];
    let agent_rows: Vec<(&str, &str)> = if app.read_model.team_members.is_empty() {
        default_agents.iter().map(|n| (*n, *n)).collect()
    } else {
        app.read_model
            .team_members
            .iter()
            .map(|a| (a.display_name.as_str(), a.id.as_str()))
            .collect()
    };

    // How many agent rows fit before we need to leave room for the sparkline (3 rows).
    let sparkline_rows = 3u16;
    let max_agent_rows = area.height.saturating_sub(1 + sparkline_rows) as usize; // 1 = separator

    for (i, (name, _id)) in agent_rows.iter().take(max_agent_rows).enumerate() {
        let phase = agent_pulse_phase(i, app.tick);
        let dot_color = lerp_accent(theme, phase);
        let dot = if phase > 0.6 {
            "●"
        } else if phase > 0.2 {
            "◉"
        } else {
            "○"
        };
        lines.push(Line::from(vec![
            Span::styled(format!("  {dot} "), Style::default().fg(dot_color)),
            Span::styled(name.to_string(), Style::default().fg(theme.muted)),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "─── Task Activity ───────────────────────────────",
        Style::default().fg(theme.border),
    )));

    let n_buckets = (area.width.saturating_sub(4) as usize).clamp(1, 40);
    let timestamps: Vec<&str> = app
        .read_model
        .tasks
        .iter()
        .map(|t| t.updated_at.as_str())
        .collect();
    let buckets = task_sparkline_buckets(&timestamps, n_buckets);
    // Phase-shift the sparkline highlight color slowly with tick for ambient movement.
    let phase = (app.tick as f32 / 80.0).rem_euclid(1.0);
    let bar_color = lerp_accent(theme, phase);
    lines.push(Line::from(vec![
        Span::styled(
            format!("  {} ", spinner_frame(app.tick)),
            Style::default().fg(theme.accent),
        ),
        Span::styled(sparkline_string(&buckets), Style::default().fg(bar_color)),
    ]));

    frame.render_widget(Paragraph::new(lines), area);
}
