//! Persistent settings view for ca tui (T5 / #139).
//!
//! Exposes ordinary and Advanced settings, inheritance status, select-only
//! provider profiles with non-secret badges, collapsible tree sections, and
//! danger-zone actions.

use crate::app::settings_ops::{format_field_status, SettingsSection};
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
    let st = &app.settings;

    let scope_str = st.active_scope.badge();
    let section_tabs = vec![
        section_span(SettingsSection::General, st.active_section, theme),
        Span::raw("  "),
        section_span(SettingsSection::TuiPreferences, st.active_section, theme),
        Span::raw("  "),
        section_span(SettingsSection::Profiles, st.active_section, theme),
        Span::raw("  "),
        section_span(SettingsSection::Advanced, st.active_section, theme),
        Span::raw("  "),
        section_span(SettingsSection::DangerZone, st.active_section, theme),
    ];

    let mut lines = vec![
        Line::from(vec![
            Span::styled(
                "Settings Scope: ",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                scope_str,
                Style::default()
                    .fg(theme.accent2)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " (Press [w] to toggle Global/Workspace scope)",
                Style::default().fg(theme.muted),
            ),
        ]),
        Line::from(""),
        Line::from(section_tabs),
        Line::from(""),
    ];

    match st.active_section {
        SettingsSection::General => {
            lines.push(Line::from(Span::styled(
                "── General Configuration ────────────────────────────────────────",
                Style::default().fg(theme.border_focus),
            )));
            lines.push(Line::from(vec![
                Span::raw("• Backup Retention: "),
                Span::styled(
                    format!("{} backups", eff.backup_retention),
                    Style::default().fg(theme.accent),
                ),
                Span::raw("  "),
                Span::styled(
                    format_field_status(eff.backup_retention_status),
                    Style::default().fg(theme.muted),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::raw("• Default Workspace: "),
                Span::styled(
                    eff.default_workspace.as_deref().unwrap_or("None (Global)"),
                    Style::default().fg(theme.accent),
                ),
                Span::raw("  "),
                Span::styled(
                    format_field_status(eff.default_workspace_status),
                    Style::default().fg(theme.muted),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::raw("• Default Session: "),
                Span::styled(
                    eff.default_session.as_deref().unwrap_or("None (Global)"),
                    Style::default().fg(theme.accent),
                ),
                Span::raw("  "),
                Span::styled(
                    format_field_status(eff.default_session_status),
                    Style::default().fg(theme.muted),
                ),
            ]));
        }
        SettingsSection::TuiPreferences => {
            lines.push(Line::from(Span::styled(
                "── TUI Preferences ([tui]) ─────────────────────────────────────",
                Style::default().fg(theme.border_focus),
            )));
            lines.push(Line::from(vec![
                Span::raw("• Prefix Chord: "),
                Span::styled(
                    &eff.tui.prefix_chord,
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("  [Press p to cycle]", Style::default().fg(theme.muted)),
            ]));
            lines.push(Line::from(vec![
                Span::raw("• Unicode Fallback: "),
                Span::styled(
                    eff.tui.unicode_fallback.to_string(),
                    Style::default().fg(if eff.tui.unicode_fallback {
                        theme.accent2
                    } else {
                        theme.muted
                    }),
                ),
                Span::styled("  [Press u to toggle]", Style::default().fg(theme.muted)),
            ]));
            lines.push(Line::from(vec![
                Span::raw("• Bell Notification: "),
                Span::styled(
                    eff.tui.bell_notification.to_string(),
                    Style::default().fg(if eff.tui.bell_notification {
                        theme.success
                    } else {
                        theme.muted
                    }),
                ),
                Span::styled("  [Press b to toggle]", Style::default().fg(theme.muted)),
            ]));
            lines.push(Line::from(vec![
                Span::raw("• High Contrast: "),
                Span::styled(
                    eff.tui.high_contrast.to_string(),
                    Style::default().fg(if eff.tui.high_contrast {
                        theme.accent2
                    } else {
                        theme.muted
                    }),
                ),
                Span::styled("  [Press c to toggle]", Style::default().fg(theme.muted)),
            ]));
            lines.push(Line::from(format!(
                "• Color Theme (session-local): {} — press [T] to cycle",
                app.theme_name.label()
            )));
        }
        SettingsSection::Profiles => {
            lines.push(Line::from(Span::styled(
                "── Provider Profiles (Select-Only) ──────────────────────────────",
                Style::default().fg(theme.border_focus),
            )));
            lines.push(Line::from(
                "Profiles are select-only in the TUI (profile create/edit stays desktop-only).",
            ));
            lines.push(Line::from(""));

            if app.read_model.profiles.is_empty() {
                lines.push(Line::from("  • No global provider profiles configured."));
            } else {
                for (i, prof) in app.read_model.profiles.iter().enumerate() {
                    let is_sel = i == st.selected_profile_cursor;
                    let marker = if is_sel { "▶ " } else { "  " };
                    let style = if is_sel {
                        Style::default()
                            .fg(theme.accent)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(theme.fg)
                    };
                    lines.push(Line::from(vec![
                        Span::styled(marker, style),
                        Span::styled(format!("{} ", prof.name), style),
                        Span::styled(
                            format!("({}) ", prof.provider),
                            Style::default().fg(theme.accent2),
                        ),
                        Span::styled(
                            format!("[{}]", prof.secret_badge),
                            Style::default().fg(theme.muted),
                        ),
                    ]));
                }
            }
        }
        SettingsSection::Advanced => {
            let toggle_symbol = if st.is_advanced_expanded {
                "[-] "
            } else {
                "[+] "
            };
            lines.push(Line::from(vec![
                Span::styled(
                    toggle_symbol,
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Advanced Settings Tree (Press [Space] to expand/collapse)",
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));

            if st.is_advanced_expanded {
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::raw("  ├─ Retention Days: "),
                    Span::styled(
                        eff.orchestration
                            .retention_days
                            .map(|d| d.to_string())
                            .unwrap_or_else(|| "unlimited".to_string()),
                        Style::default().fg(theme.accent),
                    ),
                    Span::raw("  "),
                    Span::styled(
                        format_field_status(eff.orchestration.retention_days_status),
                        Style::default().fg(theme.muted),
                    ),
                ]));
                lines.push(Line::from(vec![
                    Span::raw("  ├─ Export Enabled: "),
                    Span::styled(
                        eff.orchestration.export_enabled.to_string(),
                        Style::default().fg(theme.accent2),
                    ),
                    Span::raw("  "),
                    Span::styled(
                        format_field_status(eff.orchestration.export_enabled_status),
                        Style::default().fg(theme.muted),
                    ),
                ]));
                lines.push(Line::from(vec![
                    Span::raw("  ├─ Sandbox Strictness: "),
                    Span::styled(
                        eff.orchestration.sandbox_strictness.as_str(),
                        Style::default().fg(theme.accent),
                    ),
                    Span::raw("  "),
                    Span::styled(
                        format_field_status(eff.orchestration.sandbox_strictness_status),
                        Style::default().fg(theme.muted),
                    ),
                ]));
                lines.push(Line::from(vec![
                    Span::raw("  ├─ Confirm Broadcast: "),
                    Span::styled(
                        eff.orchestration.confirm_broadcast.to_string(),
                        Style::default().fg(theme.accent2),
                    ),
                    Span::raw("  "),
                    Span::styled(
                        format_field_status(eff.orchestration.confirm_broadcast_status),
                        Style::default().fg(theme.muted),
                    ),
                ]));
                lines.push(Line::from(vec![
                    Span::raw("  └─ Link Suggestion Mode: "),
                    Span::styled(
                        eff.orchestration.link_suggestion_mode.as_str(),
                        Style::default().fg(theme.accent),
                    ),
                    Span::raw("  "),
                    Span::styled(
                        format_field_status(eff.orchestration.link_suggestion_mode_status),
                        Style::default().fg(theme.muted),
                    ),
                ]));
            }
        }
        SettingsSection::DangerZone => {
            lines.push(Line::from(Span::styled(
                "── Danger Zone (Destructive Operations) ─────────────────────────",
                Style::default().fg(theme.error),
            )));
            lines.push(Line::from(Span::styled(
                "All danger operations enforce the cancel-first confirmation contract and typed matching.",
                Style::default().fg(theme.muted),
            )));
            lines.push(Line::from(""));

            let danger_items = [
                ("1", "Reset Workspace Overrides", theme.accent2, false),
                ("2", "Purge Workspace Transcript", theme.error, true),
                ("3", "Purge Workspace Memories", theme.error, true),
                ("4", "Purge All Workspace Data", theme.error, true),
                ("5", "Delete Provider Profile", theme.error, true),
            ];

            for (idx, title, color, is_red) in danger_items {
                let badge = if is_red {
                    "[IRREVERSIBLE]"
                } else {
                    "[RECOVERABLE]"
                };
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  [{idx}] "),
                        Style::default().fg(color).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(format!("{title} "), Style::default().fg(color)),
                    Span::styled(badge, Style::default().fg(theme.muted)),
                    Span::styled(
                        " (Press number to trigger)",
                        Style::default().fg(theme.muted),
                    ),
                ]));
            }
        }
    }

    if let Some(msg) = &app.settings.status_message {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("Status: {msg}"),
            Style::default().fg(theme.success),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Navigation: [Tab] Next Section │ [Shift+Tab] Prev Section │ [w] Toggle Scope │ [Space] Expand/Toggle",
        Style::default().fg(theme.muted),
    )));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .title(" Settings & Configuration Panel ");
    let paragraph = Paragraph::new(lines).block(block).wrap(Wrap { trim: true });
    frame.render_widget(paragraph, area);
}

fn section_span(
    section: SettingsSection,
    active: SettingsSection,
    theme: &crate::theme::Theme,
) -> Span<'static> {
    let is_active = section == active;
    let label = section.label();
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
