//! Scheduled Goals (`roadmaps/schedule.md`, epic #350).
//!
//! Split for the 500-LoC cap: [types] (pure data), [schema] (DDL), [crud]
//! (plain CRUD), `gate` + `gate_subject` (S2, OpenCode #352; P20 #367),
//! `tick`/`dispatch` (S3, #351).

mod types;
pub use types::*;

mod crud;
mod dispatch;
mod gate;
mod gate_contract;
mod gate_proposal;
mod gate_subject;
mod globset;
mod schema;
mod tick;
mod time;
pub use gate::{CriterionOutcome, GateCheckResult};
pub use gate_contract::PendingGateEntry;
pub use gate_subject::GateSubject;
pub use tick::{TickOutcome, TickSummary};
