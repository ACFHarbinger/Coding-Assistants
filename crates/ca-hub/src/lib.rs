//! Coding-Assistants shared memory, messaging, and wake primitives.
//!
//! Implements the M1/M2/M3/M4 and C1/C3 spine from
//! `docs/moon/roadmaps/memory.md` and `docs/moon/roadmaps/communication.md`.
//! See [`store::HubStore`] for the concrete SQLite + file-backed
//! implementation; this module just re-exports it as the crate's public API.

mod harness;
mod store;

pub use harness::{
    claude_spawn_args, codex_spawn_args, gemini_spawn_args, grok_spawn_args, inject_harness,
    start_harness, HarnessId, HarnessInjectRequest, HarnessInjectResult, HarnessStartRequest,
    HarnessStartResult,
};
pub use store::{
    parse_memory_references, AgentCard, AgentMetrics, AgentRecord, AuditEvent, BudgetPauseOutcome,
    BudgetStatus, CompactReport, GitExportOutcome, HubError, HubStore, MemoryRecord, MemoryScope,
    MemoryTier, MessageKind, MessageRecord, MessageStatus, SendOutcome, ShutdownOutcome,
    TaskRecord, TaskStatus, WakePolicy, WakeRecord, WakeStatus, WorkSessionRecord, WorkflowStep,
};
