//! Stable Tauri-command facade for the durable shared hub.
//!
//! Commands are grouped by responsibility under `commands/`, with files
//! further organized by concern into `messager/` (chat + memory),
//! `hub/` (store bootstrap + workflow), `quota/`, `settings/` (settings +
//! roles), and `tests/` subdirectories. Module names (`commands::messaging`,
//! `commands::quotas`, etc.) are unchanged for callers — only the on-disk
//! file layout moved.

#[path = "messager/attachments.rs"]
pub mod attachments;
#[path = "hub/avatar.rs"]
pub mod avatar;
#[path = "messager/memory.rs"]
pub mod memory;
#[path = "messager/messaging.rs"]
pub mod messaging;
#[path = "quota/claude.rs"]
mod quota_claude;
#[path = "quota/codex.rs"]
mod quota_codex;
#[path = "quota/gemini.rs"]
mod quota_gemini;
#[path = "quota/grok.rs"]
mod quota_grok;
#[path = "quota/quotas.rs"]
pub mod quotas;
#[path = "settings/roles.rs"]
pub mod roles;
#[path = "settings/settings.rs"]
pub mod settings;
#[path = "hub/store.rs"]
pub mod store;
#[path = "hub/workflow.rs"]
pub mod workflow;

#[cfg(test)]
#[path = "tests/mod.rs"]
pub(crate) mod tests;
