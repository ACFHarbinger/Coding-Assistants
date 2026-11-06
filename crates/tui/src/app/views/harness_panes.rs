//! Rendering for owned and observed harness panes, tabbed bar, and split tiles (T6).

use crate::app::pane_ops::{HarnessPane, PaneKind};
use crate::app::runner::is_ascii_terminal;
use crate::app::state::AppState;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn draw_harness_workspace_view(frame: &mut Frame, area: Rect, app: &AppState) {
    let ws_state = &app.harnesses;

    if ws_state.panes.is_empty() {
        draw_empty_harness_view(frame, area, app);
        return;
    }

    // Top: Tabbed active-pane bar (height 3)
    // Bottom: Status/shortcuts bar (height 1)
    // Center: Terminal viewports (remaining)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(1),
        ])
        .split(area);

    draw_pane_tabs(frame, chunks[0], app);
    draw_pane_viewports(frame, chunks[1], app);
    draw_pane_footer(frame, chunks[2], app);
}

fn draw_empty_harness_view(frame: &mut Frame, area: Rect, app: &AppState) {
    let theme = &app.theme;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .title(" Harness Workspace [No Open Panes] ");

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "No active harness panes are currently open.",
            Style::default().fg(theme.muted),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("Press ", Style::default().fg(theme.muted)),
            Span::styled(
                "[c]",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" or ", Style::default().fg(theme.muted)),
            Span::styled(
                "[Ctrl+B c]",
                Style::default()
                    .fg(theme.accent2)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " to launch an owned or observed harness pane.",
                Style::default().fg(theme.muted),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Supported harnesses: ", Style::default().fg(theme.muted)),
            Span::styled(
                "claude, grok, gemini, opencode, cursor, muse, qwen, kimi, vibe, chat",
                Style::default().fg(theme.accent),
            ),
        ]),
    ];

    let p = Paragraph::new(lines).block(block).wrap(Wrap { trim: true });
    frame.render_widget(p, area);
}

fn draw_pane_tabs(frame: &mut Frame, area: Rect, app: &AppState) {
    let theme = &app.theme;
    let ws_state = &app.harnesses;
    let is_ascii = app.read_model.effective_settings.tui.unicode_fallback || is_ascii_terminal();

    let mut tab_spans = Vec::new();
    for (idx, pane) in ws_state.panes.iter().enumerate() {
        let is_active = idx == ws_state.active_pane_idx;
        let badge = pane.status.badge_for(is_ascii);
        let kind_str = match pane.kind {
            PaneKind::Owned => "owned",
            PaneKind::Observed => "obs",
        };
        let label = format!(
            " [{}:{}] {} {} ",
            idx + 1,
            pane.harness.as_str(),
            kind_str,
            badge
        );

        let style = if is_active {
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else {
            Style::default().fg(theme.muted)
        };
        tab_spans.push(Span::styled(label, style));
        tab_spans.push(Span::raw(" "));
    }

    // Add Launch button tab
    tab_spans.push(Span::styled(
        " [+ Launch (c)] ",
        Style::default().fg(theme.accent2),
    ));

    if app.is_prefix_mode_active {
        tab_spans.push(Span::styled(
            " [PREFIX Ctrl+B ACTIVE] ",
            Style::default()
                .fg(theme.error)
                .add_modifier(Modifier::BOLD | Modifier::REVERSED),
        ));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .title(" Harness Workspace Panes ");

    let p = Paragraph::new(Line::from(tab_spans)).block(block);
    frame.render_widget(p, area);
}

fn draw_pane_viewports(frame: &mut Frame, area: Rect, app: &AppState) {
    let ws_state = &app.harnesses;

    // If split view is enabled and terminal is wide enough (>= 100 cols) and we have >= 2 panes:
    if ws_state.is_split_view && area.width >= 100 && ws_state.panes.len() >= 2 {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        let left_idx = ws_state.active_pane_idx;
        let right_idx = (ws_state.active_pane_idx + 1) % ws_state.panes.len();

        draw_single_pane(frame, cols[0], &ws_state.panes[left_idx], true, app);
        draw_single_pane(frame, cols[1], &ws_state.panes[right_idx], false, app);
    } else if let Some(pane) = ws_state.active_pane() {
        draw_single_pane(frame, area, pane, true, app);
    }
}

fn draw_single_pane(
    frame: &mut Frame,
    area: Rect,
    pane: &HarnessPane,
    is_active: bool,
    app: &AppState,
) {
    let theme = &app.theme;
    let border_color = if is_active {
        theme.accent
    } else {
        theme.border
    };

    let kind_badge = match pane.kind {
        PaneKind::Owned => "[OWNED INTERACTIVE]",
        PaneKind::Observed => "[OBSERVED READ-ONLY]",
    };

    let is_ascii = app.read_model.effective_settings.tui.unicode_fallback || is_ascii_terminal();
    let badge = pane.status.badge_for(is_ascii);

    let title = format!(
        " {} {} | {} | {} ",
        pane.title,
        kind_badge,
        pane.workspace.display(),
        badge,
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(title);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height == 0 || inner.width == 0 {
        return;
    }

    let mut lines = Vec::new();

    // Top watermark line
    let watermark = match pane.kind {
        PaneKind::Owned => {
            let sym = if is_ascii { "[*] " } else { "● " };
            Span::styled(
                format!("{sym}Interactive terminal — Type to send input. Press Ctrl+B d to detach focus."),
                Style::default().fg(theme.accent2).add_modifier(Modifier::DIM),
            )
        }
        PaneKind::Observed => {
            let sym = if is_ascii { "[OBS] " } else { "👁 " };
            Span::styled(
                format!("{sym}Captured session — Read-only. Input is prohibited."),
                Style::default()
                    .fg(theme.error)
                    .add_modifier(Modifier::BOLD),
            )
        }
    };
    lines.push(Line::from(watermark));

    // Visible buffer lines
    let content_height = (inner.height as usize).saturating_sub(1);
    let visible = pane.buffer.visible_lines(content_height);
    lines.extend(visible);

    let p = Paragraph::new(lines);
    frame.render_widget(p, inner);
}

fn draw_pane_footer(frame: &mut Frame, area: Rect, app: &AppState) {
    let theme = &app.theme;
    let ws_state = &app.harnesses;

    let active_title = ws_state
        .active_pane()
        .map(|p| p.title.as_str())
        .unwrap_or("None");

    let scroll_info = ws_state
        .active_pane()
        .map(|p| {
            format!(
                "Scroll: {}/{}",
                p.buffer.scroll_offset,
                p.buffer.line_count()
            )
        })
        .unwrap_or_default();

    let text = vec![
        Span::styled(
            format!(" Active: {active_title} | {scroll_info} "),
            Style::default().fg(theme.accent2),
        ),
        Span::styled(
            " [c] Launch  [x] Close  [s] Split  [Tab] Switch  [Ctrl+B d] Detach ",
            Style::default().fg(theme.muted),
        ),
    ];

    let p = Paragraph::new(Line::from(text));
    frame.render_widget(p, area);
}
