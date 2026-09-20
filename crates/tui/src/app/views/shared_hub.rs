//! Shared Hub view for ca tui (T2 / #136, T5 / #139).
//!
//! Renders active tasks, audit journal review with hash-chain verification,
//! and agent budgets with truthful freshness.

use crate::app::state::AppState;
use crate::theme::{lerp_accent, sparkline_string, spinner_frame, task_sparkline_buckets};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HubViewMode {
    #[default]
    Tasks,
    AuditJournal,
    Budgets,
}

impl HubViewMode {
    pub fn next(self) -> Self {
        match self {
            Self::Tasks => Self::AuditJournal,
            Self::AuditJournal => Self::Budgets,
            Self::Budgets => Self::Tasks,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Tasks => "Tasks",
            Self::AuditJournal => "Audit Journal",
            Self::Budgets => "Budgets & Telemetry",
        }
    }
}

pub fn draw_shared_hub_view(frame: &mut Frame, area: Rect, app: &AppState) {
    let theme = &app.theme;
    let home = app.home_dir.display().to_string();

    let tab_spans = vec![
        hub_tab_span(HubViewMode::Tasks, app.hub_view_mode, theme),
        Span::raw("  "),
        hub_tab_span(HubViewMode::AuditJournal, app.hub_view_mode, theme),
        Span::raw("  "),
        hub_tab_span(HubViewMode::Budgets, app.hub_view_mode, theme),
        Span::styled(
            "  (Press [u] or [1/2/3] to switch)",
            Style::default().fg(theme.muted),
        ),
    ];

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
        Line::from(tab_spans),
        Line::from(""),
    ];

    match app.hub_view_mode {
        HubViewMode::Tasks => {
            text.push(Line::from(Span::styled(
                "── Active & Recent Tasks ────────────────────────────────────────",
                Style::default().fg(theme.border_focus),
            )));
            if app.read_model.tasks.is_empty() {
                text.push(Line::from("  • No durable hub tasks found."));
            } else {
                for task in app.read_model.tasks.iter().skip(app.scroll_offset).take(8) {
                    let status_color = match format!("{:?}", task.status).to_lowercase().as_str() {
                        s if s.contains("completed") => theme.success,
                        s if s.contains("running") || s.contains("in_progress") => theme.accent,
                        _ => theme.muted,
                    };
                    text.push(Line::from(vec![
                        Span::styled(
                            format!("  [{:?}] ", task.status),
                            Style::default()
                                .fg(status_color)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(&task.title, Style::default().fg(theme.fg)),
                        Span::styled(
                            format!(" (id: {})", &task.id[..task.id.len().min(8)]),
                            Style::default().fg(theme.muted),
                        ),
                    ]));
                }
            }
        }
        HubViewMode::AuditJournal => {
            text.push(Line::from(Span::styled(
                "── Hash-Chained Audit Journal Review ───────────────────────────",
                Style::default().fg(theme.border_focus),
            )));

            // Hash chain verification check
            let events = &app.read_model.all_audit_events;
            let mut chain_valid = true;
            for i in 1..events.len() {
                if events[i].previous_hash.as_deref() != Some(events[i - 1].event_hash.as_str()) {
                    chain_valid = false;
                    break;
                }
            }

            let chain_badge = if events.is_empty() {
                Span::styled(" [Empty Journal] ", Style::default().fg(theme.muted))
            } else if chain_valid {
                Span::styled(
                    " [✓ Hash Chain Verified] ",
                    Style::default()
                        .fg(theme.success)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled(
                    " [✗ Hash Chain Discontinuity] ",
                    Style::default()
                        .fg(theme.error)
                        .add_modifier(Modifier::BOLD),
                )
            };

            text.push(Line::from(vec![
                Span::raw("Journal Integrity: "),
                chain_badge,
                Span::styled(
                    format!(" ({} events recorded)", events.len()),
                    Style::default().fg(theme.muted),
                ),
            ]));
            text.push(Line::from(""));

            if events.is_empty() {
                text.push(Line::from("  • No audit events recorded."));
            } else {
                for event in events.iter().skip(app.scroll_offset).take(8) {
                    let hash_preview = if event.event_hash.len() > 10 {
                        &event.event_hash[..10]
                    } else {
                        &event.event_hash
                    };
                    text.push(Line::from(vec![
                        Span::styled(
                            format!("  [{}] ", event.operation),
                            Style::default()
                                .fg(theme.accent)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            format!("{} ", event.path),
                            Style::default().fg(theme.accent2),
                        ),
                        Span::styled(
                            format!("status: {} ", event.status),
                            Style::default().fg(if event.status == "approved" {
                                theme.success
                            } else {
                                theme.muted
                            }),
                        ),
                        Span::styled(
                            format!("hash: {hash_preview}…"),
                            Style::default().fg(theme.muted),
                        ),
                    ]));
                }
            }
        }
        HubViewMode::Budgets => {
            text.push(Line::from(Span::styled(
                "── Agent Budgets & Truthful Freshness ──────────────────────────",
                Style::default().fg(theme.border_focus),
            )));

            if app.read_model.agent_budgets.is_empty() {
                text.push(Line::from("  • No agent budgets configured in hub."));
            } else {
                for b in &app.read_model.agent_budgets {
                    let pct = if b.limit_units > 0.0 {
                        (b.spent_units / b.limit_units) * 100.0
                    } else {
                        0.0
                    };
                    let status_badge = if b.paused {
                        Span::styled(
                            " [PAUSED] ",
                            Style::default()
                                .fg(theme.error)
                                .add_modifier(Modifier::BOLD),
                        )
                    } else {
                        Span::styled(" [ACTIVE] ", Style::default().fg(theme.success))
                    };
                    text.push(Line::from(vec![
                        Span::styled(
                            format!("  • Agent: {:<10} ", b.agent_id),
                            Style::default()
                                .fg(theme.accent)
                                .add_modifier(Modifier::BOLD),
                        ),
                        status_badge,
                        Span::styled(
                            format!("Spent: {}/{} ({:.1}%) ", b.spent_units, b.limit_units, pct),
                            Style::default().fg(theme.fg),
                        ),
                        Span::styled(
                            format!("Freshness: Refreshed at {}", b.updated_at),
                            Style::default().fg(theme.muted),
                        ),
                    ]));
                }
            }

            text.push(Line::from(""));
            text.push(Line::from(Span::styled(
                "Agent Metric Counters (Live Process / Disk Sync):",
                Style::default().fg(theme.accent2),
            )));
            if app.read_model.agent_metrics.is_empty() {
                text.push(Line::from("  • No agent metric counters reported."));
            } else {
                for m in &app.read_model.agent_metrics {
                    text.push(Line::from(vec![
                        Span::styled(
                            format!("  • {:<10} ", m.agent_id),
                            Style::default().fg(theme.accent),
                        ),
                        Span::raw(format!(
                            "Tokens: {} (cached: {}) │ Lines: {} │ Calls: {} ",
                            m.tokens_used, m.tokens_cached, m.lines_written, m.provider_calls
                        )),
                        Span::styled("[Live]", Style::default().fg(theme.success)),
                    ]));
                }
            }
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
            " Shared Hub Panel [{}] (Scroll: {}) ",
            app.hub_view_mode.label(),
            app.scroll_offset
        ));
    let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: true });
    frame.render_widget(paragraph, area);
}

fn hub_tab_span(
    mode: HubViewMode,
    active: HubViewMode,
    theme: &crate::theme::Theme,
) -> Span<'static> {
    let is_active = mode == active;
    let label = mode.label();
    if is_active {
        Span::styled(
            format!(" [{label}] "),
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )
    } else {
        Span::styled(format!("  {label}  "), Style::default().fg(theme.muted))
    }
}
