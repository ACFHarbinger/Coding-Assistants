//! Shared #351/#352 gate-result contract.
//!
//! OpenCode owns `gate.rs` / `globset.rs` and must not be edited in the
//! same turn as Cursor's tick/crud/schema work. Cursor does not call into
//! P5 resume logic here.
//!
//! OpenCode should extend the existing `GateCheckResult` /
//! `CriterionOutcome` (do not invent a parallel type) as:
//!
//! ```text
//! GateCheckResult { passed, pending, outcomes }
//! CriterionOutcome { passed, pending, detail, audit_id }
//! ```
//!
//! Semantics tick will honor once those fields exist:
//! - `pending == true` → action `waiting`, do not consume `max_attempts`,
//!   do not treat the criterion as failed, do not re-prompt.
//! - `audit_id` is the `os_tool.proposed` row to resume; persist it in
//!   `scheduled_tasks.pending_gate_json` (column added by #351).
//! - Approving that proposal must supply the gate result; do not submit a
//!   second proposal for the same criterion/attempt.
//!
//! `pending_gate_json` entries:
//! `[{"index":0,"audit_id":"<os_tool audit id>","status":"proposed"}]`
//!
//! `MinTokensWritten`: pass only when every *eligible* target identity has
//! a measured delta `>= min_tokens`. Eligible = the Agent id, or standing
//! roster minus `system`/`human`. Absent metrics, missing baseline keys,
//! and an empty eligible set are unknown/fail — never a vacuous pass.

use serde::{Deserialize, Serialize};

/// One outstanding P5 proposal that #352's evaluator should resume.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingGateEntry {
    pub index: usize,
    pub audit_id: String,
    pub status: String,
}
