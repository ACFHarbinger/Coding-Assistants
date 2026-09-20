use crate::app::state::AppState;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn draw_settings_view(frame: &mut Frame, area: Rect, app: &AppState) {
    let theme = &app.theme;
    let eff = &app.read_model.effective_settings;

    let text = vec![
        Line::from(Span::styled(
            "Persistent Settings (toml) Configuration:",
            Style::default()
                .fg(theme.accent)
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
                Style::default().fg(theme.accent2),
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
                Style::default().fg(theme.accent2),
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
                .fg(theme.accent2)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(format!("• Prefix Chord: {}", eff.tui.prefix_chord)),
        Line::from(format!("• Unicode Fallback: {}", eff.tui.unicode_fallback)),
        Line::from(format!(
            "• Bell Notification: {}",
            eff.tui.bell_notification
        )),
        Line::from(format!("• High Contrast: {}", eff.tui.high_contrast)),
        Line::from(format!(
            "• Color Theme (session-local): {} — press T to cycle",
            app.theme_name.label()
        )),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .title(" Settings Panel ");
    let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: true });
    frame.render_widget(paragraph, area);
}
