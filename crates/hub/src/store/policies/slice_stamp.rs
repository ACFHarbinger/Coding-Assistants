//! Slice stamps for Team Scorecard T1 (#341).
//!
//! Nullable `task_id` / `work_session_id` / `attribution` live in
//! `audit_events.process_json`, not as new columns. D4 already parses
//! `task_id`, `session_id`, and `attribution` from that JSON.

use super::super::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

const KNOWN: &[&str] = &["registered", "observed", "external", "inferred", "unknown"];

/// Attribution basis beside a slice stamp. Unknown is the honest default
/// when a writer has no identity confidence — never guess `registered`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Attribution {
    Registered,
    Observed,
    External,
    Inferred,
    #[default]
    Unknown,
}

impl Attribution {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Registered => "registered",
            Self::Observed => "observed",
            Self::External => "external",
            Self::Inferred => "inferred",
            Self::Unknown => "unknown",
        }
    }

    /// Invalid or empty values become [`Self::Unknown`]. Never errors.
    pub fn parse(raw: Option<&str>) -> Self {
        match raw.map(str::trim).filter(|value| !value.is_empty()) {
            Some("registered") => Self::Registered,
            Some("observed") => Self::Observed,
            Some("external") => Self::External,
            Some("inferred") => Self::Inferred,
            _ => Self::Unknown,
        }
    }
}

/// Per-row slice identity written into `process_json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SliceStamp {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub work_session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    #[serde(default)]
    pub attribution: Attribution,
}

impl SliceStamp {
    pub fn registered(
        task_id: Option<String>,
        work_session_id: Option<String>,
        agent: Option<String>,
    ) -> Self {
        Self {
            task_id: empty_to_none(task_id),
            work_session_id: empty_to_none(work_session_id),
            agent: empty_to_none(agent),
            attribution: Attribution::Registered,
        }
    }

    pub fn from_json_str(process_json: &str) -> Self {
        serde_json::from_str::<Value>(process_json)
            .ok()
            .map(|value| Self::from_json(&value))
            .unwrap_or_default()
    }

    pub fn from_json(value: &Value) -> Self {
        let session =
            string_field(value, "work_session_id").or_else(|| string_field(value, "session_id"));
        Self {
            task_id: string_field(value, "task_id"),
            work_session_id: session,
            agent: string_field(value, "agent"),
            attribution: Attribution::parse(value.get("attribution").and_then(Value::as_str)),
        }
    }

    /// Merge stamps into an existing process object. Writes `session_id` as
    /// an alias of `work_session_id` so D4's `ProcessJsonMeta` matches.
    pub fn apply(&self, value: &mut Value) {
        let Some(object) = value.as_object_mut() else {
            return;
        };
        if let Some(task_id) = self.task_id.as_deref().filter(|id| !id.is_empty()) {
            object.insert("task_id".into(), json!(task_id));
        }
        if let Some(session) = self.work_session_id.as_deref().filter(|id| !id.is_empty()) {
            object.insert("work_session_id".into(), json!(session));
            object.insert("session_id".into(), json!(session));
        }
        if let Some(agent) = self.agent.as_deref().filter(|id| !id.is_empty()) {
            object.insert("agent".into(), json!(agent));
        }
        object.insert("attribution".into(), json!(self.attribution.as_str()));
    }
}

fn empty_to_none(value: Option<String>) -> Option<String> {
    value.and_then(|raw| {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn string_field(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|raw| !raw.is_empty())
        .map(str::to_string)
}

impl HubStore {
    /// Write one hash-chained audit row whose `process_json` carries `stamp`.
    pub fn record_stamped_audit(
        &self,
        root: &Path,
        path: &Path,
        operation: &str,
        mut process: Value,
        stamp: &SliceStamp,
    ) -> Result<AuditEvent, HubError> {
        stamp.apply(&mut process);
        self.record_audit_event(root, path, operation, &process.to_string(), None)
    }

    /// One `tagged.send` row per send, not per recipient.
    pub fn record_tagged_send_audit(
        &self,
        from_agent: &str,
        subject: &str,
        workspace_path: Option<&str>,
        task_id: Option<&str>,
        session_id: Option<&str>,
        recipient_count: usize,
    ) -> Result<AuditEvent, HubError> {
        let stamp = SliceStamp::registered(
            task_id.map(str::to_string),
            session_id.map(str::to_string),
            Some(from_agent.to_string()),
        );
        let root = workspace_path
            .map(PathBuf::from)
            .unwrap_or_else(|| self.data_dir.clone());
        self.record_stamped_audit(
            &root,
            &root,
            "tagged.send",
            json!({
                "kind": "tagged.send",
                "subject": subject,
                "from_agent": from_agent,
                "recipient_count": recipient_count,
            }),
            &stamp,
        )
    }

    pub fn known_attribution_values() -> &'static [&'static str] {
        KNOWN
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_falls_back_to_unknown() {
        assert_eq!(Attribution::parse(None), Attribution::Unknown);
        assert_eq!(Attribution::parse(Some("")), Attribution::Unknown);
        assert_eq!(Attribution::parse(Some("guessed")), Attribution::Unknown);
        assert_eq!(
            Attribution::parse(Some("registered")),
            Attribution::Registered
        );
    }

    #[test]
    fn apply_writes_session_alias_and_skips_empty_ids() {
        let stamp = SliceStamp::registered(
            Some("task-1".into()),
            Some("sess-9".into()),
            Some("cursor".into()),
        );
        let mut value = json!({"kind": "os_tool"});
        stamp.apply(&mut value);
        assert_eq!(value["task_id"], "task-1");
        assert_eq!(value["work_session_id"], "sess-9");
        assert_eq!(value["session_id"], "sess-9");
        assert_eq!(value["agent"], "cursor");
        assert_eq!(value["attribution"], "registered");

        let empty = SliceStamp::default();
        let mut blank = json!({"kind": "x"});
        empty.apply(&mut blank);
        assert!(blank.get("task_id").is_none());
        assert_eq!(blank["attribution"], "unknown");
    }

    #[test]
    fn from_json_prefers_work_session_id() {
        let stamp = SliceStamp::from_json_str(
            r#"{"task_id":"t","work_session_id":"w","session_id":"s","attribution":"observed"}"#,
        );
        assert_eq!(stamp.task_id.as_deref(), Some("t"));
        assert_eq!(stamp.work_session_id.as_deref(), Some("w"));
        assert_eq!(stamp.attribution, Attribution::Observed);
    }
}
