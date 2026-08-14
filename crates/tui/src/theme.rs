//! Color themes for the Ratatui TUI client. Session-local (cycled with `T`
//! or the `theme` command palette entry), not persisted to Settings — this
//! is a display preference, not orchestration state.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeName {
    Grok,
    Dracula,
    Solarized,
    Dark,
}

impl ThemeName {
    pub const ALL: [ThemeName; 4] = [
        ThemeName::Grok,
        ThemeName::Dracula,
        ThemeName::Solarized,
        ThemeName::Dark,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ThemeName::Grok => "Grok",
            ThemeName::Dracula => "Dracula",
            ThemeName::Solarized => "Solarized",
            ThemeName::Dark => "Dark (classic)",
        }
    }

    pub fn from_label(label: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|theme| theme.label().eq_ignore_ascii_case(label))
    }

    pub fn next(self) -> Self {
        let idx = Self::ALL.iter().position(|t| *t == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    pub fn prev(self) -> Self {
        let idx = Self::ALL.iter().position(|t| *t == self).unwrap_or(0);
        Self::ALL[(idx + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

/// A resolved palette. Every field is a concrete color so draw code never
/// hardcodes `Color::X` directly — swapping `ThemeName` swaps the whole UI.
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub name: ThemeName,
    /// Primary brand accent — header icon, active tab underline/text, focused borders.
    pub accent: Color,
    /// Secondary accent — section headings, secondary emphasis.
    pub accent2: Color,
    pub fg: Color,
    pub muted: Color,
    pub border: Color,
    pub border_focus: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub tab_inactive_fg: Color,
}

impl Theme {
    pub fn from_name(name: ThemeName) -> Self {
        match name {
            // xAI Grok's product surfaces lean near-black with a single hot
            // accent rather than a rainbow of ANSI colors — bright cyan
            // pulled toward electric blue, with a warm amber for secondary
            // emphasis so warnings/highlights don't fight the primary accent.
            ThemeName::Grok => Theme {
                name,
                accent: Color::Rgb(0x2b, 0xd9, 0xd9),
                accent2: Color::Rgb(0xff, 0xb4, 0x4c),
                fg: Color::Rgb(0xe8, 0xe8, 0xec),
                muted: Color::Rgb(0x7a, 0x7a, 0x88),
                border: Color::Rgb(0x33, 0x33, 0x3d),
                border_focus: Color::Rgb(0x2b, 0xd9, 0xd9),
                success: Color::Rgb(0x4c, 0xd9, 0x7a),
                warning: Color::Rgb(0xff, 0xb4, 0x4c),
                error: Color::Rgb(0xff, 0x5c, 0x5c),
                tab_inactive_fg: Color::Rgb(0x9a, 0x9a, 0xa8),
            },
            ThemeName::Dracula => Theme {
                name,
                accent: Color::Rgb(0xbd, 0x93, 0xf9),
                accent2: Color::Rgb(0xff, 0x79, 0xc6),
                fg: Color::Rgb(0xf8, 0xf8, 0xf2),
                muted: Color::Rgb(0x62, 0x72, 0xa4),
                border: Color::Rgb(0x44, 0x47, 0x5a),
                border_focus: Color::Rgb(0xbd, 0x93, 0xf9),
                success: Color::Rgb(0x50, 0xfa, 0x7b),
                warning: Color::Rgb(0xf1, 0xfa, 0x8c),
                error: Color::Rgb(0xff, 0x55, 0x55),
                tab_inactive_fg: Color::Rgb(0x8b, 0xe9, 0xfd),
            },
            ThemeName::Solarized => Theme {
                name,
                accent: Color::Rgb(0x26, 0x8b, 0xd2),
                accent2: Color::Rgb(0xb5, 0x89, 0x00),
                fg: Color::Rgb(0x93, 0xa1, 0xa1),
                muted: Color::Rgb(0x58, 0x6e, 0x75),
                border: Color::Rgb(0x07, 0x36, 0x42),
                border_focus: Color::Rgb(0x26, 0x8b, 0xd2),
                success: Color::Rgb(0x85, 0x99, 0x00),
                warning: Color::Rgb(0xcb, 0x4b, 0x16),
                error: Color::Rgb(0xdc, 0x32, 0x2f),
                tab_inactive_fg: Color::Rgb(0x65, 0x7b, 0x83),
            },
            // The original hardcoded palette, kept as an explicit choice
            // rather than deleted outright.
            ThemeName::Dark => Theme {
                name,
                accent: Color::Cyan,
                accent2: Color::Yellow,
                fg: Color::White,
                muted: Color::DarkGray,
                border: Color::DarkGray,
                border_focus: Color::Cyan,
                success: Color::Green,
                warning: Color::Yellow,
                error: Color::Red,
                tab_inactive_fg: Color::White,
            },
        }
    }
}

/// A small gradient pixel-art mountain, in the spirit of the idle splash
/// shown by other agent CLIs (Antigravity/Gemini, Claude Code, Grok Build)
/// before any chat activity — filling otherwise-empty view space rather
/// than leaving it blank. Two half-width block columns per "pixel" keep the
/// shape roughly square despite terminal cells being roughly twice as tall
/// as wide. Colors interpolate across the theme's accent pair; themes whose
/// accents aren't `Rgb` (nothing here currently) fall back to a flat accent.
///
/// `phase` (any f32, typically driven by an incrementing frame counter and
/// wrapped into `0.0..1.0`) shifts where the accent/accent2 gradient starts,
/// giving the pyramid a slow animated color sweep — pass `0.0` for a static
/// render.
pub fn logo_lines(theme: &Theme, phase: f32) -> Vec<Line<'static>> {
    const ROW_WIDTHS: [usize; 6] = [1, 3, 5, 7, 9, 11];
    let max_width = *ROW_WIDTHS.last().unwrap();
    let phase = phase.rem_euclid(1.0);
    ROW_WIDTHS
        .iter()
        .map(|&width| {
            let pad = (max_width - width) / 2;
            let mut spans = vec![Span::raw(" ".repeat(pad * 2))];
            for col in 0..width {
                let base_t = if width > 1 {
                    col as f32 / (width - 1) as f32
                } else {
                    0.5
                };
                // A triangle wave from `base_t` folded through `phase` keeps
                // the sweep bouncing between the two accents instead of
                // hard-cutting back to the start every wrap.
                let t = (base_t + phase).rem_euclid(2.0);
                let t = if t > 1.0 { 2.0 - t } else { t };
                let color = lerp_color(theme.accent, theme.accent2, t);
                spans.push(Span::styled("██", Style::default().fg(color)));
            }
            Line::from(spans)
        })
        .collect()
}

/// Classic Braille dot-spinner frames — the same family of glyph Grok
/// Build's idle spinner and most CLI progress indicators use.
pub const SPINNER_FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

pub fn spinner_frame(tick: u64) -> &'static str {
    SPINNER_FRAMES[(tick as usize) % SPINNER_FRAMES.len()]
}

/// The wordmark + tagline shown under the logo pyramid.
pub fn wordmark_lines(theme: &Theme, tagline: &str) -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(Span::styled(
            "CODING-ASSISTANTS",
            Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            tagline.to_string(),
            Style::default().fg(theme.muted),
        )),
    ]
}

fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    match (a, b) {
        (Color::Rgb(ar, ag, ab), Color::Rgb(br, bg, bb)) => {
            Color::Rgb(lerp_u8(ar, br, t), lerp_u8(ag, bg, t), lerp_u8(ab, bb, t))
        }
        _ => a,
    }
}

fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t).round() as u8
}

/// Interpolates between `theme.muted` and `theme.accent` by `t ∈ [0, 1]`.
/// Used by ambient fill elements to color dots and sparkline bars in proportion
/// to a pulse phase — 0.0 gives muted, 1.0 gives full accent.  Falls back to
/// accent for non-Rgb themes (the existing `lerp_color` fall-back).
pub fn lerp_accent(theme: &Theme, t: f32) -> ratatui::style::Color {
    lerp_color(theme.muted, theme.accent, t)
}

/// Buckets a slice of RFC3339 timestamps into `n_buckets` equal time-slices
/// spanning the actual oldest-to-newest range in `timestamps`, and returns
/// bar heights in `0..=7`, ready for block-character sparkline rendering —
/// bucket 0 is the oldest slice, the last bucket is the most recent.
///
/// Unparsable entries are skipped rather than distorting the range. When
/// every entry is unparsable, `n_buckets` is 0, or `timestamps` is empty,
/// returns an all-zero vec of length `n_buckets`. When every parsed
/// timestamp falls at the same instant (or only one remains), every count
/// lands in the last (most recent) bucket rather than being spread by an
/// arbitrary rule.
pub fn task_sparkline_buckets(timestamps: &[&str], n_buckets: usize) -> Vec<u8> {
    use chrono::{DateTime, Utc};

    if n_buckets == 0 {
        return vec![];
    }
    let mut counts = vec![0u32; n_buckets];
    let parsed: Vec<DateTime<Utc>> = timestamps
        .iter()
        .filter_map(|ts| DateTime::parse_from_rfc3339(ts).ok())
        .map(|dt| dt.with_timezone(&Utc))
        .collect();
    if parsed.is_empty() {
        return counts.iter().map(|&c| c as u8).collect();
    }
    let min = *parsed.iter().min().unwrap();
    let max = *parsed.iter().max().unwrap();
    let span = (max - min).num_milliseconds().max(0) as f64;
    for dt in &parsed {
        let bucket = if span == 0.0 {
            n_buckets - 1
        } else {
            let elapsed = (*dt - min).num_milliseconds().max(0) as f64;
            (((elapsed / span) * n_buckets as f64) as usize).min(n_buckets - 1)
        };
        counts[bucket] = counts[bucket].saturating_add(1);
    }
    let peak = counts.iter().copied().max().unwrap_or(1).max(1);
    counts
        .iter()
        .map(|&c| (c * 7).div_ceil(peak) as u8)
        .collect()
}

/// Renders `task_sparkline_buckets`' `0..=7` bar heights as a string of
/// Unicode block characters (▁▂▃▄▅▆▇█), one per bucket.
pub fn sparkline_string(buckets: &[u8]) -> String {
    const BAR_CHARS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    buckets
        .iter()
        .map(|&h| BAR_CHARS[h.min(7) as usize])
        .collect()
}

/// Maps agent index `i` and the global `tick` counter to a pulse brightness
/// in `0.0..=1.0` using a slow sine-like triangle wave.  Each agent is offset
/// by `i * 23` ticks so the presence dots pulse in a staggered cascade rather
/// than all together.
///
/// Returns a value in `[0.0, 1.0]` where 1.0 is full brightness.
pub fn agent_pulse_phase(i: usize, tick: u64) -> f32 {
    let offset = (i as u64).wrapping_mul(23);
    let t = tick.wrapping_add(offset) % 60;
    // Triangle wave: 0 → 1 over 30 ticks, 1 → 0 over next 30
    if t < 30 {
        t as f32 / 30.0
    } else {
        (60 - t) as f32 / 30.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_and_prev_cycle_through_every_theme_and_wrap() {
        let mut name = ThemeName::Grok;
        for _ in 0..ThemeName::ALL.len() {
            name = name.next();
        }
        assert_eq!(name, ThemeName::Grok, "next() must wrap back to the start");

        let mut name = ThemeName::Grok;
        for _ in 0..ThemeName::ALL.len() {
            name = name.prev();
        }
        assert_eq!(name, ThemeName::Grok, "prev() must wrap back to the start");

        assert_eq!(ThemeName::Grok.next(), ThemeName::Dracula);
        assert_eq!(ThemeName::Grok.prev(), ThemeName::Dark);
    }

    #[test]
    fn from_label_is_case_insensitive_and_rejects_unknown_names() {
        assert_eq!(ThemeName::from_label("grok"), Some(ThemeName::Grok));
        assert_eq!(ThemeName::from_label("DRACULA"), Some(ThemeName::Dracula));
        assert_eq!(ThemeName::from_label("nonexistent"), None);
    }

    #[test]
    fn every_theme_name_resolves_to_a_palette() {
        for name in ThemeName::ALL {
            let theme = Theme::from_name(name);
            assert_eq!(theme.name, name);
        }
    }

    #[test]
    fn logo_lines_form_a_widening_then_symmetric_pyramid() {
        let theme = Theme::from_name(ThemeName::Grok);
        let lines = logo_lines(&theme, 0.0);
        assert_eq!(lines.len(), 6, "one line per pyramid row");
        // Each row's rendered width (padding + 2 cols per pixel) must match
        // every other row's, so the pyramid doesn't jaggedly shift the
        // right edge as it widens.
        let widths: Vec<usize> = lines
            .iter()
            .map(|line| line.spans.iter().map(|s| s.content.chars().count()).sum())
            .collect();
        assert!(widths.windows(2).all(|w| w[0] == w[1]));
    }

    #[test]
    fn lerp_color_interpolates_rgb_and_falls_back_for_non_rgb() {
        let a = Color::Rgb(0, 0, 0);
        let b = Color::Rgb(100, 200, 40);
        assert_eq!(lerp_color(a, b, 0.0), a);
        assert_eq!(lerp_color(a, b, 1.0), b);
        assert_eq!(lerp_color(Color::Cyan, Color::Yellow, 0.5), Color::Cyan);
    }

    #[test]
    fn spinner_frame_wraps_around_the_frame_table() {
        let len = SPINNER_FRAMES.len() as u64;
        assert_eq!(spinner_frame(0), spinner_frame(len));
        assert_eq!(spinner_frame(1), spinner_frame(len + 1));
    }

    #[test]
    fn sparkline_empty_input_yields_all_zeros() {
        let buckets = task_sparkline_buckets(&[], 8);
        assert_eq!(buckets.len(), 8);
        assert!(buckets.iter().all(|&b| b == 0));
    }

    #[test]
    fn sparkline_zero_buckets_returns_empty() {
        assert_eq!(
            task_sparkline_buckets(&["2026-01-01T12:00:00Z"], 0),
            Vec::<u8>::new()
        );
    }

    #[test]
    fn sparkline_all_unparsable_yields_all_zeros() {
        let buckets = task_sparkline_buckets(&["not-a-timestamp", "also-bad"], 8);
        assert_eq!(buckets.len(), 8);
        assert!(buckets.iter().all(|&b| b == 0));
    }

    #[test]
    fn sparkline_identical_timestamps_land_in_the_last_bucket() {
        // No real span to distribute across — everything is "now", so it
        // must all land in the most-recent bucket rather than an arbitrary one.
        let ts = ["2026-01-01T00:00:00Z"; 5];
        let buckets = task_sparkline_buckets(&ts, 4);
        assert_eq!(buckets.len(), 4);
        assert_eq!(buckets[3], 7);
        assert!(buckets[..3].iter().all(|&b| b == 0));
    }

    #[test]
    fn sparkline_orders_oldest_to_newest_across_the_real_range() {
        // Three events spread evenly across a 100ms span with 4 buckets:
        // the earliest must land at or before the middle bucket and the
        // latest must land in the final bucket — this is the property the
        // old "last two digits" heuristic could not guarantee at all.
        let ts = [
            "2026-01-01T00:00:00.000Z",
            "2026-01-01T00:00:00.050Z",
            "2026-01-01T00:00:00.099Z",
        ];
        let buckets = task_sparkline_buckets(&ts, 4);
        assert_eq!(buckets.len(), 4);
        // A non-trivial spread across buckets, not everything crushed into one.
        assert!(buckets.iter().filter(|&&b| b > 0).count() >= 2);
        assert!(
            buckets[3] > 0,
            "the most recent event must reach the last bucket"
        );
    }

    #[test]
    fn sparkline_output_length_matches_n_buckets() {
        let ts = ["2026-01-01T12:34:56Z", "2026-06-15T08:00:01Z"];
        for n in [1, 4, 8, 16] {
            assert_eq!(task_sparkline_buckets(&ts, n).len(), n);
        }
    }

    #[test]
    fn agent_pulse_phase_stays_in_zero_to_one() {
        for i in 0..8 {
            for tick in [0u64, 1, 15, 29, 30, 45, 59, 60, 1000] {
                let p = agent_pulse_phase(i, tick);
                assert!(
                    (0.0..=1.0).contains(&p),
                    "pulse {p} out of range at i={i} tick={tick}"
                );
            }
        }
    }

    #[test]
    fn agent_pulse_phase_staggers_adjacent_agents() {
        // Two agents at the same tick should not always have the same phase.
        let p0 = agent_pulse_phase(0, 0);
        let p1 = agent_pulse_phase(1, 0);
        assert_ne!(p0, p1, "adjacent agents must start at different phases");
    }
}
