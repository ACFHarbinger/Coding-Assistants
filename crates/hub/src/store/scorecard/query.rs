use super::helpers::*;
use super::types::*;
use crate::settings::ScorecardVisibility;
use crate::store::{HubError, HubStore};
use chrono::Utc;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};

impl HubStore {
    /// Projects stamped audit rows, tasks, work sessions, budgets, and quota snapshots
    /// into the three scorecard row kinds: agent, harness, provider.
    pub fn get_scorecard(
        &self,
        caller: Option<&str>,
        visibility: ScorecardVisibility,
    ) -> Result<ScorecardView, HubError> {
        let is_owner = caller.is_none_or(|c| c.trim().is_empty() || c == "human");

        // Visibility enforcement at read time (T3 / #343).
        if !is_owner && visibility == ScorecardVisibility::OwnerOnly {
            return Ok(ScorecardView {
                agents: Vec::new(),
                harnesses: Vec::new(),
                providers: Vec::new(),
                aggregation_version: 1,
                generated_at: Utc::now().to_rfc3339(),
            });
        }

        // 1. Load roster agents
        let has_oversight: bool = self
            .conn
            .prepare("SELECT oversight_role FROM agents LIMIT 1")
            .is_ok();

        let mut agents_map: BTreeMap<String, (String, Option<String>)> = BTreeMap::new();
        if has_oversight {
            let mut stmt = self
                .conn
                .prepare("SELECT id, display_name, oversight_role FROM agents ORDER BY id")?;
            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            })?;
            for r in rows {
                let (id, name, oversight) = r?;
                agents_map.insert(id, (name, oversight));
            }
        } else {
            let mut stmt = self
                .conn
                .prepare("SELECT id, display_name, role FROM agents ORDER BY id")?;
            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            })?;
            for r in rows {
                let (id, name, role) = r?;
                agents_map.insert(id, (name, role));
            }
        }

        // 2. Load agent budgets
        let mut budgets: HashMap<String, (f64, f64, bool)> = HashMap::new();
        {
            let mut stmt = self
                .conn
                .prepare("SELECT agent_id, limit_units, spent_units, paused FROM agent_budgets")?;
            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, f64>(1)?,
                    row.get::<_, f64>(2)?,
                    row.get::<_, i64>(3)? != 0,
                ))
            })?;
            for r in rows {
                let (id, limit, spent, paused) = r?;
                budgets.insert(id, (limit, spent, paused));
            }
        }

        // 3. Load pending gate approvals where a human is an explicit decision target
        let mut oldest_decisions: HashMap<String, String> = HashMap::new();
        {
            let mut stmt = self.conn.prepare(
                "SELECT from_agent, to_agents_json, created_at FROM pending_gate_approvals \
                 WHERE status = 'pending' ORDER BY created_at ASC",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })?;
            for r in rows {
                let (from_a, to_json, created) = r?;
                let is_human_target = match serde_json::from_str::<Value>(&to_json) {
                    Ok(Value::Array(arr)) => arr.iter().any(|v| v.as_str() == Some("human")),
                    _ => false,
                };
                if is_human_target {
                    oldest_decisions
                        .entry(from_a)
                        .and_modify(|existing| {
                            if created < *existing {
                                *existing = created.clone();
                            }
                        })
                        .or_insert(created);
                }
            }
        }

        // 4. Process audit events for security-class rows and provider.usage
        let mut security_events: HashMap<String, Vec<ScorecardSecurityEvent>> = HashMap::new();
        let mut agent_usage: HashMap<String, UsageAccumulator> = HashMap::new();
        let mut provider_metrics: BTreeMap<(String, Option<String>), UsageAccumulator> =
            BTreeMap::new();

        {
            let mut stmt = self.conn.prepare(
                "SELECT id, operation, observed_at, process_json, status FROM audit_events ORDER BY rowid ASC",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })?;

            for r in rows {
                let (id, operation, observed_at, process_raw, status) = r?;
                let json_meta: Value = serde_json::from_str(&process_raw).unwrap_or(Value::Null);

                // Explicit actor identity only; do not fall back to attribution basis
                let explicit_actor = json_meta
                    .get("actor_agent")
                    .or_else(|| json_meta.get("agent"))
                    .or_else(|| json_meta.get("actor"))
                    .and_then(Value::as_str)
                    .map(str::to_string);

                let task_id = json_meta
                    .get("task_id")
                    .and_then(Value::as_str)
                    .map(str::to_string);

                let session_id = json_meta
                    .get("work_session_id")
                    .or_else(|| json_meta.get("session_id"))
                    .and_then(Value::as_str)
                    .map(str::to_string);

                if operation == "provider.usage" {
                    let provider = json_meta
                        .get("provider")
                        .and_then(Value::as_str)
                        .unwrap_or("unknown")
                        .to_string();
                    let via_gateway = json_meta
                        .get("via_gateway")
                        .and_then(Value::as_str)
                        .map(str::to_string);

                    let prompt_opt = json_meta.get("prompt_tokens").and_then(Value::as_u64);
                    let completion_opt =
                        json_meta.get("completion_tokens").and_then(Value::as_u64);
                    let has_tok = prompt_opt.is_some() || completion_opt.is_some();

                    let cost_opt = json_meta.get("cost").and_then(Value::as_f64);
                    let currency_is_measured = json_meta.get("currency").and_then(Value::as_str)
                        == Some("measured");
                    let has_cost = cost_opt.is_some() || currency_is_measured;
                    let cost_val = cost_opt.unwrap_or(0.0);

                    let p_entry = provider_metrics
                        .entry((provider, via_gateway))
                        .or_default();
                    if has_tok {
                        p_entry.prompt_tokens += prompt_opt.unwrap_or(0);
                        p_entry.completion_tokens += completion_opt.unwrap_or(0);
                        p_entry.has_tokens = true;
                    }
                    if has_cost {
                        p_entry.cost += cost_val;
                        p_entry.has_cost = true;
                    }
                    p_entry.request_count += 1;

                    let agent_key = explicit_actor
                        .clone()
                        .unwrap_or_else(|| "unattributed".to_string());
                    let a_entry = agent_usage.entry(agent_key).or_default();
                    if has_tok {
                        a_entry.prompt_tokens += prompt_opt.unwrap_or(0);
                        a_entry.completion_tokens += completion_opt.unwrap_or(0);
                        a_entry.has_tokens = true;
                    }
                    if has_cost {
                        a_entry.cost += cost_val;
                        a_entry.has_cost = true;
                    }
                    a_entry.request_count += 1;
                }

                if is_security_class_op(&operation) && is_open_security_event(&status, &operation) {
                    let event = ScorecardSecurityEvent {
                        id,
                        operation,
                        observed_at,
                        status,
                        task_id,
                        work_session_id: session_id,
                        provenance: FigureProvenance::Measured,
                    };
                    let key = explicit_actor
                        .clone()
                        .unwrap_or_else(|| "unattributed".to_string());
                    security_events.entry(key).or_default().push(event);
                }

                // Preserve non-roster explicit actors
                if let Some(ref act) = explicit_actor {
                    if !agents_map.contains_key(act) {
                        agents_map.insert(act.clone(), (act.clone(), None));
                    }
                }
            }
        }

        // 5. Build Agent rows
        let mut agent_rows: Vec<AgentScorecardRow> = Vec::new();
        for (aid, (name, oversight)) in agents_map {
            let open_sec = security_events.remove(&aid).unwrap_or_default();
            let oldest_pending = oldest_decisions.remove(&aid);

            let (budget_limit, budget_spent, paused) = budgets
                .remove(&aid)
                .map(|(l, s, p)| (Some(l), Some(s), p))
                .unwrap_or((None, None, false));

            let usage = agent_usage.remove(&aid).unwrap_or_default();

            let tokens_figure = if usage.has_tokens {
                ScorecardFigure::measured(
                    (usage.prompt_tokens + usage.completion_tokens) as f64,
                    "tokens",
                )
            } else {
                ScorecardFigure::unknown("tokens")
            };

            let (cost_figure, prov_str, measured_cost_units, estimated_cost_units) =
                if usage.has_cost {
                    (
                        ScorecardFigure::measured(usage.cost, "currency"),
                        "measured".to_string(),
                        Some(usage.cost),
                        None,
                    )
                } else if let Some(spent) = budget_spent {
                    (
                        ScorecardFigure::estimated(spent, "units"),
                        "estimated".to_string(),
                        None,
                        Some(spent),
                    )
                } else {
                    (
                        ScorecardFigure::unknown("currency"),
                        "unknown".to_string(),
                        None,
                        None,
                    )
                };

            agent_rows.push(AgentScorecardRow {
                agent_id: aid,
                display_name: name,
                oversight_role: oversight,
                open_security_events: open_sec,
                oldest_pending_human_decision: oldest_pending,
                tokens: tokens_figure,
                cost: cost_figure,
                measured_cost_units,
                estimated_cost_units,
                budget_spent,
                budget_limit,
                budget_paused: paused,
                provenance: prov_str,
                task_count: ScorecardFigure::unknown("tasks"),
            });
        }

        // Preserve unattributed open security events or usage
        let unattr_events = security_events.remove("unattributed").unwrap_or_default();
        let unattr_usage = agent_usage.remove("unattributed").unwrap_or_default();
        if !unattr_events.is_empty() || unattr_usage.has_tokens || unattr_usage.has_cost {
            let tokens_figure = if unattr_usage.has_tokens {
                ScorecardFigure::measured(
                    (unattr_usage.prompt_tokens + unattr_usage.completion_tokens) as f64,
                    "tokens",
                )
            } else {
                ScorecardFigure::unknown("tokens")
            };
            let (cost_figure, prov_str, measured_cost_units) = if unattr_usage.has_cost {
                (
                    ScorecardFigure::measured(unattr_usage.cost, "currency"),
                    "measured".to_string(),
                    Some(unattr_usage.cost),
                )
            } else {
                (
                    ScorecardFigure::unknown("currency"),
                    "unknown".to_string(),
                    None,
                )
            };

            agent_rows.push(AgentScorecardRow {
                agent_id: "unattributed".to_string(),
                display_name: "Unattributed Operations".to_string(),
                oversight_role: None,
                open_security_events: unattr_events,
                oldest_pending_human_decision: None,
                tokens: tokens_figure,
                cost: cost_figure,
                measured_cost_units,
                estimated_cost_units: None,
                budget_spent: None,
                budget_limit: None,
                budget_paused: false,
                provenance: prov_str,
                task_count: ScorecardFigure::unknown("tasks"),
            });
        }

        // Sort agent rows:
        // 1. open security-class rows first (descending by count)
        // 2. oldest pending human decision (ascending by timestamp, Some before None)
        // 3. measured cost ONLY (descending). Estimated budget units must NOT be compared here!
        // 4. alphabetical tie-breaker
        // NEVER a composite score.
        agent_rows.sort_by(|a, b| {
            let a_sec = a.open_security_events.len();
            let b_sec = b.open_security_events.len();
            if a_sec != b_sec {
                return b_sec.cmp(&a_sec);
            }

            match (
                &a.oldest_pending_human_decision,
                &b.oldest_pending_human_decision,
            ) {
                (Some(ta), Some(tb)) if ta != tb => return ta.cmp(tb),
                (Some(_), None) => return std::cmp::Ordering::Less,
                (None, Some(_)) => return std::cmp::Ordering::Greater,
                _ => {}
            }

            match (a.measured_cost_units, b.measured_cost_units) {
                (Some(ca), Some(cb)) if (ca - cb).abs() > f64::EPSILON => {
                    return cb.partial_cmp(&ca).unwrap_or(std::cmp::Ordering::Equal);
                }
                (Some(_), None) => return std::cmp::Ordering::Less,
                (None, Some(_)) => return std::cmp::Ordering::Greater,
                _ => {}
            }

            a.agent_id.cmp(&b.agent_id)
        });

        // 6. Build Harness rows
        let mut harness_rows: Vec<HarnessScorecardRow> = Vec::new();
        let mut registered_workspaces = Vec::new();
        {
            let mut stmt = self.conn.prepare(
                "SELECT harness, workspace, disk_session_id, mode, state, writer_owner \
                 FROM harness_session_registrations ORDER BY harness, workspace",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<String>>(5)?,
                ))
            })?;

            for r in rows {
                let (harness, workspace, session_id, mode, state, writer_owner) = r?;
                if !registered_workspaces.contains(&workspace) {
                    registered_workspaces.push(workspace.clone());
                }
                let coverage = match mode.as_str() {
                    "managed" => HarnessCoverage::Managed,
                    _ => HarnessCoverage::Observed,
                };
                // Only set linked_agent_id when proven by explicit registration relationship
                let linked_agent_id = writer_owner.filter(|w| !w.trim().is_empty());
                harness_rows.push(HarnessScorecardRow {
                    linked_agent_id,
                    harness,
                    workspace,
                    coverage,
                    disk_session_id: Some(session_id),
                    state: Some(state),
                    session_count: ScorecardFigure::measured(1.0, "sessions"),
                });
            }
        }

        // Add unregistered harnesses for registered workspaces (linked_agent_id is None)
        let target_workspaces = if registered_workspaces.is_empty() {
            vec!["default".to_string()]
        } else {
            registered_workspaces
        };

        for ws in &target_workspaces {
            for &kh in KNOWN_HARNESSES {
                let exists = harness_rows
                    .iter()
                    .any(|r| r.harness == kh && r.workspace == *ws);
                if !exists {
                    harness_rows.push(HarnessScorecardRow {
                        harness: kh.to_string(),
                        workspace: ws.clone(),
                        coverage: HarnessCoverage::Unregistered,
                        linked_agent_id: None,
                        disk_session_id: None,
                        state: None,
                        session_count: ScorecardFigure::unknown("sessions"),
                    });
                }
            }
        }

        // 7. Build Provider rows
        let mut provider_rows: Vec<ProviderScorecardRow> = Vec::new();
        for ((provider, via_gateway), p) in provider_metrics {
            let tokens_fig = if p.has_tokens {
                ScorecardFigure::measured(
                    (p.prompt_tokens + p.completion_tokens) as f64,
                    "tokens",
                )
            } else {
                ScorecardFigure::unknown("tokens")
            };
            let cost_fig = if p.has_cost {
                ScorecardFigure::measured(p.cost, "currency")
            } else {
                ScorecardFigure::unknown("currency")
            };
            provider_rows.push(ProviderScorecardRow {
                provider,
                via_gateway,
                tokens: tokens_fig,
                cost: cost_fig,
                request_count: ScorecardFigure::measured(p.request_count as f64, "requests"),
            });
        }

        // Filter if non-owner and visibility is AgentsOwnCard
        if !is_owner && visibility == ScorecardVisibility::AgentsOwnCard {
            if let Some(target_agent) = caller {
                agent_rows.retain(|a| a.agent_id == target_agent);
                harness_rows.retain(|h| h.linked_agent_id.as_deref() == Some(target_agent));
                provider_rows.clear();
            }
        }

        Ok(ScorecardView {
            agents: agent_rows,
            harnesses: harness_rows,
            providers: provider_rows,
            aggregation_version: 1,
            generated_at: Utc::now().to_rfc3339(),
        })
    }
}
