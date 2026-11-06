//! Runtime budget gate (platform.md P10).
//!
//! Wraps C6 consume/pause/handoff into one decision a caller must honor
//! before starting a provider call. Affine typing remains postponed.
//! T1 (#341): Reserved and first-stop write stamped audit rows.

use super::super::*;
use super::slice_stamp::SliceStamp;
use serde_json::json;

impl HubStore {
    /// Decide whether `agent_id` may start a provider call of `amount` units.
    ///
    /// - No budget row → [`ProviderCallGate::Unmetered`].
    /// - Already paused → [`ProviderCallGate::Stopped`] without a new handoff.
    /// - Reservation succeeds → [`ProviderCallGate::Reserved`] + `budget.reserved`.
    /// - Reservation would exceed → pause, write the C6 Markdown summary +
    ///   delegation message, [`ProviderCallGate::Stopped`] with `handoff`
    ///   and `budget.stopped`.
    #[allow(clippy::too_many_arguments)]
    pub fn gate_provider_call(
        &self,
        agent_id: &str,
        amount: f64,
        stamp: &SliceStamp,
        objective: &str,
        completed: &str,
        missing: &str,
        delegate_to: Option<&str>,
    ) -> Result<ProviderCallGate, HubError> {
        if !amount.is_finite() || amount <= 0.0 {
            return Err(HubError::Invalid(
                "budget amount must be finite and > 0".into(),
            ));
        }
        let Some(existing) = self.get_budget(agent_id)? else {
            return Ok(ProviderCallGate::Unmetered);
        };
        if existing.paused {
            return Ok(ProviderCallGate::Stopped {
                status: existing,
                handoff: None,
            });
        }
        match self.try_consume_budget(agent_id, amount) {
            Ok(status) => {
                let _ = self.record_budget_gate_audit(
                    "budget.reserved",
                    agent_id,
                    amount,
                    &status,
                    stamp,
                );
                Ok(ProviderCallGate::Reserved { status })
            }
            Err(HubError::Invalid(_)) => {
                let outcome = self.pause_for_budget(
                    agent_id,
                    stamp.task_id.as_deref(),
                    objective,
                    completed,
                    missing,
                    delegate_to,
                )?;
                let _ = self.record_budget_gate_audit(
                    "budget.stopped",
                    agent_id,
                    amount,
                    &outcome.status,
                    stamp,
                );
                Ok(ProviderCallGate::Stopped {
                    status: outcome.status.clone(),
                    handoff: Some(outcome),
                })
            }
            Err(other) => Err(other),
        }
    }

    fn record_budget_gate_audit(
        &self,
        operation: &str,
        agent_id: &str,
        amount: f64,
        status: &BudgetStatus,
        stamp: &SliceStamp,
    ) -> Result<AuditEvent, HubError> {
        let mut stamp = stamp.clone();
        if stamp.agent.is_none() {
            stamp.agent = Some(agent_id.to_string());
        }
        let root = self.data_dir.clone();
        self.record_stamped_audit(
            &root,
            &root,
            operation,
            json!({
                "kind": "budget_gate",
                "agent": agent_id,
                "amount": amount,
                "spent_units": status.spent_units,
                "limit_units": status.limit_units,
                "paused": status.paused,
            }),
            &stamp,
        )
    }
}
