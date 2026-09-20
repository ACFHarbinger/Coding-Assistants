//! Lightweight ANSI / VT parser and screen buffer for terminal rendering in Ratatui (T6).

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

const DEFAULT_MAX_LINES: usize = 3000;

/// A screen buffer that parses and accumulates ANSI/VT terminal output into styled Ratatui lines.
#[derive(Debug, Clone)]
pub struct VtScreenBuffer {
    lines: Vec<Line<'static>>,
    current_line_spans: Vec<Span<'static>>,
    current_span_text: String,
    current_style: Style,
    max_lines: usize,
    pub scroll_offset: usize,
}

impl Default for VtScreenBuffer {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_LINES)
    }
}

impl VtScreenBuffer {
    pub fn new(max_lines: usize) -> Self {
        Self {
            lines: Vec::new(),
            current_line_spans: Vec::new(),
            current_span_text: String::new(),
            current_style: Style::default(),
            max_lines,
            scroll_offset: 0,
        }
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
            + if self.current_span_text.is_empty() && self.current_line_spans.is_empty() {
                0
            } else {
                1
            }
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
            && self.current_line_spans.is_empty()
            && self.current_span_text.is_empty()
    }

    pub fn clear(&mut self) {
        self.lines.clear();
        self.current_line_spans.clear();
        self.current_span_text.clear();
        self.current_style = Style::default();
        self.scroll_offset = 0;
    }

    /// Feed a byte slice (e.g. from PTY read).
    pub fn feed_bytes(&mut self, bytes: &[u8]) {
        let text = String::from_utf8_lossy(bytes);
        self.feed_str(&text);
    }

    /// Feed a string chunk into the parser.
    pub fn feed_str(&mut self, text: &str) {
        let mut chars = text.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == '\x1b' {
                if chars.peek() == Some(&'[') {
                    chars.next(); // consume '['
                    let mut seq = String::new();
                    while let Some(&c) = chars.peek() {
                        if c.is_ascii_alphabetic() || c == '?' || c == '@' {
                            seq.push(c);
                            chars.next();
                            break;
                        } else {
                            seq.push(c);
                            chars.next();
                        }
                    }
                    self.handle_csi_sequence(&seq);
                } else if chars.peek() == Some(&']') {
                    // OSC sequence (e.g. \x1b]0;Title\x07)
                    chars.next(); // consume ']'
                    while let Some(c) = chars.next() {
                        if c == '\x07' || (c == '\x1b' && chars.peek() == Some(&'\\')) {
                            if c == '\x1b' {
                                chars.next();
                            }
                            break;
                        }
                    }
                }
            } else if ch == '\r' {
                if chars.peek() == Some(&'\n') {
                    chars.next(); // consume '\n'
                    self.flush_line();
                } else {
                    // Carriage return without newline: overwrite current line
                    self.flush_span();
                    self.current_line_spans.clear();
                }
            } else if ch == '\n' {
                self.flush_line();
            } else if ch == '\t' {
                self.current_span_text.push_str("    ");
            } else if !ch.is_control() {
                self.current_span_text.push(ch);
            }
        }
    }

    fn flush_span(&mut self) {
        if !self.current_span_text.is_empty() {
            let span = Span::styled(
                std::mem::take(&mut self.current_span_text),
                self.current_style,
            );
            self.current_line_spans.push(span);
        }
    }

    fn flush_line(&mut self) {
        self.flush_span();
        let spans = std::mem::take(&mut self.current_line_spans);
        self.lines.push(Line::from(spans));
        if self.lines.len() > self.max_lines {
            let overflow = self.lines.len() - self.max_lines;
            self.lines.drain(..overflow);
        }
        // Auto-scroll to bottom if offset was 0
        if self.scroll_offset > 0 {
            self.scroll_offset = self.scroll_offset.saturating_add(1);
        }
    }

    fn handle_csi_sequence(&mut self, seq: &str) {
        if let Some(code_str) = seq.strip_suffix('m') {
            // SGR code
            self.flush_span();
            if code_str.is_empty() || code_str == "0" {
                self.current_style = Style::default();
            } else {
                for part in code_str.split(';') {
                    if let Ok(num) = part.parse::<u8>() {
                        match num {
                            0 => self.current_style = Style::default(),
                            1 => {
                                self.current_style = self.current_style.add_modifier(Modifier::BOLD)
                            }
                            2 => {
                                self.current_style = self.current_style.add_modifier(Modifier::DIM)
                            }
                            3 => {
                                self.current_style =
                                    self.current_style.add_modifier(Modifier::ITALIC)
                            }
                            4 => {
                                self.current_style =
                                    self.current_style.add_modifier(Modifier::UNDERLINED)
                            }
                            7 => {
                                self.current_style =
                                    self.current_style.add_modifier(Modifier::REVERSED)
                            }
                            22 => {
                                self.current_style = self
                                    .current_style
                                    .remove_modifier(Modifier::BOLD | Modifier::DIM)
                            }
                            23 => {
                                self.current_style =
                                    self.current_style.remove_modifier(Modifier::ITALIC)
                            }
                            24 => {
                                self.current_style =
                                    self.current_style.remove_modifier(Modifier::UNDERLINED)
                            }
                            27 => {
                                self.current_style =
                                    self.current_style.remove_modifier(Modifier::REVERSED)
                            }
                            30 => self.current_style = self.current_style.fg(Color::Black),
                            31 => self.current_style = self.current_style.fg(Color::Red),
                            32 => self.current_style = self.current_style.fg(Color::Green),
                            33 => self.current_style = self.current_style.fg(Color::Yellow),
                            34 => self.current_style = self.current_style.fg(Color::Blue),
                            35 => self.current_style = self.current_style.fg(Color::Magenta),
                            36 => self.current_style = self.current_style.fg(Color::Cyan),
                            37 => self.current_style = self.current_style.fg(Color::Gray),
                            39 => self.current_style = self.current_style.fg(Color::Reset),
                            40 => self.current_style = self.current_style.bg(Color::Black),
                            41 => self.current_style = self.current_style.bg(Color::Red),
                            42 => self.current_style = self.current_style.bg(Color::Green),
                            43 => self.current_style = self.current_style.bg(Color::Yellow),
                            44 => self.current_style = self.current_style.bg(Color::Blue),
                            45 => self.current_style = self.current_style.bg(Color::Magenta),
                            46 => self.current_style = self.current_style.bg(Color::Cyan),
                            47 => self.current_style = self.current_style.bg(Color::Gray),
                            49 => self.current_style = self.current_style.bg(Color::Reset),
                            90 => self.current_style = self.current_style.fg(Color::DarkGray),
                            91 => self.current_style = self.current_style.fg(Color::LightRed),
                            92 => self.current_style = self.current_style.fg(Color::LightGreen),
                            93 => self.current_style = self.current_style.fg(Color::LightYellow),
                            94 => self.current_style = self.current_style.fg(Color::LightBlue),
                            95 => self.current_style = self.current_style.fg(Color::LightMagenta),
                            96 => self.current_style = self.current_style.fg(Color::LightCyan),
                            97 => self.current_style = self.current_style.fg(Color::White),
                            _ => {}
                        }
                    }
                }
            }
        } else if seq == "2K" || seq == "K" {
            // Clear line
            self.current_span_text.clear();
            self.current_line_spans.clear();
        }
    }

    /// Retrieve all assembled lines including any in-progress line.
    pub fn all_lines(&self) -> Vec<Line<'static>> {
        let mut result = self.lines.clone();
        if !self.current_line_spans.is_empty() || !self.current_span_text.is_empty() {
            let mut spans = self.current_line_spans.clone();
            if !self.current_span_text.is_empty() {
                spans.push(Span::styled(
                    self.current_span_text.clone(),
                    self.current_style,
                ));
            }
            result.push(Line::from(spans));
        }
        result
    }

    /// Render visible lines window for a given viewport height.
    pub fn visible_lines(&self, height: usize) -> Vec<Line<'static>> {
        let all = self.all_lines();
        if all.is_empty() {
            return Vec::new();
        }
        let total = all.len();
        if total <= height {
            return all;
        }

        let max_scroll = total.saturating_sub(height);
        let effective_scroll = self.scroll_offset.min(max_scroll);
        let end_idx = total.saturating_sub(effective_scroll);
        let start_idx = end_idx.saturating_sub(height);

        all[start_idx..end_idx].to_vec()
    }

    pub fn scroll_up(&mut self, n: usize) {
        let total = self.line_count();
        self.scroll_offset = (self.scroll_offset + n).min(total.saturating_sub(1));
    }

    pub fn scroll_down(&mut self, n: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(n);
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = 0;
    }

    /// Plaintext representation for assertions and testing.
    pub fn as_raw_text(&self) -> String {
        let mut out = String::new();
        for line in self.all_lines() {
            for span in line.spans {
                out.push_str(&span.content);
            }
            out.push('\n');
        }
        out
    }
}
