//! `quota.snapshot` audit rows — the balance side of the T7 quota probe.
//!
//! Written best-effort on explicit provider-quota refresh (never on the
//! 60-second poll: an audit row per poll would grow the hash chain for no
//! new information). Snapshots dedupe against the provider's latest row, so
//! unchanged balances write nothing. This is `estimated`-grade context for
//! the scorecard — the `measured` cost figure remains `provider.usage`.

use super::super::*;
use serde_json::{json, Value};

#[derive(Debug, Clone)]
pub struct QuotaSnapshot {
    pub provider: String,
    /// Account balance in the provider's currency (e.g. DeepSeek dollars).
    pub balance: Option<f64>,
    /// Primary percent window, for providers without a balance.
    pub used_percent: Option<i32>,
    pub window_label: Option<String>,
}

impl HubStore {
    /// One snapshot row per refresh, and none when nothing was measured or
    /// nothing changed since the provider's previous snapshot.
    pub fn record_quota_snapshot(
        &self,
        snapshot: &QuotaSnapshot,
    ) -> Result<Option<AuditEvent>, HubError> {
        let provider = snapshot.provider.trim();
        if provider.is_empty() {
            return Err(HubError::Invalid("provider is required".into()));
        }
        if snapshot.balance.is_none() && snapshot.used_percent.is_none() {
            return Ok(None);
        }
        if let Some(balance) = snapshot.balance {
            if !balance.is_finite() || balance < 0.0 {
                return Err(HubError::Invalid(format!(
                    "balance must be a finite non-negative number, got {balance}"
                )));
            }
        }

        let mut stmt = self.conn.prepare(
            "SELECT process_json FROM audit_events WHERE operation = 'quota.snapshot' ORDER BY rowid DESC",
        )?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        for raw in rows.flatten() {
            let Ok(value) = serde_json::from_str::<Value>(&raw) else {
                continue;
            };
            if value.get("provider").and_then(Value::as_str) != Some(provider) {
                continue;
            }
            let same_balance = value.get("balance").and_then(Value::as_f64) == snapshot.balance;
            let same_percent = value.get("used_percent").and_then(Value::as_i64)
                == snapshot.used_percent.map(i64::from);
            if same_balance && same_percent {
                return Ok(None);
            }
            break;
        }

        let mut process = json!({
            "kind": "quota.snapshot",
            "provider": provider,
        });
        let object = process.as_object_mut().expect("object");
        if let Some(balance) = snapshot.balance {
            object.insert("balance".into(), json!(balance));
        }
        if let Some(percent) = snapshot.used_percent {
            object.insert("used_percent".into(), json!(percent));
        }
        if let Some(label) = snapshot
            .window_label
            .as_deref()
            .map(str::trim)
            .filter(|l| !l.is_empty())
        {
            object.insert("window_label".into(), json!(label));
        }
        let root = self.data_dir.clone();
        Ok(Some(self.record_audit_event(
            &root,
            &root,
            "quota.snapshot",
            &process.to_string(),
            None,
        )?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn skips_when_nothing_measured_and_dedupes_unchanged() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let none = QuotaSnapshot {
            provider: "deepseek".into(),
            balance: None,
            used_percent: None,
            window_label: None,
        };
        assert!(store.record_quota_snapshot(&none).unwrap().is_none());

        let snap = QuotaSnapshot {
            provider: "deepseek".into(),
            balance: Some(12.34),
            used_percent: None,
            window_label: None,
        };
        let first = store.record_quota_snapshot(&snap).unwrap().unwrap();
        assert_eq!(first.operation, "quota.snapshot");
        assert!(store.record_quota_snapshot(&snap).unwrap().is_none());

        let lower = QuotaSnapshot {
            provider: "deepseek".into(),
            balance: Some(10.0),
            used_percent: None,
            window_label: None,
        };
        assert!(store.record_quota_snapshot(&lower).unwrap().is_some());
        assert_eq!(store.list_audit_events(false).unwrap().len(), 2);
    }

    #[test]
    fn rejects_negative_balance() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let bad = QuotaSnapshot {
            provider: "deepseek".into(),
            balance: Some(-1.0),
            used_percent: None,
            window_label: None,
        };
        assert!(store.record_quota_snapshot(&bad).is_err());
    }
}
