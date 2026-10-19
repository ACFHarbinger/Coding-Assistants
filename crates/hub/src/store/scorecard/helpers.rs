//! Query helpers and predicates for Team Scorecard (T3 / #343).

use super::types::{MeasurementCoverage, ScorecardFigure};
use serde_json::Value;

pub const KNOWN_HARNESSES: &[&str] = &[
    "claude", "cursor", "gemini", "grok", "opencode", "deepseek", "vibe", "muse", "qwen", "kimi",
    "hermes", "chat",
];

pub fn is_security_class_op(op: &str) -> bool {
    op.starts_with("os_tool.")
        || op.starts_with("vault.")
        || op.starts_with("secret.")
        || op == "tcp.auth_rejected"
        || op == "budget.stopped"
        || op.starts_with("oversight.")
        || op.starts_with("mcp.invoke")
}

pub fn is_open_security_event(status: &str, op: &str) -> bool {
    if status == "approved" || status == "resolved" {
        return false;
    }
    match op {
        "os_tool.denied" | "tcp.auth_rejected" | "budget.stopped" => true,
        "os_tool.proposed" => status == "pending" || status == "quarantined",
        _ => status == "quarantined",
    }
}

#[derive(Default)]
pub struct UsageAccumulator {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub has_tokens: bool,
    pub cost: f64,
    pub has_cost: bool,
    pub request_count: u64,
    pub token_measurements: u64,
    pub cost_measurements: u64,
}

impl UsageAccumulator {
    pub fn observe(&mut self, row: &Value) {
        let prompt = row.get("prompt_tokens").and_then(Value::as_u64);
        let completion = row.get("completion_tokens").and_then(Value::as_u64);
        self.prompt_tokens = self.prompt_tokens.saturating_add(prompt.unwrap_or(0));
        self.completion_tokens = self
            .completion_tokens
            .saturating_add(completion.unwrap_or(0));
        self.has_tokens |= prompt.is_some() || completion.is_some();
        self.token_measurements += u64::from(prompt.is_some() && completion.is_some());
        if let Some(cost) = row
            .get("cost")
            .and_then(Value::as_f64)
            .filter(|c| c.is_finite() && *c >= 0.0)
        {
            self.cost += cost;
            self.has_cost = true;
            self.cost_measurements += 1;
        }
        self.request_count += 1;
    }

    pub fn tokens_figure(&self) -> ScorecardFigure {
        self.figure(
            self.has_tokens,
            self.prompt_tokens as f64 + self.completion_tokens as f64,
            "tokens",
            self.token_measurements,
        )
    }

    pub fn cost_figure(&self) -> ScorecardFigure {
        self.figure(self.has_cost, self.cost, "currency", self.cost_measurements)
    }

    fn figure(&self, present: bool, value: f64, unit: &str, measured: u64) -> ScorecardFigure {
        let mut figure = if present {
            ScorecardFigure::measured(value, unit)
        } else {
            ScorecardFigure::unknown(unit)
        };
        figure.coverage = Some(MeasurementCoverage {
            measured_requests: measured,
            total_requests: self.request_count,
        });
        figure
    }
}
