//! Opt-in prior-context filters for handoff seeding (P17 / #361).
//!
//! Grammar is a comma-separated list applied left to right:
//! `strip_tools`, `cap:<n>` (1..=10000 lines), `summarize`.
//! Callers that have no filter do not call this module, so the unfiltered
//! handoff body stays byte-for-byte what it was before this slice.
//! `summarize` is a deterministic digest (counts plus first and last
//! lines). It does not call a model.

use super::*;

pub(crate) fn validate_context_filter(spec: &str) -> Result<(), HubError> {
    parse_steps(spec.trim())?;
    Ok(())
}

pub(crate) fn apply_context_filter(prior: &str, spec: &str) -> Result<String, HubError> {
    let mut text = prior.to_string();
    for step in parse_steps(spec.trim())? {
        text = match step {
            Step::StripTools => strip_tools(&text),
            Step::Cap(n) => cap_lines(&text, n),
            Step::Summarize => summarize(&text),
        };
    }
    Ok(text)
}

/// Audit-only object. `reason` and `priority` are the only keys, matching
/// the structured input idea without becoming a routing channel.
pub(crate) fn validate_handoff_metadata(value: &serde_json::Value) -> Result<(), HubError> {
    let obj = value.as_object().ok_or_else(|| {
        HubError::Invalid(
            "handoff_metadata must be a JSON object with reason and/or priority".into(),
        )
    })?;
    if obj.is_empty() {
        return Err(HubError::Invalid(
            "handoff_metadata must include reason and/or priority".into(),
        ));
    }
    for (key, val) in obj {
        match key.as_str() {
            "reason" => {
                if val.as_str().is_none_or(|s| s.trim().is_empty()) {
                    return Err(HubError::Invalid(
                        "handoff_metadata.reason must be a non-empty string".into(),
                    ));
                }
            }
            "priority" => {
                let ok = val.is_number() || val.as_str().is_some_and(|s| !s.trim().is_empty());
                if !ok {
                    return Err(HubError::Invalid(
                        "handoff_metadata.priority must be a string or number".into(),
                    ));
                }
            }
            other => {
                return Err(HubError::Invalid(format!(
                    "handoff_metadata key '{other}' is not reason or priority"
                )));
            }
        }
    }
    Ok(())
}

enum Step {
    StripTools,
    Cap(usize),
    Summarize,
}

fn parse_steps(spec: &str) -> Result<Vec<Step>, HubError> {
    if spec.is_empty() {
        return Err(HubError::Invalid(
            "context_filter must be strip_tools, cap:<n>, summarize, or a comma-separated list"
                .into(),
        ));
    }
    let mut steps = Vec::new();
    for raw in spec.split(',') {
        let token = raw.trim();
        if token.is_empty() {
            return Err(HubError::Invalid("context_filter has an empty step".into()));
        }
        if token == "strip_tools" {
            steps.push(Step::StripTools);
        } else if token == "summarize" {
            steps.push(Step::Summarize);
        } else if let Some(n) = token.strip_prefix("cap:") {
            let n: usize = n.parse().map_err(|_| {
                HubError::Invalid(format!(
                    "context_filter cap must be a positive integer, got {token}"
                ))
            })?;
            if n == 0 || n > 10_000 {
                return Err(HubError::Invalid(
                    "context_filter cap must be from 1 to 10000".into(),
                ));
            }
            steps.push(Step::Cap(n));
        } else {
            return Err(HubError::Invalid(format!(
                "unknown context_filter step '{token}' (expected strip_tools, cap:<n>, summarize)"
            )));
        }
    }
    Ok(steps)
}

fn strip_tools(text: &str) -> String {
    text.lines()
        .filter(|line| !is_tool_noise(line))
        .collect::<Vec<_>>()
        .join("\n")
}

fn is_tool_noise(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return false;
    }
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("tool:")
        || lower.starts_with("tool_call")
        || lower.starts_with("function_call")
        || lower.starts_with("[tool]")
        || lower.starts_with("tool result:")
    {
        return true;
    }
    const NEEDLES: &[&str] = &[
        "\"tool_calls\"",
        "\"tool_use\"",
        "\"tool_result\"",
        "\"function_call\"",
        "\"function_calls\"",
    ];
    if NEEDLES.iter().any(|needle| lower.contains(needle)) {
        return true;
    }
    let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) else {
        return false;
    };
    if value.get("tool_calls").is_some()
        || value.get("tool_use").is_some()
        || value.get("tool_result").is_some()
        || value.get("function_call").is_some()
    {
        return true;
    }
    value
        .get("type")
        .and_then(|kind| kind.as_str())
        .is_some_and(is_tool_type)
}

fn is_tool_type(kind: &str) -> bool {
    matches!(
        kind,
        "tool_use"
            | "tool_result"
            | "function_call"
            | "function_call_output"
            | "VIEW_FILE"
            | "LIST_DIR"
            | "RUN_COMMAND"
            | "REPLACE_FILE_CONTENT"
            | "WRITE_TO_FILE"
            | "MULTI_REPLACE_FILE_CONTENT"
            | "READ_URL_CONTENT"
            | "SEARCH_WEB"
            | "GREP_SEARCH"
            | "INVOKE_SUBAGENT"
    )
}

fn cap_lines(text: &str, n: usize) -> String {
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() <= n {
        return text.trim_end_matches('\n').to_string();
    }
    lines[lines.len() - n..].join("\n")
}

fn summarize(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let lines: Vec<&str> = trimmed
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    let first = clip(lines.first().copied().unwrap_or(""));
    let last = clip(lines.last().copied().unwrap_or(""));
    format!(
        "{count} lines, {chars} chars\nfirst: {first}\nlast: {last}",
        count = lines.len(),
        chars = trimmed.chars().count(),
    )
}

fn clip(line: &str) -> String {
    let mut out = String::new();
    for (i, ch) in line.trim().chars().enumerate() {
        if i == 160 {
            out.push('…');
            break;
        }
        out.push(ch);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_then_cap_drops_tool_lines_and_keeps_the_tail() {
        let prior = "keep-a\nTool: read src/lib.rs\n{\"type\":\"tool_use\",\"name\":\"bash\"}\nkeep-b\nkeep-c";
        let out = apply_context_filter(prior, "strip_tools,cap:2").unwrap();
        assert_eq!(out, "keep-b\nkeep-c");
    }

    #[test]
    fn summarize_is_a_digest_not_the_middle() {
        let prior = "alpha\nbeta secret middle\ngamma";
        let out = apply_context_filter(prior, "summarize").unwrap();
        assert!(out.contains("3 lines"));
        assert!(out.contains("first: alpha"));
        assert!(out.contains("last: gamma"));
        assert!(!out.contains("secret middle"));
    }

    #[test]
    fn unknown_and_empty_steps_fail() {
        assert!(apply_context_filter("x", "drop_all").is_err());
        assert!(apply_context_filter("x", "cap:0").is_err());
        assert!(validate_context_filter("strip_tools,,cap:1").is_err());
        assert!(validate_handoff_metadata(&serde_json::json!({"agent":"root"})).is_err());
        assert!(validate_handoff_metadata(&serde_json::json!("nope")).is_err());
        validate_handoff_metadata(&serde_json::json!({"reason":"review","priority":2})).unwrap();
    }
}
