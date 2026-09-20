//! Modal dialog for launching owned interactive or observed read-only harnesses (T6).

use crate::app::state::AppState;
use hub::HarnessId;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

pub const AVAILABLE_HARNESSES: &[HarnessId] = &[
    HarnessId::Claude,
    HarnessId::Grok,
    HarnessId::Gemini,
    HarnessId::OpenCode,
    HarnessId::Cursor,
    HarnessId::Muse,
    HarnessId::Qwen,
    HarnessId::Kimi,
    HarnessId::Vibe,
    HarnessId::Chat,
];

pub fn draw_harness_launcher_modal(frame: &mut Frame, area: Rect, app: &AppState) {
    let theme = &app.theme;
    let ws_state = &app.harnesses;

    let width = 64.min(area.width.saturating_sub(4));
    let height = 18.min(area.height.saturating_sub(2));

    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let modal_area = Rect::new(x, y, width, height);

    frame.render_widget(Clear, modal_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent))
        .title(" Launch Harness Pane [T6] ");

    let inner = block.inner(modal_area);
    frame.render_widget(block, modal_area);

    let mut lines = Vec::new();

    // Mode Selector
    let mode_str = if ws_state.launcher_mode_is_owned {
        "● Owned Interactive (PTY Child Process)"
    } else {
        "👁 Observed Read-Only (Captured Session)"
    };
    lines.push(Line::from(vec![
        Span::styled(
            "Spawn Mode: ",
            Style::default()
                .fg(theme.accent2)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("[ {mode_str} ]"),
            Style::default()
                .fg(theme.success)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" (Press [m] to toggle)", Style::default().fg(theme.muted)),
    ]));
    lines.push(Line::from(""));

    lines.push(Line::from(Span::styled(
        "Select Harness to Launch:",
        Style::default()
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD),
    )));

    for (idx, harness) in AVAILABLE_HARNESSES.iter().enumerate() {
        let is_selected = idx == ws_state.launcher_selected_idx;
        let prefix = if is_selected { " ▶ " } else { "   " };
        let exe = (*harness).executable();
        let label = format!("{prefix}{:<12} (executable: {exe})", harness.as_str());

        let style = if is_selected {
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD | Modifier::REVERSED)
        } else {
            Style::default().fg(theme.fg)
        };
        lines.push(Line::from(Span::styled(label, style)));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("[↑/↓]", Style::default().fg(theme.accent)),
        Span::raw(" Navigate   "),
        Span::styled("[m]", Style::default().fg(theme.accent)),
        Span::raw(" Mode   "),
        Span::styled("[Enter]", Style::default().fg(theme.accent)),
        Span::raw(" Launch   "),
        Span::styled("[Esc]", Style::default().fg(theme.accent)),
        Span::raw(" Cancel"),
    ]));

    let p = Paragraph::new(lines).wrap(Wrap { trim: true });
    frame.render_widget(p, inner);
}
