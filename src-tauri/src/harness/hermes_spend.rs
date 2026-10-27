//! Hermes Agent `--usage-file` spend for C6 / #84.
//!
//! Each file is one call's own figures, not a session-cumulative snapshot,
//! so the counts are recorded as-is (no delta). The quota reader still only
//! sums these files. `record_harness_turn_usage` always leaves `via_gateway`
//! unset, so recording calls `record_provider_usage_once` with the same key
//! dedup and sets the gateway from the report's `provider` string.

use super::spend::HarnessTurnUsage;
use hub::{HubStore, ProviderUsage, SliceStamp};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

/// One parsed `--usage-file` report. `turn` is the shared harness usage
/// shape; `via_gateway` is the report provider when it is not `hermes`.
pub(crate) struct HermesUsageTurn {
    pub turn: HarnessTurnUsage,
    pub via_gateway: Option<String>,
}

/// Reads one JSON file, or every `*.json` file in a directory.
///
/// `session_id` keeps reports whose JSON `session_id` is this session, or
/// whose file stem is the filesystem-safe session id. `None` keeps every
/// report that has a token bucket. A missing directory yields nothing.
pub(crate) fn hermes_usage_turns(path: &Path, session_id: Option<&str>) -> Vec<HermesUsageTurn> {
    json_paths(path)
        .iter()
        .filter_map(|path| report_from_file(path, session_id))
        .collect()
}

/// Records matching reports under `usage_dir`. A missing directory records
/// nothing and is not an error.
pub(crate) fn record_hermes_session_usage(
    store: &HubStore,
    hub_session_id: Option<&str>,
    usage_dir: &Path,
    session_id: &str,
) {
    for report in hermes_usage_turns(usage_dir, Some(session_id)) {
        record_report(store, hub_session_id, &report);
    }
}

fn record_report(store: &HubStore, hub_session_id: Option<&str>, report: &HermesUsageTurn) {
    let usage = ProviderUsage {
        provider: report.turn.provider.to_string(),
        actor_agent: report.turn.actor_agent.to_string(),
        prompt_tokens: report.turn.prompt_tokens,
        completion_tokens: report.turn.completion_tokens,
        cache_tokens: report.turn.cache_tokens,
        cost: None,
        via_gateway: report.via_gateway.clone(),
        model: report.turn.model.clone(),
        stamp: SliceStamp::registered(
            None,
            hub_session_id.map(str::to_string),
            Some(report.turn.actor_agent.to_string()),
        ),
    };
    let recorded = match report.turn.usage_key.as_deref() {
        Some(key) if !key.trim().is_empty() => store.record_provider_usage_once(&usage, key),
        _ => store.record_provider_usage(&usage),
    };
    let _ = recorded;
}

fn json_paths(path: &Path) -> Vec<PathBuf> {
    if path.is_file() {
        return vec![path.to_path_buf()];
    }
    let Ok(entries) = fs::read_dir(path) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("json")
        })
        .collect();
    files.sort();
    files
}

fn report_from_file(path: &Path, session_id: Option<&str>) -> Option<HermesUsageTurn> {
    let bytes = fs::read(path).ok()?;
    let value: Value = serde_json::from_slice(&bytes).ok()?;
    if !value.is_object() {
        return None;
    }
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .unwrap_or("unknown");
    if let Some(session_id) = session_id {
        if !belongs_to_session(&value, stem, session_id) {
            return None;
        }
    }
    let prompt_tokens = token_field(&value, "input_tokens");
    let completion_tokens = sum_fields(
        token_field(&value, "output_tokens"),
        token_field(&value, "reasoning_tokens"),
    );
    let cache_tokens = sum_fields(
        token_field(&value, "cache_read_tokens"),
        token_field(&value, "cache_write_tokens"),
    );
    if prompt_tokens.is_none() && completion_tokens.is_none() && cache_tokens.is_none() {
        return None;
    }
    let session = non_empty_str(&value, "session_id").unwrap_or_else(|| stem.to_string());
    Some(HermesUsageTurn {
        turn: HarnessTurnUsage {
            provider: "hermes",
            actor_agent: "hermes",
            text: String::new(),
            model: non_empty_str(&value, "model"),
            prompt_tokens,
            completion_tokens,
            cache_tokens,
            usage_key: Some(format!("{session}:{}", fnv1a64_hex(&bytes))),
        },
        via_gateway: via_gateway(&value),
    })
}

fn belongs_to_session(value: &Value, stem: &str, session_id: &str) -> bool {
    let safe = filesystem_safe_session_id(session_id);
    if !safe.is_empty() && stem == safe {
        return true;
    }
    let Some(json_id) = non_empty_str(value, "session_id") else {
        return false;
    };
    json_id == session_id.trim()
        || hub::hermes_disk_session_id(session_id).is_some_and(|disk| disk == json_id)
}

/// Same filename filter as `hermes_usage_file`: disk id, then alnum/`_`/`-`.
fn filesystem_safe_session_id(session_id: &str) -> String {
    let Some(id) = hub::hermes_disk_session_id(session_id) else {
        return String::new();
    };
    id.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .collect()
}

fn via_gateway(value: &Value) -> Option<String> {
    let provider = non_empty_str(value, "provider")?;
    (provider != "hermes").then_some(provider)
}

fn non_empty_str(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(|item| item.as_str())
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_string)
}

/// A missing key is `None`. A present non-negative integer that fits in
/// `u32` is `Some`, including 0. Negative, non-integer, or too big is `None`.
fn token_field(value: &Value, key: &str) -> Option<u32> {
    value.get(key).and_then(json_u32)
}

fn json_u32(value: &Value) -> Option<u32> {
    value.as_u64().and_then(|n| u32::try_from(n).ok())
}

fn sum_fields(left: Option<u32>, right: Option<u32>) -> Option<u32> {
    match (left, right) {
        (None, None) => None,
        (Some(left), None) | (None, Some(left)) => Some(left),
        (Some(left), Some(right)) => left.checked_add(right),
    }
}

fn fnv1a64_hex(bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    const SESSION: &str = "20260912_232548_916eae";
    const SAMPLE: &str = r#"{"estimated_cost_usd":0.0,"cost_status":"estimated","input_tokens":12356,"output_tokens":17,"cache_read_tokens":0,"cache_write_tokens":0,"reasoning_tokens":13,"total_tokens":12373,"api_calls":1,"model":"upstage/solar-pro4:free","provider":"nous","session_id":"20260912_232548_916eae","completed":true,"failed":false}"#;

    fn usage_rows(store: &HubStore) -> Vec<Value> {
        store
            .list_audit_events(false)
            .unwrap()
            .into_iter()
            .filter(|event| event.operation == "provider.usage")
            .map(|event| serde_json::from_str(&event.process_json).unwrap())
            .collect()
    }

    fn record(dir: &Path, session_id: &str) -> HubStore {
        let store = HubStore::open(tempdir().unwrap().path()).unwrap();
        record_hermes_session_usage(&store, Some("hub-hermes"), dir, session_id);
        store
    }

    #[test]
    fn documented_sample_records_tokens_model_and_gateway_not_cost() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join(format!("{SESSION}.json")), SAMPLE).unwrap();
        let store = record(dir.path(), SESSION);
        let rows = usage_rows(&store);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["provider"], "hermes");
        assert_eq!(rows[0]["actor_agent"], "hermes");
        assert_eq!(rows[0]["prompt_tokens"], 12356);
        assert_eq!(rows[0]["completion_tokens"], 30);
        assert_eq!(rows[0]["cache_tokens"], 0);
        assert_eq!(rows[0]["model"], "upstage/solar-pro4:free");
        assert_eq!(rows[0]["via_gateway"], "nous");
        assert_eq!(rows[0]["currency"], "unknown");
        assert!(rows[0].get("cost").is_none());
        let turns = hermes_usage_turns(&dir.path().join(format!("{SESSION}.json")), None);
        assert_eq!(turns.len(), 1);
        assert_eq!(
            turns[0].turn.usage_key.as_deref(),
            Some(format!("{SESSION}:{}", fnv1a64_hex(SAMPLE.as_bytes())).as_str())
        );
    }

    #[test]
    fn reread_of_the_same_bytes_does_not_add_a_row() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(format!("{SESSION}.json"));
        fs::write(&path, SAMPLE).unwrap();
        let store = HubStore::open(tempdir().unwrap().path()).unwrap();
        record_hermes_session_usage(&store, None, dir.path(), SESSION);
        record_hermes_session_usage(&store, None, dir.path(), SESSION);
        assert_eq!(usage_rows(&store).len(), 1);
    }

    #[test]
    fn replacing_the_file_adds_a_row_and_keeps_the_old_one() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(format!("{SESSION}.json"));
        fs::write(&path, SAMPLE).unwrap();
        let store = HubStore::open(tempdir().unwrap().path()).unwrap();
        record_hermes_session_usage(&store, None, dir.path(), SESSION);
        fs::write(
            &path,
            r#"{"input_tokens":50,"output_tokens":2,"reasoning_tokens":3,"cache_read_tokens":1,"cache_write_tokens":1,"model":"upstage/solar-pro4:free","provider":"nous","session_id":"20260912_232548_916eae","completed":true,"failed":false,"estimated_cost_usd":1.25}"#,
        )
        .unwrap();
        record_hermes_session_usage(&store, None, dir.path(), SESSION);
        let prompts: Vec<u64> = usage_rows(&store)
            .iter()
            .map(|row| row["prompt_tokens"].as_u64().unwrap())
            .collect();
        assert_eq!(prompts, vec![12356, 50]);
        assert!(usage_rows(&store)
            .iter()
            .all(|row| row.get("cost").is_none()));
    }

    #[test]
    fn missing_input_stays_absent_and_a_file_with_no_token_keys_writes_nothing() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join(format!("{SESSION}.json")),
            r#"{"output_tokens":4,"reasoning_tokens":1,"cache_write_tokens":2,"session_id":"20260912_232548_916eae","provider":"nous"}"#,
        )
        .unwrap();
        fs::write(
            dir.path().join("no-tokens.json"),
            r#"{"estimated_cost_usd":9.5,"api_calls":1,"total_tokens":10,"model":"m","provider":"nous","session_id":"20260912_232548_916eae"}"#,
        )
        .unwrap();
        let rows = usage_rows(&record(dir.path(), SESSION));
        assert_eq!(rows.len(), 1);
        assert!(rows[0].get("prompt_tokens").is_none());
        assert_eq!(rows[0]["completion_tokens"], 5);
        assert_eq!(rows[0]["cache_tokens"], 2);
        assert!(rows[0].get("cost").is_none());
    }

    #[test]
    fn failed_report_still_records_tokens_and_estimated_dollars_are_not_cost() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("oneshot-failed.json"),
            r#"{"estimated_cost_usd":4.25,"cost_status":"estimated","input_tokens":3,"output_tokens":4,"reasoning_tokens":5,"failed":true,"completed":false,"model":"m","provider":"nous","session_id":"20260912_232548_916eae"}"#,
        )
        .unwrap();
        let rows = usage_rows(&record(dir.path(), SESSION));
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["prompt_tokens"], 3);
        assert_eq!(rows[0]["completion_tokens"], 9);
        assert!(rows[0].get("cache_tokens").is_none());
        assert!(rows[0].get("cost").is_none());
        assert_eq!(rows[0]["currency"], "unknown");
        assert_eq!(rows[0]["via_gateway"], "nous");
    }

    #[test]
    fn another_sessions_report_is_not_recorded_for_this_session() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("other-session.json"),
            r#"{"input_tokens":99,"provider":"nous","session_id":"other-session"}"#,
        )
        .unwrap();
        fs::write(
            dir.path().join("oneshot-1.json"),
            r#"{"input_tokens":7,"provider":"nous","session_id":"20260912_232548_916eae"}"#,
        )
        .unwrap();
        fs::write(
            dir.path().join(format!("{SESSION}.json")),
            r#"{"input_tokens":8,"provider":"nous","session_id":"not-this"}"#,
        )
        .unwrap();
        let prompts: Vec<u64> = usage_rows(&record(dir.path(), SESSION))
            .iter()
            .map(|row| row["prompt_tokens"].as_u64().unwrap())
            .collect();
        assert_eq!(prompts, vec![8, 7]);
    }

    #[test]
    fn invalid_numbers_stay_absent_and_an_overflowing_sum_is_absent() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("edge.json");
        fs::write(
            &path,
            r#"{"input_tokens":-1,"output_tokens":4294967295,"reasoning_tokens":1,"cache_read_tokens":1.5,"cache_write_tokens":0,"provider":"hermes","session_id":"s"}"#,
        )
        .unwrap();
        let turns = hermes_usage_turns(&path, None);
        assert_eq!(turns.len(), 1);
        assert_eq!(turns[0].turn.prompt_tokens, None);
        assert_eq!(turns[0].turn.completion_tokens, None);
        assert_eq!(turns[0].turn.cache_tokens, Some(0));
        assert_eq!(turns[0].via_gateway, None);
        assert_eq!(turns[0].turn.provider, "hermes");
    }

    #[test]
    fn a_missing_usage_directory_records_nothing() {
        let dir = tempdir().unwrap();
        let missing = dir.path().join("no-such-hermes-usage");
        let store = HubStore::open(dir.path().join("hub")).unwrap();
        record_hermes_session_usage(&store, None, &missing, SESSION);
        assert!(usage_rows(&store).is_empty());
    }
}
