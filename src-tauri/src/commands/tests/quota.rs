use super::super::quota_claude::{claude_home, claude_quota};
use super::super::quota_codex::{
    ProviderQuotaBalance, ProviderQuotaLocalUsage, ProviderQuotaWindow,
};
use super::super::quota_grok::{
    grok_home, grok_quota, grok_token_from_auth, grok_windows_from_value,
};
use super::super::quota_mistral::MistralAdminBudget;
use super::super::quotas::compose_mistral_quota;

#[test]
fn claude_quota_is_well_formed_when_logged_in() {
    if !claude_home().join(".credentials.json").exists() {
        return;
    }
    let quota = claude_quota();
    assert_eq!(quota.agent_id, "claude");
    assert_eq!(quota.provider, "anthropic");
    match quota.status.as_str() {
        "ok" => {
            for window in &quota.windows {
                assert!((0..=100).contains(&window.used_percent));
                assert_eq!(window.used_percent + window.remaining_percent, 100);
            }
        }
        "unavailable" => assert!(quota.detail.is_some()),
        other => panic!("unexpected status: {other}"),
    }
}

#[test]
fn grok_token_prefers_accounts_sign_in_key() {
    let auth = serde_json::json!({"https://accounts.x.ai/sign-in": {"key": "session-token-from-grok-login-xyz"}, "access_token": "should-not-win-over-sign-in-key"});
    assert_eq!(
        grok_token_from_auth(&auth).as_deref(),
        Some("session-token-from-grok-login-xyz")
    );
    let alt = serde_json::json!({"https://auth.x.ai/callback": {"access_token": "oidc-access-token-value-xx"}});
    assert_eq!(
        grok_token_from_auth(&alt).as_deref(),
        Some("oidc-access-token-value-xx")
    );
}

#[test]
fn grok_windows_parse_weekly_credit_snapshot() {
    let payload = serde_json::json!({"isUnifiedBillingUser": true, "creditUsagePercent": 37.4, "currentPeriod": "WEEKLY", "billingPeriodStart": "2026-08-10T00:00:00Z", "billingPeriodEnd": "2026-08-17T00:00:00Z", "onDemandUsed": 2.5, "onDemandCap": 10.0, "history": [{"creditUsagePercent": 99, "currentPeriod": "WEEKLY"}]});
    let windows = grok_windows_from_value(&payload);
    assert_eq!(windows.len(), 2, "{windows:?}");
    assert_eq!(windows[0].label, "Weekly");
    assert_eq!(windows[0].used_percent, 37);
    assert_eq!(windows[0].remaining_percent, 63);
    assert_eq!(windows[0].window_minutes, Some(7 * 24 * 60));
    assert_eq!(windows[0].resets_at, Some(1_786_924_800));
    assert_eq!(windows[1].label, "Extra usage credits");
    assert_eq!(windows[1].used_percent, 25);
    assert_eq!(windows[1].remaining_percent, 75);
}

#[test]
fn grok_quota_is_well_formed_when_logged_in() {
    if !grok_home().join("auth.json").exists() {
        return;
    }
    let quota = grok_quota();
    assert_eq!(quota.agent_id, "grok");
    assert_eq!(quota.provider, "xai");
    match quota.status.as_str() {
        "ok" => {
            for window in &quota.windows {
                assert!((0..=100).contains(&window.used_percent));
                assert_eq!(window.used_percent + window.remaining_percent, 100);
            }
        }
        "unavailable" => assert!(quota.detail.is_some()),
        other => panic!("unexpected status: {other}"),
    }
}

/// A capped Mistral Admin API response merged with local Vibe usage (#306):
/// the Monthly spend window and the spend-typed currency figure must survive
/// beside `local_usage` — spend must never be shaped like available credit.
fn capped_admin_budget() -> MistralAdminBudget {
    MistralAdminBudget {
        windows: vec![ProviderQuotaWindow {
            label: "Monthly spend".into(),
            family: Some("Mistral Admin".into()),
            used_percent: 16,
            remaining_percent: 84,
            resets_at: None,
            window_minutes: None,
        }],
        balance: Some("Spent $16.25 USD this period (Vibe $12.50)".into()),
        balance_info: Some(ProviderQuotaBalance {
            currency: "USD".into(),
            total: 16.25,
            kind: Some("spend".into()),
            spent: None,
            granted: None,
            topped_up: None,
            paid: None,
            gift: None,
        }),
        detail: None,
    }
}

fn two_session_local_usage() -> ProviderQuotaLocalUsage {
    ProviderQuotaLocalUsage {
        sessions: 2,
        prompt_tokens: 4967,
        completion_tokens: 40,
        cached_tokens: 2048,
        tool_calls_succeeded: 3,
        tool_calls_failed: 1,
        tool_calls_rejected: 0,
        since: Some(1_786_000_000),
    }
}

#[test]
fn mistral_quota_merges_capped_admin_budget_with_local_usage() {
    let quota = compose_mistral_quota(Ok(capped_admin_budget()), Some(two_session_local_usage()));
    assert_eq!(quota.agent_id, "mistral");
    assert_eq!(quota.provider, "mistral");
    assert_eq!(quota.status, "ok");
    assert!(quota.detail.is_none());

    assert_eq!(quota.windows.len(), 1);
    assert_eq!(quota.windows[0].label, "Monthly spend");
    assert_eq!(quota.windows[0].used_percent, 16);
    assert_eq!(quota.windows[0].remaining_percent, 84);

    let info = quota.balance_info.expect("currency figure");
    assert_eq!(info.currency, "USD");
    assert!((info.total - 16.25).abs() < 1e-9);
    // Spend, not credit: the Usage UI keys its "Period spend" rendering off this.
    assert_eq!(info.kind.as_deref(), Some("spend"));

    let local = quota.local_usage.expect("local usage preserved");
    assert_eq!(local.sessions, 2);
    assert_eq!(local.prompt_tokens, 4967);
    assert_eq!(local.completion_tokens, 40);
}

#[test]
fn mistral_quota_without_an_admin_key_stays_local_only() {
    let quota = compose_mistral_quota(
        Err("MISTRAL_ADMIN_API_KEY is not set".into()),
        Some(two_session_local_usage()),
    );
    assert_eq!(quota.status, "ok");
    assert!(quota.windows.is_empty());
    assert!(quota.balance.is_none());
    assert!(quota.balance_info.is_none());
    assert_eq!(quota.local_usage.map(|local| local.sessions), Some(2));
    assert!(quota
        .detail
        .as_deref()
        .unwrap_or_default()
        .contains("MISTRAL_ADMIN_API_KEY is not set"));
}

#[test]
fn mistral_quota_with_neither_half_is_unavailable_with_both_reasons() {
    let quota = compose_mistral_quota(Err("MISTRAL_ADMIN_API_KEY is not set".into()), None);
    assert_eq!(quota.status, "unavailable");
    let detail = quota.detail.unwrap_or_default();
    assert!(
        detail.contains("MISTRAL_ADMIN_API_KEY is not set"),
        "{detail}"
    );
    assert!(
        detail.contains("No Mistral Vibe sessions recorded yet"),
        "{detail}"
    );
}
