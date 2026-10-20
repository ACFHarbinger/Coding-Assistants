use super::*;

/// Trimmed but verbatim-shaped capture of a real `POST /v1/responses`
/// stream (2026-09-09): a couple of lifecycle frames, then the tail
/// `response.subscription_usage` frame this adapter reads.
const LIVE_STREAM: &str = "event: response.created\n\
data: {\"response\":{\"id\":\"resp_x\",\"status\":\"in_progress\"},\"sequence_number\":0,\"type\":\"response.created\"}\n\
\n\
event: response.incomplete\n\
data: {\"response\":{\"id\":\"resp_x\",\"status\":\"incomplete\",\"usage\":{\"input_tokens\":8,\"output_tokens\":16,\"total_tokens\":24}},\"sequence_number\":3,\"type\":\"response.incomplete\"}\n\
\n\
event: response.subscription_usage\n\
data: {\"subscription\":{\"tier\":\"27681527378179523\",\"weekly\":{\"resets_at\":1789344000,\"used_percent\":35},\"window\":{\"resets_at\":1788962220,\"used_percent\":5,\"window_duration_mins\":300}},\"type\":\"response.subscription_usage\"}\n";

#[test]
fn metered_probe_off_short_circuits_before_any_request() {
    let quota = muse_quota(false);
    assert_eq!(quota.status, "unavailable");
    assert_eq!(quota.agent_id, AGENT_ID);
    let detail = quota.detail.unwrap();
    assert!(detail.contains("Allow metered usage probes"), "{detail}");
    assert!(quota.windows.is_empty());
}

#[test]
fn api_creds_parse_from_auth_json_with_base_url_fallback() {
    let raw = r#"{ "schema_version": 1, "providers": { "meta": {
        "api_key": "  LLM|123|abc  ",
        "api_base_url": "https://api.meta.ai/v1/",
        "access_token": "dca:opaque" } } }"#;
    let (key, base) = muse_api_creds_from(raw).expect("valid creds");
    assert_eq!(key, "LLM|123|abc");
    assert_eq!(base, "https://api.meta.ai/v1");

    // Missing / blank base_url -> documented default.
    let no_base = r#"{ "providers": { "meta": { "api_key": "LLM|k" } } }"#;
    assert_eq!(
        muse_api_creds_from(no_base).unwrap().1,
        "https://api.meta.ai/v1"
    );

    // A non-https base_url is not trusted.
    let http_base =
        r#"{ "providers": { "meta": { "api_key": "LLM|k", "api_base_url": "http://evil" } } }"#;
    assert_eq!(
        muse_api_creds_from(http_base).unwrap().1,
        "https://api.meta.ai/v1"
    );
}

#[test]
fn api_creds_rejected_without_a_key() {
    for raw in [
        "{}",
        r#"{ "providers": {} }"#,
        r#"{ "providers": { "meta": {} } }"#,
        r#"{ "providers": { "meta": { "api_key": "   " } } }"#,
        r#"{ "providers": { "meta": { "api_key": 42 } } }"#,
        "not json",
    ] {
        assert!(muse_api_creds_from(raw).is_none(), "should reject: {raw}");
    }
}

#[test]
fn subscription_extracted_from_the_sse_tail() {
    let subscription = subscription_from_stream(LIVE_STREAM).expect("frame present");
    assert_eq!(
        subscription.get("tier").and_then(|v| v.as_str()),
        Some("27681527378179523")
    );
    assert_eq!(subscription["weekly"]["used_percent"], 35);

    // A stream that never carries the frame yields nothing (caller -> unavailable).
    let only_lifecycle = "event: response.created\ndata: {\"type\":\"response.created\"}\n";
    assert!(subscription_from_stream(only_lifecycle).is_none());
    // An auth-error body is not an SSE stream at all.
    assert!(subscription_from_stream(r#"{"error":{"code":"invalid_api_key"}}"#).is_none());
}

#[test]
fn windows_cover_weekly_and_rolling() {
    let subscription = subscription_from_stream(LIVE_STREAM).unwrap();
    let windows = windows_from_subscription(&subscription);
    assert_eq!(windows.len(), 2);

    assert_eq!(windows[0].label, "Weekly limit");
    assert_eq!(windows[0].used_percent, 35);
    assert_eq!(windows[0].remaining_percent, 65);
    assert_eq!(windows[0].resets_at, Some(1_789_344_000));
    assert_eq!(windows[0].window_minutes, Some(7 * 24 * 60));
    assert_eq!(windows[0].family.as_deref(), Some("Muse subscription"));

    assert_eq!(windows[1].label, "5-hour rolling limit");
    assert_eq!(windows[1].used_percent, 5);
    assert_eq!(windows[1].remaining_percent, 95);
    assert_eq!(windows[1].resets_at, Some(1_788_962_220));
    assert_eq!(windows[1].window_minutes, Some(300));
}

#[test]
fn rolling_label_handles_odd_and_missing_durations() {
    assert_eq!(rolling_label(Some(300)), "5-hour rolling limit");
    assert_eq!(rolling_label(Some(90)), "90-minute rolling limit");
    assert_eq!(rolling_label(None), "Rolling limit");
    assert_eq!(rolling_label(Some(0)), "Rolling limit");
}

#[test]
fn quota_from_subscription_is_ok_with_windows_and_unavailable_without() {
    let subscription = subscription_from_stream(LIVE_STREAM).unwrap();
    let quota = muse_quota_from_subscription(&subscription);
    assert_eq!(quota.agent_id, "muse");
    assert_eq!(quota.provider, "muse");
    assert_eq!(quota.status, "ok");
    assert!(quota.detail.is_none());
    assert_eq!(quota.balance, None);
    assert_eq!(quota.windows.len(), 2);

    let empty = muse_quota_from_subscription(&serde_json::json!({ "tier": "x" }));
    assert_eq!(empty.status, "unavailable");
    assert!(empty.windows.is_empty());
    assert!(empty
        .detail
        .as_deref()
        .unwrap_or_default()
        .contains("no weekly or rolling window"));
}

#[test]
fn partial_subscription_still_yields_the_window_it_has() {
    let weekly_only = serde_json::json!({ "weekly": { "used_percent": 12.6 } });
    let windows = windows_from_subscription(&weekly_only);
    assert_eq!(windows.len(), 1);
    assert_eq!(windows[0].label, "Weekly limit");
    assert_eq!(windows[0].used_percent, 13); // rounded
    assert_eq!(windows[0].resets_at, None);
}
