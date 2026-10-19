//! Per-request `provider.usage` audit rows (Team Scorecard T1 / #341).
//!
//! This is the measured cost source. Account-level quota deltas must not
//! be written here. Missing token or currency figures stay absent so the
//! scorecard can read `unknown`, never zero.

use super::super::*;
use super::slice_stamp::SliceStamp;
use serde_json::json;

/// One provider response's measured usage. Tokens now; currency only when
/// the adapter reported a cost (OpenRouter `usage.cost`).
#[derive(Debug, Clone)]
pub struct ProviderUsage {
    pub provider: String,
    pub actor_agent: String,
    pub prompt_tokens: Option<u32>,
    pub completion_tokens: Option<u32>,
    pub cache_tokens: Option<u32>,
    pub cost: Option<f64>,
    pub via_gateway: Option<String>,
    pub model: Option<String>,
    pub stamp: SliceStamp,
}

impl ProviderUsage {
    pub fn has_measurement(&self) -> bool {
        self.prompt_tokens.is_some()
            || self.completion_tokens.is_some()
            || self.cache_tokens.is_some()
            || self.cost.is_some()
    }

    pub fn currency_status(&self) -> &'static str {
        if self.cost.is_some() {
            "measured"
        } else {
            "unknown"
        }
    }

    /// OpenRouter (and similar) models look like `openai/gpt-4o-mini`.
    /// The upstream owner is the first path segment; the gateway is cited
    /// separately so rows are never silently joined.
    pub fn gateway_route(gateway: &str, model: &str) -> (String, Option<String>) {
        match model.split_once('/') {
            Some((upstream, _)) if !upstream.trim().is_empty() => (
                upstream.trim().to_string(),
                Some(gateway.trim().to_string()),
            ),
            _ => ("unknown".to_string(), Some(gateway.trim().to_string())),
        }
    }
}

impl HubStore {
    /// No-op when nothing was measured, so CLI/harness paths without usage
    /// do not invent empty cost rows.
    pub fn record_provider_usage(
        &self,
        usage: &ProviderUsage,
    ) -> Result<Option<AuditEvent>, HubError> {
        if !usage.has_measurement() {
            return Ok(None);
        }
        let mut process = json!({
            "kind": "provider.usage",
            "provider": usage.provider,
            "actor_agent": usage.actor_agent,
            "currency": usage.currency_status(),
        });
        let object = process.as_object_mut().expect("object");
        if let Some(tokens) = usage.prompt_tokens {
            object.insert("prompt_tokens".into(), json!(tokens));
        }
        if let Some(tokens) = usage.completion_tokens {
            object.insert("completion_tokens".into(), json!(tokens));
        }
        if let Some(tokens) = usage.cache_tokens {
            object.insert("cache_tokens".into(), json!(tokens));
        }
        if let Some(cost) = usage.cost {
            object.insert("cost".into(), json!(cost));
        }
        if let Some(gateway) = usage.via_gateway.as_deref() {
            object.insert("via_gateway".into(), json!(gateway));
        }
        if let Some(model) = usage.model.as_deref() {
            object.insert("model".into(), json!(model));
        }
        let root = self.data_dir.clone();
        Ok(Some(self.record_stamped_audit(
            &root,
            &root,
            "provider.usage",
            process,
            &usage.stamp,
        )?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn gateway_route_splits_upstream() {
        let (provider, via) = ProviderUsage::gateway_route("openrouter", "openai/gpt-4o-mini");
        assert_eq!(provider, "openai");
        assert_eq!(via.as_deref(), Some("openrouter"));
        let (plain, via) = ProviderUsage::gateway_route("openrouter", "gpt-4o");
        assert_eq!(plain, "unknown");
        assert_eq!(via.as_deref(), Some("openrouter"));
    }

    #[test]
    fn skips_when_nothing_was_measured() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let usage = ProviderUsage {
            provider: "openai".into(),
            actor_agent: "cursor".into(),
            prompt_tokens: None,
            completion_tokens: None,
            cache_tokens: None,
            cost: None,
            via_gateway: None,
            model: None,
            stamp: SliceStamp::default(),
        };
        assert!(store.record_provider_usage(&usage).unwrap().is_none());
        assert!(store.list_audit_events(false).unwrap().is_empty());
    }

    #[test]
    fn writes_measured_cost_and_stamps() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let usage = ProviderUsage {
            provider: "openai".into(),
            actor_agent: "cursor".into(),
            prompt_tokens: Some(3),
            completion_tokens: Some(4),
            cache_tokens: None,
            cost: Some(0.0015),
            via_gateway: Some("openrouter".into()),
            model: Some("openai/gpt-4o-mini".into()),
            stamp: SliceStamp::registered(
                Some("task-1".into()),
                Some("sess-1".into()),
                Some("cursor".into()),
            ),
        };
        let event = store.record_provider_usage(&usage).unwrap().unwrap();
        assert_eq!(event.operation, "provider.usage");
        let json: serde_json::Value = serde_json::from_str(&event.process_json).unwrap();
        assert_eq!(json["provider"], "openai");
        assert_eq!(json["via_gateway"], "openrouter");
        assert_eq!(json["currency"], "measured");
        assert_eq!(json["prompt_tokens"], 3);
        assert_eq!(json["cost"], 0.0015);
        assert_eq!(json["task_id"], "task-1");
        assert_eq!(json["session_id"], "sess-1");
        assert_eq!(json["attribution"], "registered");
        let round = SliceStamp::from_json_str(&event.process_json);
        assert_eq!(round.task_id.as_deref(), Some("task-1"));
        assert_eq!(round.work_session_id.as_deref(), Some("sess-1"));
    }
}
