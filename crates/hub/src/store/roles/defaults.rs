//! Per-provider default role resolution, the effective-permission union
//! across an agent's assigned roles, and the two seed steps every
//! `HubStore::open` runs: the protected `cto` role, and a starter set of
//! roles/provider-defaults matching the display titles the desktop chat
//! already showed before this system existed (`Lead Orchestrator`,
//! `Code Agent`, ...), so a freshly-migrated Hub's chat looks unchanged
//! until an owner (or Grok's Orchestrate UI) reconfigures it.

use super::super::*;
use super::CTO_ROLE_ID;

impl HubStore {
    /// Which role a `provider` (today, simply the agent id — `grok`,
    /// `claude`, `chat`, `gemini`, ...) gets assigned by default: the
    /// workspace-specific default if one exists, else the global default,
    /// else `None`.
    pub fn resolve_role_for_provider(
        &self,
        provider: &str,
        workspace_path: Option<&str>,
    ) -> Result<Option<Role>, HubError> {
        if let Some(workspace) = workspace_path.filter(|w| !w.is_empty()) {
            if let Some(role_id) = self.get_role_provider_default(provider, Some(workspace))? {
                return self.get_role(&role_id);
            }
        }
        if let Some(role_id) = self.get_role_provider_default(provider, None)? {
            return self.get_role(&role_id);
        }
        Ok(None)
    }

    fn get_role_provider_default(
        &self,
        provider: &str,
        workspace_path: Option<&str>,
    ) -> Result<Option<String>, HubError> {
        let workspace = workspace_path.unwrap_or("");
        self.conn
            .query_row(
                "SELECT role_id FROM role_provider_defaults WHERE provider = ?1 AND workspace_path = ?2",
                params![provider, workspace],
                |r| r.get(0),
            )
            .optional()
            .map_err(HubError::from)
    }

    /// Sets the default role a `provider` resolves to. `workspace_path =
    /// None` sets the *global* default; a workspace-specific default
    /// always takes priority over it for that same provider.
    pub fn set_role_provider_default(
        &self,
        provider: &str,
        workspace_path: Option<&str>,
        role_id: &str,
    ) -> Result<(), HubError> {
        self.get_role(role_id)?
            .ok_or_else(|| HubError::NotFound(role_id.to_string()))?;
        let workspace = workspace_path.unwrap_or("");
        self.conn.execute(
            "INSERT INTO role_provider_defaults(provider, workspace_path, role_id) VALUES (?1, ?2, ?3)
             ON CONFLICT(provider, workspace_path) DO UPDATE SET role_id = excluded.role_id",
            params![provider, workspace, role_id],
        )?;
        Ok(())
    }

    /// Every configured provider-default row (global and workspace-scoped
    /// alike), for a Shared Hub/Settings panel to render as an editable
    /// table rather than needing one lookup call per provider.
    pub fn list_role_provider_defaults(&self) -> Result<Vec<RoleProviderDefault>, HubError> {
        let mut stmt = self.conn.prepare(
            "SELECT provider, workspace_path, role_id FROM role_provider_defaults ORDER BY provider, workspace_path",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(RoleProviderDefault {
                provider: row.get(0)?,
                workspace_path: row.get(1)?,
                role_id: row.get(2)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// An agent's effective permissions: the union of every role it's
    /// explicitly assigned (numeric limits take the highest/most
    /// permissive value, `None` — unlimited — always wins; booleans OR;
    /// responsibilities union). With no explicit assignment, falls back to
    /// [`Self::resolve_role_for_provider`] (treating the agent id as the
    /// provider) for `workspace_path`; with no default either, the agent is
    /// **unlimited** — the role system only ever *restricts* an agent once
    /// it's actually been given a role with a finite limit. This keeps a
    /// freshly-migrated Hub (nothing configured yet) behaviorally identical
    /// to before role gating existed, rather than silently blocking every
    /// send until an owner configures roles for everyone.
    pub fn effective_agent_permissions(
        &self,
        agent_id: &str,
        workspace_path: Option<&str>,
    ) -> Result<EffectiveAgentPermissions, HubError> {
        let mut roles = self.list_agent_roles(agent_id)?;
        if roles.is_empty() {
            if let Some(default_role) = self.resolve_role_for_provider(agent_id, workspace_path)? {
                roles.push(default_role);
            }
        }
        if roles.is_empty() {
            return Ok(EffectiveAgentPermissions {
                agent_id: agent_id.to_string(),
                roles: Vec::new(),
                daily_ungated_quota: None,
                max_broadcast_recipients: None,
                can_archive_messages: false,
                can_update_agent_roles: false,
                can_allocate_tasks: false,
                responsibilities: Vec::new(),
            });
        }

        let mut daily_ungated_quota = Some(0i64);
        let mut max_broadcast_recipients = Some(0i64);
        let mut can_archive_messages = false;
        let mut can_update_agent_roles = false;
        let mut can_allocate_tasks = false;
        let mut responsibilities: Vec<String> = Vec::new();

        for role in &roles {
            daily_ungated_quota = merge_limit(daily_ungated_quota, role.daily_ungated_quota);
            max_broadcast_recipients =
                merge_limit(max_broadcast_recipients, role.max_broadcast_recipients);
            can_archive_messages |= role.can_archive_messages;
            can_update_agent_roles |= role.can_update_agent_roles;
            can_allocate_tasks |= role.can_allocate_tasks;
            for resp in &role.responsibilities {
                if !responsibilities.contains(resp) {
                    responsibilities.push(resp.clone());
                }
            }
        }

        Ok(EffectiveAgentPermissions {
            agent_id: agent_id.to_string(),
            roles,
            daily_ungated_quota,
            max_broadcast_recipients,
            can_archive_messages,
            can_update_agent_roles,
            can_allocate_tasks,
            responsibilities,
        })
    }

    /// Idempotently ensures the protected `cto` role exists (unlimited
    /// quotas, every capability, no default responsibilities beyond what
    /// full access already implies) and is assigned to `human`. Called
    /// once per `HubStore::open` via `migrate()` — the human must never be
    /// able to end up without it.
    pub(crate) fn ensure_builtin_roles(&self) -> Result<(), HubError> {
        if self.get_role(CTO_ROLE_ID)?.is_none() {
            let now = Utc::now().to_rfc3339();
            self.conn.execute(
                r#"
                INSERT INTO roles(
                    id, display_name, is_builtin, daily_ungated_quota, max_broadcast_recipients,
                    can_archive_messages, can_update_agent_roles, can_allocate_tasks,
                    responsibilities_json, created_at, updated_at
                ) VALUES (?1, 'CTO', 1, NULL, NULL, 1, 1, 1, '[]', ?2, ?2)
                "#,
                params![CTO_ROLE_ID, now],
            )?;
        }
        self.assign_agent_role("human", CTO_ROLE_ID)?;
        self.ensure_starter_role_defaults()
    }

    /// Idempotently (once ever, tracked via `meta`) seeds a starter role
    /// per standard provider, as a *global* default — matching the
    /// display titles `AGENT_COLORS` already hardcoded in the desktop
    /// chat before roles existed, so migrating to this system doesn't
    /// silently change anyone's displayed title or break existing task/
    /// wake delivery (see [`Self::effective_agent_permissions`]'s
    /// unlimited-until-configured fallback for the same reasoning). An
    /// owner (or Grok's Orchestrate UI) is expected to tune or replace
    /// these; this only ever runs once, so a later edit or delete is
    /// never overwritten by a repeat migration.
    pub(crate) fn ensure_starter_role_defaults(&self) -> Result<(), HubError> {
        let seeded: Option<String> = self
            .conn
            .query_row(
                "SELECT value FROM meta WHERE key = 'starter_roles_seeded'",
                [],
                |r| r.get(0),
            )
            .optional()?;
        if seeded.is_some() {
            return Ok(());
        }

        // (role id, display name, provider, daily quota, max recipients,
        //  can_archive, can_update_roles, can_allocate_tasks, responsibilities)
        #[allow(clippy::type_complexity)]
        let starters: [(
            &str,
            &str,
            &str,
            Option<i64>,
            Option<i64>,
            bool,
            bool,
            bool,
            &[&str],
        ); 4] = [
            (
                "lead-orchestrator",
                "Lead Orchestrator",
                "grok",
                Some(20),
                Some(10),
                true,
                true,
                true,
                &["task_allocator"],
            ),
            (
                "code-agent",
                "Code Agent",
                "claude",
                Some(10),
                Some(5),
                false,
                false,
                false,
                &[],
            ),
            (
                "co-lead",
                "Co-Lead / Codex",
                "chat",
                Some(10),
                Some(5),
                false,
                false,
                false,
                &["reviewer"],
            ),
            (
                "supporting",
                "Supporting",
                "gemini",
                Some(10),
                Some(5),
                false,
                false,
                false,
                &[],
            ),
        ];

        for (id, name, provider, quota, max_recipients, archive, update_roles, allocate, resp) in
            starters
        {
            self.upsert_role(
                id,
                name,
                quota,
                max_recipients,
                archive,
                update_roles,
                allocate,
                &resp.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
            )?;
            self.set_role_provider_default(provider, None, id)?;
        }

        self.conn.execute(
            "INSERT OR REPLACE INTO meta(key, value) VALUES ('starter_roles_seeded', '1')",
            [],
        )?;
        Ok(())
    }
}

/// `None` (unlimited) always wins; otherwise the higher of the two.
fn merge_limit(a: Option<i64>, b: Option<i64>) -> Option<i64> {
    match (a, b) {
        (None, _) | (_, None) => None,
        (Some(x), Some(y)) => Some(x.max(y)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn ensure_builtin_roles_seeds_cto_and_assigns_human() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let cto = store.get_role(CTO_ROLE_ID).unwrap().expect("cto seeded");
        assert!(cto.is_builtin);
        assert_eq!(cto.daily_ungated_quota, None);
        assert_eq!(cto.max_broadcast_recipients, None);
        assert!(cto.can_archive_messages && cto.can_update_agent_roles && cto.can_allocate_tasks);

        let human_roles = store.list_agent_roles("human").unwrap();
        assert!(human_roles.iter().any(|r| r.id == CTO_ROLE_ID));
    }

    #[test]
    fn ensure_starter_role_defaults_seeds_every_standard_provider_once() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();

        for provider in ["grok", "claude", "chat", "gemini"] {
            let role = store
                .resolve_role_for_provider(provider, None)
                .unwrap()
                .unwrap_or_else(|| panic!("no starter default for {provider}"));
            assert!(!role.is_builtin);
        }

        let grok_role = store
            .resolve_role_for_provider("grok", None)
            .unwrap()
            .unwrap();
        assert_eq!(grok_role.display_name, "Lead Orchestrator");
        assert!(grok_role.can_allocate_tasks);

        // Idempotent: an owner's edit afterward must survive a repeat open.
        store
            .upsert_role(
                "lead-orchestrator",
                "Customized Lead",
                Some(99),
                Some(99),
                true,
                true,
                true,
                &[],
            )
            .unwrap();
        store.ensure_starter_role_defaults().unwrap();
        let after = store.get_role("lead-orchestrator").unwrap().unwrap();
        assert_eq!(after.display_name, "Customized Lead");
    }

    #[test]
    fn effective_permissions_union_across_multiple_roles() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        store
            .upsert_role(
                "reviewer",
                "Reviewer",
                Some(10),
                Some(5),
                true,
                false,
                false,
                &["reviewer".to_string()],
            )
            .unwrap();
        store
            .upsert_role(
                "planner",
                "Planner",
                Some(5),
                Some(20),
                false,
                true,
                false,
                &["planner".to_string()],
            )
            .unwrap();
        store.assign_agent_role("gemini", "reviewer").unwrap();
        store.assign_agent_role("gemini", "planner").unwrap();

        let effective = store.effective_agent_permissions("gemini", None).unwrap();
        assert_eq!(effective.daily_ungated_quota, Some(10)); // max(10, 5)
        assert_eq!(effective.max_broadcast_recipients, Some(20)); // max(5, 20)
        assert!(effective.can_archive_messages); // from reviewer
        assert!(effective.can_update_agent_roles); // from planner
        assert!(!effective.can_allocate_tasks); // neither role grants it
        assert_eq!(effective.responsibilities.len(), 2);
    }

    #[test]
    fn effective_permissions_unlimited_wins_over_a_finite_limit() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        store
            .upsert_role(
                "finite",
                "Finite",
                Some(3),
                Some(3),
                false,
                false,
                false,
                &[],
            )
            .unwrap();
        store
            .upsert_role(
                "unlimited",
                "Unlimited",
                None,
                None,
                false,
                false,
                false,
                &[],
            )
            .unwrap();
        store.assign_agent_role("codex", "finite").unwrap();
        store.assign_agent_role("codex", "unlimited").unwrap();

        let effective = store.effective_agent_permissions("codex", None).unwrap();
        assert_eq!(effective.daily_ungated_quota, None);
        assert_eq!(effective.max_broadcast_recipients, None);
    }

    #[test]
    fn effective_permissions_falls_back_to_provider_default_then_unlimited() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        store
            .upsert_role(
                "lead2",
                "Lead Orchestrator 2",
                Some(15),
                Some(8),
                false,
                false,
                true,
                &[],
            )
            .unwrap();
        // Override the starter default so this test is self-contained.
        store
            .set_role_provider_default("grok", None, "lead2")
            .unwrap();

        let via_default = store.effective_agent_permissions("grok", None).unwrap();
        assert_eq!(via_default.daily_ungated_quota, Some(15));
        assert!(via_default.can_allocate_tasks);

        // No assignment and no default at all: unlimited, not gated — the
        // role system only ever restricts an agent once it's configured.
        let unconfigured = store
            .effective_agent_permissions("mystery-agent", None)
            .unwrap();
        assert_eq!(unconfigured.daily_ungated_quota, None);
        assert_eq!(unconfigured.max_broadcast_recipients, None);
        assert!(!unconfigured.can_allocate_tasks);
    }

    #[test]
    fn workspace_specific_provider_default_overrides_the_global_one() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        store
            .upsert_role(
                "global-role",
                "Global Role",
                Some(5),
                Some(5),
                false,
                false,
                false,
                &[],
            )
            .unwrap();
        store
            .upsert_role(
                "workspace-role",
                "Workspace Role",
                Some(50),
                Some(50),
                false,
                false,
                false,
                &[],
            )
            .unwrap();
        store
            .set_role_provider_default("gemini", None, "global-role")
            .unwrap();
        store
            .set_role_provider_default("gemini", Some("/abs/repo"), "workspace-role")
            .unwrap();

        let global = store.effective_agent_permissions("gemini", None).unwrap();
        assert_eq!(global.daily_ungated_quota, Some(5));

        let scoped = store
            .effective_agent_permissions("gemini", Some("/abs/repo"))
            .unwrap();
        assert_eq!(scoped.daily_ungated_quota, Some(50));
    }

    #[test]
    fn list_role_provider_defaults_returns_every_configured_row() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        store
            .upsert_role(
                "lead3",
                "Lead Orchestrator 3",
                Some(15),
                Some(8),
                false,
                false,
                true,
                &[],
            )
            .unwrap();
        store
            .set_role_provider_default("grok", None, "lead3")
            .unwrap();
        store
            .set_role_provider_default("grok", Some("/abs/repo"), "lead3")
            .unwrap();

        let rows = store.list_role_provider_defaults().unwrap();
        // 4 starter defaults (grok/claude/chat/gemini) + the 2 set here,
        // minus 1 since "grok" global gets overwritten by lead3 (still one row).
        assert!(rows.len() >= 5);
        assert!(rows.iter().any(|r| r.provider == "grok"
            && r.workspace_path == "/abs/repo"
            && r.role_id == "lead3"));
    }
}
