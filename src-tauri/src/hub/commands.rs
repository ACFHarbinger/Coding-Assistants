//! Stable Tauri-command facade for the durable shared hub.
//!
//! Commands are grouped by responsibility under `commands/`.

pub mod memory;
pub mod messaging;
mod quota_claude;
mod quota_codex;
mod quota_grok;
pub mod quotas;
pub mod store;
pub mod workflow;

#[cfg(test)]
mod tests;
