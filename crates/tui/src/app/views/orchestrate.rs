use super::super::ambient::draw_orchestrate_ambient;
use crate::app::state::AppState;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn draw_orchestrate_view(frame: &mut Frame, area: Rect, app: &AppState) {
    let theme = &app.theme;
    let ws = app
        .workspace_path
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "None".to_string());
    let sess = app.session_id.as_deref().unwrap_or("general");

    let mut content_lines = vec![
        Line::from(vec![
            Span::styled(
                "Workspace Root: ",
                Style::default()
                    .fg(theme.accent2)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(ws),
            if app.is_workspace_overridden {
                Span::styled(" [Invocation Override]", Style::default().fg(theme.error))
            } else {
                Span::styled(" [Default]", Style::default().fg(theme.success))
            },
        ]),
        Line::from(vec![
            Span::styled(
                "Active Work Session: ",
                Style::default()
                    .fg(theme.accent2)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!("#{sess}"), Style::default().fg(theme.success)),
            if app.is_session_overridden {
                Span::styled(" [Invocation Override]", Style::default().fg(theme.error))
            } else {
                Span::styled(" [Default]", Style::default().fg(theme.success))
            },
            Span::styled(
                "  (Press [s] to Switch, [n] for New Session)",
                Style::default().fg(theme.muted),
            ),
        ]),
        Line::from(""),
    ];

    // ── Team Roster & Presence ───────────────────────────────────────────────
    content_lines.push(Line::from(Span::styled(
        "Team Roster & Status:",
        Style::default()
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD),
    )));

    if app.read_model.team_members.is_empty() {
        content_lines.push(Line::from(
            "  • Default roster: human, claude, grok, gemini, chat",
        ));
    } else {
        let members: Vec<String> = app
            .read_model
            .team_members
            .iter()
            .map(|a| {
                let status_dot = "●";
                format!("{status_dot} {} [{}]", a.display_name, a.id)
            })
            .collect();
        content_lines.push(Line::from(format!("  {}", members.join("   "))));
    }

    // ── Pending Wake Gate Approvals (C12) ────────────────────────────────────
    let pending_gates = &app.read_model.pending_gates;
    content_lines.push(Line::from(""));
    content_lines.push(Line::from(vec![
        Span::styled(
            format!("Wake Gate Approvals ({} pending):", pending_gates.len()),
            Style::default()
                .fg(if pending_gates.is_empty() {
                    theme.accent2
                } else {
                    theme.error
                })
                .add_modifier(Modifier::BOLD),
        ),
        if !pending_gates.is_empty() {
            Span::styled(
                "  [a] Approve  [d] Deny  [↑/↓] Select",
                Style::default().fg(theme.accent),
            )
        } else {
            Span::styled(
                "  (all requests resolved)",
                Style::default().fg(theme.muted),
            )
        },
    ]));

    if pending_gates.is_empty() {
        content_lines.push(Line::from("  • No pending human-gate wake requests."));
    } else {
        for (i, gate) in pending_gates.iter().take(3).enumerate() {
            let is_selected = i == app.approvals.selected_approval_index;
            let marker = if is_selected { "▶ " } else { "  " };
            let body_preview: String = gate.body.chars().take(50).collect();
            let style = if is_selected {
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg)
            };
            content_lines.push(Line::from(vec![
                Span::styled(marker, style),
                Span::styled(
                    format!(
                        "[{}] From {} -> {}: ",
                        gate.status,
                        gate.from_agent,
                        gate.to_agents.join(", ")
                    ),
                    style,
                ),
                Span::styled(
                    format!("\"{body_preview}...\""),
                    Style::default().fg(theme.muted),
                ),
            ]));
        }
    }

    // ── Active Tasks ─────────────────────────────────────────────────────────
    content_lines.push(Line::from(""));
    content_lines.push(Line::from(Span::styled(
        format!("Active Tasks ({} total):", app.read_model.tasks.len()),
        Style::default()
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD),
    )));

    if app.read_model.tasks.is_empty() {
        content_lines.push(Line::from("  • No active tasks recorded in Hub."));
    } else {
        for task in app.read_model.tasks.iter().take(4) {
            let status_style = match task.status.to_lowercase().as_str() {
                "completed" | "done" => Style::default().fg(theme.success),
                "in_progress" | "running" => Style::default().fg(theme.accent),
                "failed" | "error" => Style::default().fg(theme.error),
                _ => Style::default().fg(theme.muted),
            };
            let preview: String = task.title.chars().take(60).collect();
            let agents_str = if !task.open_agents.is_empty() {
                task.open_agents.join(", ")
            } else if !task.pending_agents.is_empty() {
                task.pending_agents.join(", ")
            } else {
                "standing".to_string()
            };
            content_lines.push(Line::from(vec![
                Span::styled(format!("  [{}] ", task.status), status_style),
                Span::styled(agents_str, Style::default().fg(theme.accent2)),
                Span::raw(" : "),
                Span::raw(preview),
            ]));
        }
    }

    let content_height = content_lines.len() as u16 + 2;
    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .title(" Orchestrate Panel ");
    frame.render_widget(&outer, area);
    let inner = outer.inner(area);

    let ambient_height = inner
        .height
        .saturating_sub(content_height.min(inner.height));
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(content_height.min(inner.height)),
            Constraint::Length(ambient_height),
        ])
        .split(inner);

    frame.render_widget(
        Paragraph::new(content_lines).wrap(Wrap { trim: true }),
        chunks[0],
    );

    if ambient_height >= 3 {
        draw_orchestrate_ambient(frame, chunks[1], app);
    }
}
