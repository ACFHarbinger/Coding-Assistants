//! Coding-Assistants shared memory, messaging, and wake primitives.
//!
//! Implements the M1/M2/M3/M4 and C1/C3 spine from
//! `docs/moon/roadmaps/memory.md` and `docs/moon/roadmaps/communication.md`.
//! See [`store::HubStore`] for the concrete SQLite + file-backed
//! implementation; this module just re-exports it as the crate's public API.

mod store;

pub use store::{
    parse_memory_references, AgentCard, AgentMetrics, AgentRecord, AuditEvent, BudgetPauseOutcome,
    BudgetStatus, CompactReport, GitExportOutcome, HubError, HubStore, MemoryRecord, MemoryScope,
    MemoryTier, MessageKind, MessageRecord, MessageStatus, ShutdownOutcome, TaskRecord, TaskStatus,
    WakePolicy, WakeRecord, WakeStatus, WorkflowStep,
};
