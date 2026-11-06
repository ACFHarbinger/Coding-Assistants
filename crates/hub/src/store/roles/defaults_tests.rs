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
    assert!(rows
        .iter()
        .any(|r| r.provider == "grok" && r.workspace_path == "/abs/repo" && r.role_id == "lead3"));
}
