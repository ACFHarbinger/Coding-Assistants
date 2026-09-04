//! Multi-agent task orchestration and its IPC-facing types.

mod memory_recall;
mod orchestrator;
mod prompt_builder;

pub use orchestrator::{AgentConfig, AgentEvent, AgentSystem};
