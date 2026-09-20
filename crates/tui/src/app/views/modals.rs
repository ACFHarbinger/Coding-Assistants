use crate::app::composer::RecipientMode;
use crate::app::state::AppState;
use crate::app::ui::centered_rect;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

pub fn draw_composer_modal(frame: &mut Frame, area: Rect, app: &AppState) {
    let popup_area = centered_rect(75, 75, area);
    frame.render_widget(Clear, popup_area);
    let theme = &app.theme;

    let sess = app.session_id.as_deref().unwrap_or("general");
    let mut lines = vec![
        Line::from(vec![
            Span::styled(
                "Recipient Mode: ",
                Style::default()
                    .fg(theme.accent2)
                    .add_modifier(Modifier::BOLD),
            ),
            mode_span(
                "All",
                app.composer.recipient_mode == RecipientMode::All,
                theme,
            ),
            Span::raw("  "),
            mode_span(
                "Subset",
                app.composer.recipient_mode == RecipientMode::Subset,
                theme,
            ),
            Span::raw("  "),
            mode_span(
                "Single",
                app.composer.recipient_mode == RecipientMode::Single,
                theme,
            ),
            Span::styled(
                "  (Press [Tab] or [1/2/3] to switch)",
                Style::default().fg(theme.muted),
            ),
        ]),
        Line::from(""),
    ];

    // Detail row for Subset or Single recipient
    match app.composer.recipient_mode {
        RecipientMode::All => {
            lines.push(Line::from(vec![
                Span::styled("Target: ", Style::default().fg(theme.muted)),
                Span::styled(
                    "All non-human members of this session/team roster.",
                    Style::default().fg(theme.success),
                ),
            ]));
        }
        RecipientMode::Subset => {
            let mut subset_spans = vec![Span::styled("Select: ", Style::default().fg(theme.muted))];
            let keys: Vec<String> = app.composer.selected_subset.keys().cloned().collect();
            for (i, key) in keys.iter().enumerate() {
                let selected = *app.composer.selected_subset.get(key).unwrap_or(&true);
                let is_cursor = i == app.composer.subset_cursor;
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
                subset_spans.push(Span::styled(format!("{box_str} {key}  "), style));
            }
            subset_spans.push(Span::styled(
                "([Space] toggle, [←/→] move)",
                Style::default().fg(theme.muted),
            ));
            lines.push(Line::from(subset_spans));
        }
        RecipientMode::Single => {
            lines.push(Line::from(vec![
                Span::styled("Target Agent: ", Style::default().fg(theme.muted)),
                Span::styled(
                    format!("[{}]", app.composer.single_recipient),
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "  ([←/→] cycle target agent)",
                    Style::default().fg(theme.muted),
                ),
            ]));
        }
    }

    // Intent tags row
    lines.push(Line::from(""));
    let task_tag_style = if app.composer.is_task_tag {
        Style::default()
            .fg(Color::Black)
            .bg(theme.accent)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.muted)
    };
    let wake_tag_style = if app.composer.is_wake_tag {
        Style::default()
            .fg(Color::Black)
            .bg(theme.accent2)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.muted)
    };

    lines.push(Line::from(vec![
        Span::styled(
            "Intent Tags: ",
            Style::default()
                .fg(theme.accent2)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            if app.composer.is_task_tag {
                " [TASK (Active)] "
            } else {
                " [TASK] "
            },
            task_tag_style,
        ),
        Span::raw(" "),
        Span::styled(
            if app.composer.is_wake_tag {
                " [WAKE (Active)] "
            } else {
                " [WAKE] "
            },
            wake_tag_style,
        ),
        Span::styled(
            "  (Press [Ctrl+T] for Task, [Ctrl+W] for Wake)",
            Style::default().fg(theme.muted),
        ),
    ]));

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Message Body:",
        Style::default()
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD),
    )));

    // Message body rendering
    let body_text = if app.composer.body.is_empty() {
        "Type message here... (explicit Send required)"
    } else {
        &app.composer.body
    };
    lines.push(Line::from(vec![
        Span::raw(body_text),
        Span::styled("█", Style::default().fg(theme.accent)),
    ]));

    if let Some(err) = &app.composer.error_message {
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
        "Commands: [Enter/Ctrl+S] Explicit Send │ [Ctrl+T] Tag Task │ [Ctrl+W] Tag Wake │ [Esc] Cancel",
        Style::default().fg(theme.muted),
    )));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_focus))
        .title(format!(" Message Composer (#{sess}) "))
        .style(Style::default().bg(Color::Reset));
    frame.render_widget(
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false }),
        popup_area,
    );
}

fn mode_span<'a>(label: &'a str, active: bool, theme: &crate::theme::Theme) -> Span<'a> {
    if active {
        Span::styled(
            format!(" [{label}] "),
            Style::default()
                .fg(Color::Black)
                .bg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(format!("  {label}  "), Style::default().fg(theme.muted))
    }
}

pub fn draw_confirmation_modal(frame: &mut Frame, area: Rect, app: &AppState) {
    let popup_area = centered_rect(65, 30, area);
    frame.render_widget(Clear, popup_area);
    let theme = &app.theme;

    let prompt_desc = app
        .composer
        .confirmation_prompt
        .as_ref()
        .map(|p| p.description())
        .unwrap_or_else(|| String::from("Confirm action?"));

    let lines = vec![
        Line::from(Span::styled(
            "⚠️ Confirmation Required (C10-C13 Policy Defaults)",
            Style::default()
                .fg(theme.error)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::raw(prompt_desc)),
        Line::from(""),
        Line::from(Span::styled(
            "Wakes, new enrollments, and broadcasts confirm by default.",
            Style::default().fg(theme.muted),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "[y / Enter] Confirm & Send",
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("    "),
            Span::styled(
                "[n / Esc] Cancel",
                Style::default()
                    .fg(theme.error)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.error))
        .title(" Confirm Send Action ")
        .style(Style::default().bg(Color::Reset));
    frame.render_widget(
        Paragraph::new(lines).block(block).wrap(Wrap { trim: true }),
        popup_area,
    );
}

pub fn draw_delivery_outcomes_modal(frame: &mut Frame, area: Rect, app: &AppState) {
    let popup_area = centered_rect(75, 55, area);
    frame.render_widget(Clear, popup_area);
    let theme = &app.theme;

    let outcomes = app.composer.delivery_outcomes.as_deref().unwrap_or(&[]);
    let mut lines = vec![
        Line::from(Span::styled(
            "Message Delivery Outcomes:",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    if outcomes.is_empty() {
        lines.push(Line::from("  • No delivery outcome records returned."));
    } else {
        for outcome in outcomes.iter().skip(app.composer.outcome_scroll).take(8) {
            let status_badge = if outcome.accepted {
                Span::styled(
                    " [ACCEPTED] ",
                    Style::default()
                        .fg(theme.success)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled(
                    " [REJECTED] ",
                    Style::default()
                        .fg(theme.error)
                        .add_modifier(Modifier::BOLD),
                )
            };

            let enroll_badge = if outcome.enrolled { " (enrolled)" } else { "" };
            let wake_badge = if outcome.wake_requested {
                " (wake requested)"
            } else {
                ""
            };
            let reason = outcome
                .reason
                .as_deref()
                .unwrap_or(&outcome.policy_decision);

            lines.push(Line::from(vec![
                status_badge,
                Span::styled(
                    format!("Target: {} ", outcome.to_agent),
                    Style::default().fg(theme.accent2),
                ),
                Span::styled(
                    format!("{enroll_badge}{wake_badge} "),
                    Style::default().fg(theme.muted),
                ),
                Span::raw(format!("Decision: {reason}")),
            ]));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Press [Enter] or [Esc] to dismiss outcomes.",
        Style::default().fg(theme.muted),
    )));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_focus))
        .title(" Delivery Outcomes ")
        .style(Style::default().bg(Color::Reset));
    frame.render_widget(
        Paragraph::new(lines).block(block).wrap(Wrap { trim: true }),
        popup_area,
    );
}
