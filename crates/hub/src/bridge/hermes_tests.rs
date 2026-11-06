use super::*;

const EXPORT_LINE: &str = r#"{"id":"20260912_232548_916eae","cwd":"/tmp/hermes-spike","started_at":"20260912_232548_916eae","last_activity_at":"1789251950.7676367","messages":[]}"#;

#[test]
fn latest_session_prefers_workspace_match_then_recency() {
    let other = r#"{"id":"20260911_000000_aaaaaa","cwd":"/other","started_at":"20260911_000000_aaaaaa","last_activity_at":"1789165000.0","messages":[]}"#;
    let payload = format!("{other}\n{EXPORT_LINE}\nnot json\n");
    let found = latest_session_from_export(&payload, Some(Path::new("/tmp/hermes-spike")));
    assert_eq!(found, Some("20260912_232548_916eae".to_string()));

    // No workspace match anywhere: newest overall wins.
    let found = latest_session_from_export(&payload, Some(Path::new("/nowhere")));
    assert_eq!(found, Some("20260912_232548_916eae".to_string()));

    assert_eq!(latest_session_from_export("not json\n", None), None);
    assert_eq!(latest_session_from_export("", None), None);
}

#[test]
fn hermes_timestamps_parse_both_live_shapes() {
    assert_eq!(parse_hermes_time("1789251950.7676367"), Some(1789251950));
    // Fixed-width stamp: lexicographic order is chronological.
    let early = parse_hermes_time("20260911_000000_aaaaaa").unwrap();
    let late = parse_hermes_time("20260912_232548_916eae").unwrap();
    assert!(late > early);
    assert_eq!(parse_hermes_time(""), None);
    assert_eq!(parse_hermes_time("yesterday"), None);
}

#[test]
fn usage_reports_resolve_newest_session_after_a_mark() {
    let _guard = crate::CA_HOME_LOCK.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let previous = std::env::var_os("CA_HOME");
    std::env::set_var("CA_HOME", dir.path());

    let usage_dir = crate::harness::hermes_usage_dir();
    std::fs::create_dir_all(&usage_dir).unwrap();
    // An older report must not shadow the fresh one.
    std::fs::write(
        usage_dir.join("old.json"),
        r#"{"session_id":"20260911_000000_aaaaaa"}"#,
    )
    .unwrap();
    // Coarse mtimes can tie across a fast test: separate the mark from both
    // writes so old < mark < fresh is strict.
    std::thread::sleep(std::time::Duration::from_millis(25));
    let since = SystemTime::now();
    std::thread::sleep(std::time::Duration::from_millis(25));
    std::fs::write(
        usage_dir.join("fresh.json"),
        r#"{"session_id":"20260912_232548_916eae"}"#,
    )
    .unwrap();

    assert_eq!(
        latest_usage_session_id(since),
        Some("20260912_232548_916eae".to_string())
    );
    // Nothing newer than now-plus-a-bit: None, not a stale id.
    assert_eq!(
        latest_usage_session_id(since + std::time::Duration::from_secs(3600)),
        None
    );

    match previous {
        Some(value) => std::env::set_var("CA_HOME", value),
        None => std::env::remove_var("CA_HOME"),
    }
}
