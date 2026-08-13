use super::state::AppState;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn draw_orchestrate_view(frame: &mut Frame, area: Rect, app: &AppState) {
    let ws = app
        .workspace_path
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "None".to_string());
    let sess = app.session_id.as_deref().unwrap_or("None");

    let team_roster = if app.read_model.team_members.is_empty() {
        "• Default roster: human, claude, grok, gemini, chat".to_string()
    } else {
        let members: Vec<String> = app
            .read_model
            .team_members
            .iter()
            .map(|agent| format!("{} ({})", agent.display_name, agent.id))
            .collect();
        format!("• Enrolled roster: {}", members.join(", "))
    };

    let session_count = app.read_model.work_sessions.len();
    let sessions_summary = if session_count == 0 {
        "• Work Sessions: None active".to_string()
    } else {
        let session_names: Vec<String> = app
            .read_model
            .work_sessions
            .iter()
            .take(5)
            .map(|s| format!("{} ({} members)", s.name, s.member_ids.len()))
            .collect();
        format!(
            "• Work Sessions ({}): {}",
            session_count,
            session_names.join(" | ")
        )
    };

    let text = vec![
        Line::from(vec![
            Span::styled(
                "Workspace Root: ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(ws),
            if app.is_workspace_overridden {
                Span::styled(
                    " [Invocation Override]",
                    Style::default().fg(Color::Magenta),
                )
            } else {
                Span::styled(" [Default]", Style::default().fg(Color::Green))
            },
        ]),
        Line::from(vec![
            Span::styled(
                "Active Session: ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(sess),
            if app.is_session_overridden {
                Span::styled(
                    " [Invocation Override]",
                    Style::default().fg(Color::Magenta),
                )
            } else {
                Span::styled(" [Default]", Style::default().fg(Color::Green))
            },
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Team Roster & Orchestration Controls:",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(team_roster),
        Line::from(sessions_summary),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Orchestrate Panel ");
    let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: true });
    frame.render_widget(paragraph, area);
}

pub fn draw_chat_view(frame: &mut Frame, area: Rect, app: &AppState) {
    let sess = app.session_id.as_deref().unwrap_or("general");

    let mut text = vec![
        Line::from(vec![
            Span::styled(
                "Active Channel/Session: ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!("#{}", sess), Style::default().fg(Color::Green)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Message Stream:",
            Style::default().fg(Color::Yellow),
        )),
    ];

    if app.read_model.channel_messages.is_empty() {
        text.push(Line::from(" [System] No messages in this channel yet. Send a message via CLI or Desktop to start."));
    } else {
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
                Span::styled(format!(" [{}] ", sender), Style::default().fg(Color::Cyan)),
                Span::raw(body_preview),
            ]));
        }
    }

    let block = Block::default().borders(Borders::ALL).title(format!(
        " Chat & Memory Panel (Scroll: {}) ",
        app.scroll_offset
    ));
    let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: true });
    frame.render_widget(paragraph, area);
}

pub fn draw_shared_hub_view(frame: &mut Frame, area: Rect, app: &AppState) {
    let home = app.home_dir.display().to_string();
    let task_count = app.read_model.tasks.len();
    let audit_count = app.read_model.audit_events.len();

    let mut text = vec![
        Line::from(vec![
            Span::styled(
                "Hub Data Location: ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(home),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Active Tasks & Audit Stream:",
            Style::default().fg(Color::Yellow),
        )),
        Line::from(format!("• Durable Hub tasks: {} tasks", task_count)),
        Line::from(format!("• Settings audit events: {} recorded", audit_count)),
    ];

    if !app.read_model.tasks.is_empty() {
        text.push(Line::from(""));
        text.push(Line::from(Span::styled(
            "Recent Tasks:",
            Style::default().fg(Color::Cyan),
        )));
        for task in app.read_model.tasks.iter().skip(app.scroll_offset).take(5) {
            text.push(Line::from(format!("  [{:?}] {}", task.status, task.id)));
        }
    }

    if !app.read_model.audit_events.is_empty() {
        text.push(Line::from(""));
        text.push(Line::from(Span::styled(
            "Recent Settings Audit Events:",
            Style::default().fg(Color::Cyan),
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

    let block = Block::default().borders(Borders::ALL).title(format!(
        " Shared Hub Panel (Scroll: {}) ",
        app.scroll_offset
    ));
    let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: true });
    frame.render_widget(paragraph, area);
}

pub fn draw_settings_view(frame: &mut Frame, area: Rect, app: &AppState) {
    let eff = &app.read_model.effective_settings;

    let text = vec![
        Line::from(Span::styled(
            "Persistent Settings (toml) Configuration:",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(format!(
            "• Backup Retention: {} backups",
            eff.backup_retention
        )),
        Line::from(format!(
            "• Default Workspace: {}",
            eff.default_workspace.as_deref().unwrap_or("None (Global)")
        )),
        Line::from(format!(
            "• Default Session: {}",
            eff.default_session.as_deref().unwrap_or("None (Global)")
        )),
        Line::from(vec![
            Span::raw("• Workspace override mode: "),
            Span::styled(
                if app.is_default_workspace_persisted {
                    "Persisted Default"
                } else if app.is_workspace_overridden {
                    "Invocation Override"
                } else {
                    "Global Default"
                },
                Style::default().fg(Color::Yellow),
            ),
        ]),
        Line::from(vec![
            Span::raw("• Session override mode: "),
            Span::styled(
                if app.is_default_session_persisted {
                    "Persisted Default"
                } else if app.is_session_overridden {
                    "Invocation Override"
                } else {
                    "Global Default"
                },
                Style::default().fg(Color::Yellow),
            ),
        ]),
        Line::from(format!(
            "• Profiles configured: {} global profiles",
            eff.profiles.len()
        )),
        Line::from(format!(
            "• Harnesses configured: {} harnesses",
            eff.harnesses.len()
        )),
        Line::from(""),
        Line::from(Span::styled(
            "TUI Preferences ([tui]):",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(format!("• Prefix Chord: {}", eff.tui.prefix_chord)),
        Line::from(format!("• Unicode Fallback: {}", eff.tui.unicode_fallback)),
        Line::from(format!(
            "• Bell Notification: {}",
            eff.tui.bell_notification
        )),
        Line::from(format!("• High Contrast: {}", eff.tui.high_contrast)),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Settings Panel ");
    let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: true });
    frame.render_widget(paragraph, area);
}
