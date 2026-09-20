//! Multi-agent task orchestration and its IPC-facing types.

mod budget;
mod memory_recall;
mod orchestrator;
mod periodic_consolidation;
mod prompt_builder;
mod suborch;
mod task_state;

pub use orchestrator::{AgentConfig, AgentEvent, AgentSystem};
pub use task_state::{
    is_valid_task_id, next_task_id, remove_task_mcp_dir, TaskHandles, TaskLifecycleEvent,
    TaskRegistry,
};
