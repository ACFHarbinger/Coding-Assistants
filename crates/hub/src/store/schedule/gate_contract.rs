//! Shared #351/#352 gate-result contract.
//!
//! Implemented `GateCheckResult` / `CriterionOutcome` shapes:
//!
//! ```text
//! GateCheckResult { passed, pending, outcomes }
//! CriterionOutcome { passed, pending, detail, audit_id }
//! ```
//!
//! Semantics honored by the shared tick engine:
//! - `pending == true` → action `waiting`, do not consume `max_attempts`,
//!   do not treat the criterion as failed, do not re-prompt.
//! - `audit_id` is the `os_tool.proposed` row to resume. Persistence uses
//!   hash-chained `process_json.schedule_gate` links (see `gate_proposal`),
//!   keyed by task/criterion/attempt(/stage for workflow branches),
//!   not `pending_gate_json`.
//! - Approving that proposal must supply the gate result; do not submit a
//!   second proposal for the same criterion/attempt.
//!
//! Legacy `pending_gate_json` entries (column/type retained for compatibility;
//! unused by the evaluator, not a second source of approval authority):
//! `[{"index":0,"audit_id":"<os_tool audit id>","status":"proposed"}]`
//!
//! `MinTokensWritten`: pass only when every *eligible* target identity has
//! a measured delta `>= min_tokens`. Eligible = the Agent id, or standing
//! roster minus `system`/`human`. Absent metrics, missing baseline keys,
//! and an empty eligible set are unknown/fail — never a vacuous pass.

use serde::{Deserialize, Serialize};

/// Legacy approval-link DTO retained for compatibility. The evaluator
/// resumes hash-chained audit links, not values of this type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingGateEntry {
    pub index: usize,
    pub audit_id: String,
    pub status: String,
}
