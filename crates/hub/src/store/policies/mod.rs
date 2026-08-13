use super::*;

mod audit;
mod settings_audit;
impl HubStore {
    pub fn get_wake_policy(&self) -> Result<WakePolicy, HubError> {
        let raw: Option<String> = self
            .conn
            .query_row(
                "SELECT value FROM meta WHERE key = 'wake_policy'",
                [],
                |r| r.get(0),
            )
            .optional()?;
        match raw {
            Some(s) => serde_json::from_str(&s)
                .map_err(|e| HubError::Invalid(format!("wake_policy JSON corrupt: {e}"))),
            None => Ok(WakePolicy::default()),
        }
    }

    pub fn set_wake_policy(&self, policy: &WakePolicy) -> Result<(), HubError> {
        let json = serde_json::to_string(policy)
            .map_err(|e| HubError::Invalid(format!("wake_policy serialize: {e}")))?;
        self.conn.execute(
            r#"
            INSERT INTO meta(key, value) VALUES ('wake_policy', ?1)
            ON CONFLICT(key) DO UPDATE SET value = excluded.value
            "#,
            params![json],
        )?;
        Ok(())
    }

    /// Set (or reset) an agent's spend budget (C6). Resets `spent_units` to 0
    /// and clears any prior pause.
    pub fn set_agent_budget(
        &self,
        agent_id: &str,
        limit_units: f64,
    ) -> Result<BudgetStatus, HubError> {
        if limit_units <= 0.0 {
            return Err(HubError::Invalid("limit_units must be > 0".into()));
        }
        self.upsert_agent(agent_id, agent_id)?;
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            r#"
            INSERT INTO agent_budgets(agent_id, limit_units, spent_units, paused, updated_at)
            VALUES (?1, ?2, 0, 0, ?3)
            ON CONFLICT(agent_id) DO UPDATE SET
                limit_units = excluded.limit_units,
                spent_units = 0,
                paused = 0,
                updated_at = excluded.updated_at
            "#,
            params![agent_id, limit_units, now],
        )?;
        Ok(self.get_budget(agent_id)?.expect("just inserted"))
    }

    pub fn get_budget(&self, agent_id: &str) -> Result<Option<BudgetStatus>, HubError> {
        self.conn
            .query_row(
                "SELECT agent_id, limit_units, spent_units, paused, updated_at \
                 FROM agent_budgets WHERE agent_id = ?1",
                params![agent_id],
                |r| {
                    Ok(BudgetStatus {
                        agent_id: r.get(0)?,
                        limit_units: r.get(1)?,
                        spent_units: r.get(2)?,
                        paused: r.get::<_, i64>(3)? != 0,
                        updated_at: r.get(4)?,
                    })
                },
            )
            .optional()
            .map_err(HubError::from)
    }

    /// All configured agent budgets. Read-only view for Settings' typed
    /// command surface (S5 / #131) — budget storage stays here, in the
    /// table every C6 budget flow already reads and writes.
    pub fn list_agent_budgets(&self) -> Result<Vec<BudgetStatus>, HubError> {
        let mut stmt = self.conn.prepare(
            "SELECT agent_id, limit_units, spent_units, paused, updated_at \
             FROM agent_budgets ORDER BY agent_id",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(BudgetStatus {
                agent_id: r.get(0)?,
                limit_units: r.get(1)?,
                spent_units: r.get(2)?,
                paused: r.get::<_, i64>(3)? != 0,
                updated_at: r.get(4)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(HubError::from)
    }

    pub fn list_agent_metrics(&self) -> Result<Vec<AgentMetrics>, HubError> {
        let mut stmt = self.conn.prepare(
            "SELECT agent_id, lines_written, tokens_used, tokens_cached, provider_calls, output_chars, updated_at
             FROM agent_metrics ORDER BY agent_id",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(AgentMetrics {
                agent_id: r.get(0)?,
                lines_written: r.get(1)?,
                tokens_used: r.get(2)?,
                tokens_cached: r.get(3)?,
                provider_calls: r.get(4)?,
                output_chars: r.get(5)?,
                updated_at: r.get(6)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(HubError::from)
    }

    pub fn record_agent_metrics(
        &self,
        agent_id: &str,
        lines_written: i64,
        tokens_used: i64,
        tokens_cached: i64,
        output_chars: i64,
    ) -> Result<AgentMetrics, HubError> {
        if [lines_written, tokens_used, tokens_cached, output_chars]
            .iter()
            .any(|value| *value < 0)
        {
            return Err(HubError::Invalid(
                "metric increments must be non-negative".into(),
            ));
        }
        self.upsert_agent(agent_id, agent_id)?;
        self.conn.execute(
            "INSERT INTO agent_metrics(agent_id, lines_written, tokens_used, tokens_cached, provider_calls, output_chars, updated_at)
             VALUES (?1, ?2, ?3, ?4, 1, ?5, ?6)
             ON CONFLICT(agent_id) DO UPDATE SET
               lines_written = lines_written + excluded.lines_written,
               tokens_used = tokens_used + excluded.tokens_used,
               tokens_cached = tokens_cached + excluded.tokens_cached,
               provider_calls = provider_calls + 1,
               output_chars = output_chars + excluded.output_chars,
               updated_at = excluded.updated_at",
            params![agent_id, lines_written, tokens_used, tokens_cached, output_chars, Utc::now().to_rfc3339()],
        )?;
        self.list_agent_metrics()?
            .into_iter()
            .find(|metric| metric.agent_id == agent_id)
            .ok_or_else(|| HubError::NotFound(agent_id.into()))
    }

    /// Record `amount` units of spend against `agent_id`. Returns the updated
    /// status; `paused` flips to true once `spent_units >= limit_units`, but
    /// this call alone does **not** write a handoff — call `pause_for_budget`
    /// when the caller is ready to hand off and stop (C6).
    pub fn record_budget_usage(
        &self,
        agent_id: &str,
        amount: f64,
    ) -> Result<BudgetStatus, HubError> {
        let budget = self
            .get_budget(agent_id)?
            .ok_or_else(|| HubError::NotFound(format!("no budget set for {agent_id}")))?;
        let spent = budget.spent_units + amount;
        let paused = budget.paused || spent >= budget.limit_units;
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE agent_budgets SET spent_units = ?1, paused = ?2, updated_at = ?3 WHERE agent_id = ?4",
            params![spent, if paused { 1 } else { 0 }, now, agent_id],
        )?;
        Ok(self.get_budget(agent_id)?.expect("just updated"))
    }

    /// Atomically reserve budget before starting a provider call. Unlike
    /// `record_budget_usage`, this rejects a call that would exceed the limit.
    pub fn try_consume_budget(
        &self,
        agent_id: &str,
        amount: f64,
    ) -> Result<BudgetStatus, HubError> {
        if !amount.is_finite() || amount <= 0.0 {
            return Err(HubError::Invalid(
                "budget amount must be finite and > 0".into(),
            ));
        }
        let budget = self
            .get_budget(agent_id)?
            .ok_or_else(|| HubError::NotFound(format!("no budget set for {agent_id}")))?;
        if budget.paused {
            return Err(HubError::Invalid(format!("{agent_id} budget is paused")));
        }
        let next_spent = budget.spent_units + amount;
        if next_spent > budget.limit_units {
            let now = Utc::now().to_rfc3339();
            self.conn.execute(
                "UPDATE agent_budgets SET paused = 1, updated_at = ?1 WHERE agent_id = ?2",
                params![now, agent_id],
            )?;
            return Err(HubError::Invalid(format!(
                "budget exceeded for {agent_id}: {}/{} units",
                next_spent, budget.limit_units
            )));
        }
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE agent_budgets SET spent_units = ?1, paused = ?2, updated_at = ?3 WHERE agent_id = ?4",
            params![next_spent, if next_spent >= budget.limit_units { 1 } else { 0 }, now, agent_id],
        )?;
        self.get_budget(agent_id)?
            .ok_or_else(|| HubError::NotFound(agent_id.into()))
    }

    /// Clear a budget pause so the agent can receive wakes again (C6). A
    /// human/owner action, not something an agent should call on itself.
    pub fn resume_agent(&self, agent_id: &str) -> Result<BudgetStatus, HubError> {
        let n = self.conn.execute(
            "UPDATE agent_budgets SET paused = 0, updated_at = ?1 WHERE agent_id = ?2",
            params![Utc::now().to_rfc3339(), agent_id],
        )?;
        if n == 0 {
            return Err(HubError::NotFound(agent_id.into()));
        }
        Ok(self.get_budget(agent_id)?.expect("just updated"))
    }

    /// C6 exhaustion flow: mark `agent_id` paused (no further wakes accepted
    /// until `resume_agent`), write a durable Markdown handoff summary under
    /// `markdown/handoffs/`, and send a `Handoff` message to `delegate_to`
    /// (defaults to `"human"`) so the work is picked up rather than lost.
    #[allow(clippy::too_many_arguments)]
    pub fn pause_for_budget(
        &self,
        agent_id: &str,
        task_id: Option<&str>,
        objective: &str,
        completed: &str,
        missing: &str,
        delegate_to: Option<&str>,
    ) -> Result<BudgetPauseOutcome, HubError> {
        if self.get_budget(agent_id)?.is_none() {
            return Err(HubError::NotFound(format!("no budget set for {agent_id}")));
        }
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE agent_budgets SET paused = 1, updated_at = ?1 WHERE agent_id = ?2",
            params![now, agent_id],
        )?;
        let status = self.get_budget(agent_id)?.expect("just updated");

        let delegate = delegate_to.unwrap_or("human");
        let summary = format!(
            "# Budget-exhaustion handoff: {agent_id}\n\n\
             Generated: {now}\n\n\
             - agent: `{agent_id}`\n\
             - task: `{}`\n\
             - spent: {} / {} units\n\n\
             ## Objective\n\n{objective}\n\n\
             ## Completed\n\n{completed}\n\n\
             ## Missing / next steps\n\n{missing}\n\n\
             ## Delegated to\n\n`{delegate}`\n",
            task_id.unwrap_or("-"),
            status.spent_units,
            status.limit_units,
        );

        let handoffs_dir = self.data_dir.join("markdown").join("handoffs");
        fs::create_dir_all(&handoffs_dir)?;
        let stamp = now.replace([':', '.'], "-");
        let summary_path = handoffs_dir.join(format!("{stamp}-{agent_id}.md"));
        fs::write(&summary_path, &summary)?;

        let message = self.send_message(
            agent_id,
            delegate,
            MessageKind::Handoff,
            &summary,
            Some("budget exhausted: handoff"),
            None,
            task_id,
        )?;

        Ok(BudgetPauseOutcome {
            status,
            summary_path,
            handoff_message_id: message.id,
        })
    }

    /// Persist a cancellation/shutdown handoff so interrupted work is not lost.
    pub fn record_shutdown(
        &self,
        agent_id: &str,
        task_id: Option<&str>,
        objective: &str,
        reason: &str,
        delegate_to: Option<&str>,
    ) -> Result<ShutdownOutcome, HubError> {
        let delegate = delegate_to.unwrap_or("human");
        let now = Utc::now().to_rfc3339();
        let summary = format!(
            "# Shutdown handoff: {agent_id}\n\nGenerated: {now}\n\n- task: `{}`\n- delegated to: `{delegate}`\n- reason: {reason}\n\n## Objective\n\n{objective}\n",
            task_id.unwrap_or("-")
        );
        let handoffs_dir = self.data_dir.join("markdown").join("handoffs");
        fs::create_dir_all(&handoffs_dir)?;
        let stamp = now.replace([':', '.'], "-");
        let summary_path = handoffs_dir.join(format!("{stamp}-{agent_id}-shutdown.md"));
        fs::write(&summary_path, &summary)?;
        let message = self.send_message(
            agent_id,
            delegate,
            MessageKind::Handoff,
            &summary,
            Some("shutdown: handoff required"),
            None,
            task_id,
        )?;
        Ok(ShutdownOutcome {
            summary_path,
            handoff_message_id: message.id,
        })
    }
}
