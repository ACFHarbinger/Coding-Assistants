//! Team Scorecard projection module (Roadmap T3 / #343).
//!
//! Provides the `HubStore::get_scorecard` method projecting audit rows,
//! agent budgets, tasks, and harness registrations into three row kinds
//! (agent, harness, provider) with explicit figure provenance and visibility
//! gating.

mod digest;
#[cfg(test)]
#[path = "probe_test_support.rs"]
pub(crate) mod probe_test_support;

pub(crate) mod helpers;
mod probe_claimant;
mod probe_quota;
mod probes;
mod query;
mod review;
mod types;
pub use digest::*;
pub use review::{ReviewFinding, FINDING_ORIGINS, FINDING_SEVERITIES};
pub use types::*;
