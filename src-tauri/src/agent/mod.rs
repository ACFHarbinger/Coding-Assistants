//! Multi-agent task orchestration and its IPC-facing types.

mod orchestrator;

pub use orchestrator::{AgentConfig, AgentEvent, AgentSystem};
