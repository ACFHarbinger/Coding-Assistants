//! Query helpers and predicates for Team Scorecard (T3 / #343).

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
}
