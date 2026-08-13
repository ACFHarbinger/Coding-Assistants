use super::super::quota_claude::{claude_home, claude_quota};
use super::super::quota_grok::{
    grok_home, grok_quota, grok_token_from_auth, grok_windows_from_value,
};

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
