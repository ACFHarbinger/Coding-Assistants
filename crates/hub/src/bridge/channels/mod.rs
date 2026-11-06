//! Provider bridges built on a documented, opt-in "Channel"-style capability
//! (as opposed to `bridge::{claude,gemini,grok}`'s capture/inject adapters
//! over each provider's own session/process contract): Claude Code's MCP
//! `claude/channel` research preview, Gemini/Antigravity's kill-capture-
//! relaunch continuation bridge, Grok's leader-socket delivery, and Chat/
//! Codex's turn-completion reply capture.

pub mod chat;
pub mod claude;
pub mod gemini;
pub mod grok;
