//! Multi-agent task orchestration and its IPC-facing types.

mod budget;
mod memory_recall;
mod orchestrator;
mod periodic_consolidation;
mod prompt_builder;

pub use orchestrator::{AgentConfig, AgentEvent, AgentSystem};
