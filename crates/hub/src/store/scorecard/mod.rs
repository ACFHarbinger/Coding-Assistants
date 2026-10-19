//! Team Scorecard projection module (Roadmap T3 / #343).
//!
//! Provides the `HubStore::get_scorecard` method projecting audit rows,
//! agent budgets, tasks, and harness registrations into three row kinds
//! (agent, harness, provider) with explicit figure provenance and visibility
//! gating.

mod helpers;
mod query;
mod types;
pub use types::*;
