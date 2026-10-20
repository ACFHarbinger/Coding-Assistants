//! How a harness is attended by a person.
//!
//! CLI and TUI harnesses are processes: a one-shot argv, or an interactive
//! program hosted in a PTY. A web-app harness is a local HTTP server embedded
//! in the desktop shell. DeepSeek's official `dsh --profile web` is the first
//! of that kind; it is not an OpenCode session and it is not a terminal.

use super::HarnessId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HarnessSurface {
    /// Headless argv. The process is given one prompt and then exits.
    Cli,
    /// Interactive terminal UI, resumed inside a PTY or a real terminal.
    Tui,
    /// Local web application. The shell embeds its URL; it is not a PTY.
    WebApp,
}

impl HarnessId {
    pub fn surface(self) -> HarnessSurface {
        match self {
            Self::Grok
            | Self::Chat
            | Self::Claude
            | Self::Gemini
            | Self::Vibe
            | Self::Muse
            | Self::Cursor
            | Self::Qwen
            | Self::Kimi
            | Self::Hermes => HarnessSurface::Tui,
            // OpenCode's wired path is `opencode run` (one shot). Its TUI is
            // not resumed by the embedded relaunch adapter.
            Self::OpenCode => HarnessSurface::Cli,
            Self::DeepSeek => HarnessSurface::WebApp,
        }
    }

    pub fn is_web_app(self) -> bool {
        self.surface() == HarnessSurface::WebApp
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deepseek_is_the_web_app_harness_and_opencode_stays_a_cli() {
        assert_eq!(HarnessId::DeepSeek.surface(), HarnessSurface::WebApp);
        assert!(HarnessId::DeepSeek.is_web_app());
        assert_eq!(HarnessId::OpenCode.surface(), HarnessSurface::Cli);
        assert_eq!(HarnessId::Claude.surface(), HarnessSurface::Tui);
        assert_eq!(HarnessId::Grok.surface(), HarnessSurface::Tui);
        assert!(!HarnessId::Claude.is_web_app());
    }
}
