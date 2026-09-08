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
