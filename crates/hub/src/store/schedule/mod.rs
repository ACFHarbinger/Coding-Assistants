//! Scheduled Goals (`roadmaps/schedule.md`, epic #350).
//!
//! Split for the 500-LoC cap: [types] (pure data), [schema] (DDL), [crud]
//! (plain CRUD), `gate` (S2, criteria evaluation), `tick` (S3, the
//! dispatch/response/re-prompt state machine).

mod types;
pub use types::*;

mod crud;
mod schema;
