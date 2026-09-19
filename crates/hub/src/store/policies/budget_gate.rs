//! Runtime budget gate (platform.md P10).
//!
//! Wraps C6 consume/pause/handoff into one decision a caller must honor
//! before starting a provider call. Affine typing remains postponed.

use super::super::*;

impl HubStore {
    /// Decide whether `agent_id` may start a provider call of `amount` units.
    ///
    /// - No budget row → [`ProviderCallGate::Unmetered`].
    /// - Already paused → [`ProviderCallGate::Stopped`] without a new handoff.
    /// - Reservation succeeds → [`ProviderCallGate::Reserved`].
    /// - Reservation would exceed → pause, write the C6 Markdown summary +
    ///   delegation message, [`ProviderCallGate::Stopped`] with `handoff`.
    #[allow(clippy::too_many_arguments)]
    pub fn gate_provider_call(
        &self,
        agent_id: &str,
        amount: f64,
        task_id: Option<&str>,
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
            Ok(status) => Ok(ProviderCallGate::Reserved { status }),
            Err(HubError::Invalid(_)) => {
                let outcome = self.pause_for_budget(
                    agent_id,
                    task_id,
                    objective,
                    completed,
                    missing,
                    delegate_to,
                )?;
                Ok(ProviderCallGate::Stopped {
                    status: outcome.status.clone(),
                    handoff: Some(outcome),
                })
            }
            Err(other) => Err(other),
        }
    }
}
