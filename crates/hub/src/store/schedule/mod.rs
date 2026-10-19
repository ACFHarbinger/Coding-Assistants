//! Scheduled Goals (`roadmaps/schedule.md`, epic #350).
//!
//! Split for the 500-LoC cap: [types] (pure data), [schema] (DDL), [crud]
//! (plain CRUD), `gate` (S2, OpenCode #352), `tick`/`dispatch` (S3, #351).

mod types;
pub use types::*;

mod crud;
mod dispatch;
mod gate;
mod gate_contract;
mod gate_proposal;
mod globset;
mod schema;
mod tick;
mod time;
pub use gate::{CriterionOutcome, GateCheckResult};
pub use gate_contract::PendingGateEntry;
pub use tick::{TickOutcome, TickSummary};
