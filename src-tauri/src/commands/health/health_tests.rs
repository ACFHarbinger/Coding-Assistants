use super::*;
use chrono::DateTime;

#[test]
fn aggregate_covers_every_known_agent_id_without_panicking() {
    let all: Vec<_> = ALL_AGENT_IDS.iter().map(|id| probe(id)).collect();
    assert_eq!(all.len(), ALL_AGENT_IDS.len());
    for h in &all {
        assert!(!h.agent_id.is_empty());
        assert!(!h.provider.is_empty());
        assert!(!h.detail.is_empty());
        assert!(DateTime::parse_from_rfc3339(&h.checked_at).is_ok());
    }
}

#[test]
fn unknown_agent_id_degrades_cleanly() {
    let h = probe("not-a-provider");
    assert!(!h.installed);
    assert_eq!(h.authenticated, None);
    assert_eq!(h.provider, "unknown");
}

#[test]
fn deepseek_needs_no_binary_but_tracks_key_presence() {
    let h = deepseek_health();
    assert!(h.installed, "HTTP-only provider is always 'installed'");
    assert!(h.authenticated.is_some(), "key presence is always knowable");
}

#[test]
fn gemini_health_is_binary_presence_only() {
    let h = gemini_health();
    // `agy` may or may not be on PATH in CI; either way auth is unknowable
    // cheaply and we must never claim a definite auth state.
    assert_eq!(h.authenticated, None);
    assert_eq!(h.agent_id, "gemini");
}

#[test]
fn epoch_conversion_round_trips_a_known_instant() {
    // 2026-01-01T00:00:00Z
    let iso = epoch_secs_to_iso(1_767_225_600).unwrap();
    assert!(iso.starts_with("2026-01-01T00:00:00"));
}

#[test]
fn grok_token_present_detects_a_scoped_key_and_ignores_short_values() {
    let ok = serde_json::json!({
        "https://accounts.x.ai/sign-in": { "key": "abcdefghijklmnopqrstuvwxyz0123456789" }
    });
    assert!(grok_token_present(&ok));
    let short = serde_json::json!({
        "https://accounts.x.ai/sign-in": { "key": "tiny" }
    });
    assert!(!grok_token_present(&short));
    let unscoped = serde_json::json!({ "note": "no https scope here" });
    assert!(!grok_token_present(&unscoped));
}

#[test]
fn health_snapshot_serializes_camel_case_and_hides_none_fields() {
    let h = health(&CLAUDE, true, Some(true), None, "ok");
    let json = serde_json::to_value(&h).unwrap();
    assert_eq!(json["agentId"], "claude");
    assert_eq!(json["harnessTitle"], "Anthropic Claude Code");
    assert!(json.get("authExpiresAt").is_none());
    assert!(json.get("endpointReachable").is_none());
}

#[test]
fn qwen_health_covers_login_and_unknown_states() {
    let logged_in = qwen_health_with(true, QwenAuth::LoggedIn);
    assert!(logged_in.installed);
    assert_eq!(logged_in.authenticated, Some(true));
    assert!(logged_in.detail.contains("login marker"));

    let logged_out = qwen_health_with(true, QwenAuth::LoggedOut);
    assert_eq!(logged_out.authenticated, Some(false));
    assert!(logged_out.detail.contains("not logged in"));

    let unknown = qwen_health_with(true, QwenAuth::Unknown);
    assert_eq!(unknown.authenticated, None);
    assert_eq!(unknown.agent_id, "qwen");

    let missing = qwen_health_with(false, QwenAuth::Unknown);
    assert!(!missing.installed);
    assert_eq!(missing.authenticated, Some(false));
}

#[test]
fn muse_health_covers_all_installation_and_login_states() {
    let both = muse_health_with(true, true);
    assert!(both.installed);
    assert_eq!(both.authenticated, Some(true));
    assert!(both.detail.contains("logged in"));

    let inst_no_login = muse_health_with(true, false);
    assert!(inst_no_login.installed);
    assert_eq!(inst_no_login.authenticated, Some(false));
    assert!(inst_no_login.detail.contains("not logged in"));

    let login_no_bin = muse_health_with(false, true);
    assert!(!login_no_bin.installed);
    assert_eq!(login_no_bin.authenticated, Some(true));
    assert!(login_no_bin.detail.contains("not on PATH"));

    let neither = muse_health_with(false, false);
    assert!(!neither.installed);
    assert_eq!(neither.authenticated, Some(false));
    assert!(neither.detail.contains("not on PATH"));
}

#[test]
fn claude_health_honours_the_refresh_token_over_the_access_token_expiry() {
    let past_ms = (Utc::now().timestamp() - 3_600) * 1000;
    let far_future_ms = (Utc::now().timestamp() + 7 * 86_400) * 1000;

    // Access token already past `expiresAt`, but a live refresh token: the
    // session is fine and the surfaced expiry is the refresh token's.
    let refreshing = serde_json::json!({
        "claudeAiOauth": {
            "accessToken": "a".repeat(40),
            "refreshToken": "r".repeat(40),
            "expiresAt": past_ms,
            "refreshTokenExpiresAt": far_future_ms,
        }
    });
    let h = claude_health_from(true, &refreshing);
    assert_eq!(h.authenticated, Some(true));
    assert!(h.detail.contains("refreshes automatically"));
    assert!(
        h.auth_expires_at.is_some(),
        "reports the refresh-token expiry"
    );

    // Refresh token itself expired -> a real re-login is needed.
    let stale = serde_json::json!({
        "claudeAiOauth": {
            "refreshToken": "r".repeat(40),
            "expiresAt": past_ms,
            "refreshTokenExpiresAt": past_ms,
        }
    });
    let h2 = claude_health_from(true, &stale);
    assert_eq!(h2.authenticated, Some(false));
    assert!(h2.detail.contains("sign in again"));

    // No refresh token at all: fall back to the access-token `expiresAt`.
    let no_refresh_expired = serde_json::json!({
        "claudeAiOauth": { "expiresAt": past_ms }
    });
    let h3 = claude_health_from(true, &no_refresh_expired);
    assert_eq!(h3.authenticated, Some(false));

    let no_refresh_live = serde_json::json!({
        "claudeAiOauth": { "expiresAt": far_future_ms }
    });
    let h4 = claude_health_from(true, &no_refresh_live);
    assert_eq!(h4.authenticated, Some(true));
}

#[test]
fn cursor_health_covers_all_installation_and_token_expiry_states() {
    use super::super::quota_cursor::CursorAuthDetails;

    // 1. Installed, valid unexpired token
    let unexpired = CursorAuthDetails {
        token_present: true,
        auth_expires_at: Some("2026-09-08T22:00:00Z".into()),
        is_expired: false,
        file_present: true,
    };
    let h1 = cursor_health_with(true, unexpired);
    assert!(h1.installed);
    assert_eq!(h1.authenticated, Some(true));
    assert_eq!(h1.auth_expires_at.as_deref(), Some("2026-09-08T22:00:00Z"));
    assert_eq!(h1.detail, "Cursor Agent is logged in");

    // 2. Installed, expired token
    let expired = CursorAuthDetails {
        token_present: true,
        auth_expires_at: Some("2026-09-08T18:00:00Z".into()),
        is_expired: true,
        file_present: true,
    };
    let h2 = cursor_health_with(true, expired);
    assert!(h2.installed);
    assert_eq!(h2.authenticated, Some(false));
    assert_eq!(h2.auth_expires_at.as_deref(), Some("2026-09-08T18:00:00Z"));
    assert!(h2.detail.contains("expired"));

    // 3. Installed, file present but no token
    let no_tok_file = CursorAuthDetails {
        token_present: false,
        auth_expires_at: None,
        is_expired: false,
        file_present: true,
    };
    let h3 = cursor_health_with(true, no_tok_file);
    assert!(h3.installed);
    assert_eq!(h3.authenticated, Some(false));
    assert!(h3.detail.contains("CLI auth file has no access token"));

    // 4. Installed, no file and no vault token
    let not_logged_in = CursorAuthDetails {
        token_present: false,
        auth_expires_at: None,
        is_expired: false,
        file_present: false,
    };
    let h4 = cursor_health_with(true, not_logged_in.clone());
    assert!(h4.installed);
    assert_eq!(h4.authenticated, Some(false));
    assert!(h4.detail.contains("not logged in"));

    // 5. Not installed, unexpired token
    let unexp_no_bin = CursorAuthDetails {
        token_present: true,
        auth_expires_at: Some("2026-09-08T22:00:00Z".into()),
        is_expired: false,
        file_present: false,
    };
    let h5 = cursor_health_with(false, unexp_no_bin);
    assert!(!h5.installed);
    assert_eq!(h5.authenticated, Some(true));
    assert!(h5.detail.contains("credentials are present"));

    // 6. Not installed, expired token
    let exp_no_bin = CursorAuthDetails {
        token_present: true,
        auth_expires_at: Some("2026-09-08T18:00:00Z".into()),
        is_expired: true,
        file_present: false,
    };
    let h6 = cursor_health_with(false, exp_no_bin);
    assert!(!h6.installed);
    assert_eq!(h6.authenticated, Some(false));
    assert!(h6.detail.contains("credentials are expired"));

    // 7. Not installed, no credentials
    let h7 = cursor_health_with(false, not_logged_in);
    assert!(!h7.installed);
    assert_eq!(h7.authenticated, Some(false));
    assert_eq!(h7.detail, "The Cursor `agent` CLI was not found on PATH");
}

#[test]
fn local_runtime_health_accepts_any_of_several_binary_names() {
    // All-bogus list: not installed, and the detail names what was tried so
    // a still-red dot is self-diagnosing.
    let miss =
        local_runtime_health_multi(&LLAMACPP, &["not-a-real-binary-xyz", "also-not-real-abc"]);
    assert!(!miss.installed);
    assert_eq!(miss.authenticated, None);
    assert!(miss.detail.contains("not-a-real-binary-xyz"));
    assert!(miss.detail.contains("also-not-real-abc"));

    // Any one candidate on PATH ⇒ installed; the detail names the match.
    // `ps` stands in for a real llama.cpp binary (present in every env this
    // test runs in).
    let hit = local_runtime_health_multi(&LLAMACPP, &["not-a-real-binary-xyz", "ps"]);
    assert!(hit.installed);
    assert_eq!(hit.authenticated, None);
    assert!(hit.detail.contains("`ps`"));
}

#[test]
fn llamacpp_probe_still_covers_the_canonical_server_name() {
    // Guard against a future edit dropping the primary name while widening.
    assert!(LLAMACPP_BINS.contains(&"llama-server"));
    let h = llamacpp_health();
    assert_eq!(h.agent_id, "llamacpp");
    assert_eq!(h.authenticated, None);
}

#[test]
fn secret_hygiene_never_leaks_tokens_into_health_snapshots() {
    use super::super::quota_cursor::CursorAuthDetails;

    let canary = "super_secret_canary_token_value_123456789";
    let auth = CursorAuthDetails {
        token_present: true,
        auth_expires_at: Some("2026-09-08T22:00:00Z".into()),
        is_expired: false,
        file_present: true,
    };
    let h = cursor_health_with(true, auth);
    let serialized = serde_json::to_string(&h).unwrap();
    assert!(!serialized.contains(canary));
    assert!(!h.detail.contains(canary));
    assert_eq!(h.auth_expires_at.as_deref(), Some("2026-09-08T22:00:00Z"));
}

#[test]
fn mistral_health_names_the_plan_and_never_leaks_customer_id() {
    use super::super::quota_vibe_usage::VibeWhoamiFacts;

    let logged_in = VibeWhoamiFacts {
        authenticated: true,
        cache_present: true,
        plan_name: Some("EDU".into()),
        plan_type: Some("chat".into()),
    };
    let h = mistral_health_with(true, logged_in);
    assert!(h.installed);
    assert_eq!(h.authenticated, Some(true));
    assert!(h.detail.contains("EDU plan"), "{}", h.detail);
    assert_eq!(h.endpoint_reachable, None);
    assert_eq!(h.agent_id, "mistral");

    let canary = "canary-customer-id-do-not-leak";
    let with_canary_plan = VibeWhoamiFacts {
        authenticated: true,
        cache_present: true,
        plan_name: Some("EDU".into()),
        plan_type: Some("chat".into()),
    };
    let serialized = serde_json::to_string(&mistral_health_with(true, with_canary_plan)).unwrap();
    assert!(!serialized.contains(canary));

    let missing = VibeWhoamiFacts {
        authenticated: false,
        cache_present: false,
        plan_name: None,
        plan_type: None,
    };
    let h2 = mistral_health_with(true, missing);
    assert_eq!(h2.authenticated, Some(false));
    assert!(h2.detail.contains("not logged in"), "{}", h2.detail);

    let unparseable = VibeWhoamiFacts {
        authenticated: false,
        cache_present: true,
        plan_name: None,
        plan_type: None,
    };
    let h3 = mistral_health_with(true, unparseable);
    assert_eq!(h3.authenticated, Some(false));
    assert!(h3.detail.contains("unparseable"), "{}", h3.detail);

    let login_no_bin = VibeWhoamiFacts {
        authenticated: true,
        cache_present: true,
        plan_name: Some("EDU".into()),
        plan_type: None,
    };
    let h4 = mistral_health_with(false, login_no_bin);
    assert!(!h4.installed);
    assert_eq!(h4.authenticated, Some(true));
    assert!(h4.detail.contains("not on PATH"), "{}", h4.detail);
}
