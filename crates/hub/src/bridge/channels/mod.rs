//! Provider bridges built on a documented, opt-in "Channel"-style capability
//! (as opposed to `bridge::{claude,codex,gemini,grok}`'s capture/inject
//! adapters over each provider's own session/process contract). Today this
//! is Claude Code's MCP `claude/channel` research preview only.

pub mod claude;
pub mod grok;
