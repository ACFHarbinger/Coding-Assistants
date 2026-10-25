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

/// One logical prior-context line. Message attribution stays in `prefix`
/// until render, so `strip_tools` can see the stored body.
#[derive(Debug, Clone)]
pub(crate) struct PriorLine {
    pub text: String,
    pub prefix: Option<String>,
    /// Stored message bodies are stripped. Advance notes are not.
    pub strip: bool,
}

pub(crate) fn render_prior_lines(lines: &[PriorLine]) -> String {
    lines
        .iter()
        .map(|line| match &line.prefix {
            Some(prefix) => format!("{prefix}{}", line.text),
            None => line.text.clone(),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Apply the filter to raw message lines, then add attribution.
pub(crate) fn filter_attributed_prior(
    mut lines: Vec<PriorLine>,
    spec: &str,
) -> Result<String, HubError> {
    for step in parse_steps(spec.trim())? {
        match step {
            Step::StripTools => lines.retain(|line| !line.strip || !is_tool_noise(&line.text)),
            Step::Cap(n) => {
                if lines.len() > n {
                    lines = lines.split_off(lines.len() - n);
                }
            }
            // Later steps see the digest lines. `summarize,cap:1` keeps one
            // digest line; `summarize,strip_tools` can still drop tool noise.
            Step::Summarize => {
                lines = summarize(&render_prior_lines(&lines))
                    .lines()
                    .map(|line| PriorLine {
                        text: line.to_string(),
                        prefix: None,
                        strip: true,
                    })
                    .collect();
            }
        }
    }
    Ok(render_prior_lines(&lines))
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

    fn body(text: &str) -> PriorLine {
        PriorLine {
            text: text.into(),
            prefix: None,
            strip: true,
        }
    }

    #[test]
    fn strip_then_cap_drops_tool_lines_and_keeps_the_tail() {
        let lines = vec![
            body("keep-a"),
            body("Tool: read src/lib.rs"),
            body(r#"{"type":"tool_use","name":"bash"}"#),
            body("keep-b"),
            body("keep-c"),
        ];
        let out = filter_attributed_prior(lines, "strip_tools,cap:2").unwrap();
        assert_eq!(out, "keep-b\nkeep-c");
    }

    #[test]
    fn summarize_is_a_digest_not_the_middle() {
        let lines = vec![body("alpha"), body("beta secret middle"), body("gamma")];
        let out = filter_attributed_prior(lines, "summarize").unwrap();
        assert!(out.contains("3 lines"));
        assert!(out.contains("first: alpha"));
        assert!(out.contains("last: gamma"));
        assert!(!out.contains("secret middle"));
    }

    #[test]
    fn steps_after_summarize_still_run_left_to_right() {
        let lines = vec![body("alpha"), body("beta secret middle"), body("gamma")];
        let capped = filter_attributed_prior(lines, "summarize,cap:1").unwrap();
        assert_eq!(capped, "last: gamma");

        let summarized_tail = filter_attributed_prior(
            vec![body("alpha"), body("beta secret middle"), body("gamma")],
            "cap:1,summarize",
        )
        .unwrap();
        assert!(summarized_tail.contains("1 lines"));
        assert!(summarized_tail.contains("first: gamma"));
        assert!(!summarized_tail.contains("alpha"));

        let stripped =
            filter_attributed_prior(vec![body(r#"{"tool_calls":[]}"#)], "summarize,strip_tools")
                .unwrap();
        assert!(!stripped.contains("tool_calls"));
        assert!(stripped.contains("lines"));
    }

    #[test]
    fn unknown_and_empty_steps_fail() {
        assert!(filter_attributed_prior(vec![body("x")], "drop_all").is_err());
        assert!(filter_attributed_prior(vec![body("x")], "cap:0").is_err());
        assert!(validate_context_filter("strip_tools,,cap:1").is_err());
        assert!(validate_handoff_metadata(&serde_json::json!({"agent":"root"})).is_err());
        assert!(validate_handoff_metadata(&serde_json::json!("nope")).is_err());
        validate_handoff_metadata(&serde_json::json!({"reason":"review","priority":2})).unwrap();
    }

    #[test]
    fn strip_tools_sees_first_line_tool_records_before_attribution() {
        let lines = vec![
            PriorLine {
                text: "Tool: read src/lib.rs".into(),
                prefix: Some("grok -> claude (message): ".into()),
                strip: true,
            },
            PriorLine {
                text: r#"{"type":"RUN_COMMAND"}"#.into(),
                prefix: Some("grok -> claude (message): ".into()),
                strip: true,
            },
            PriorLine {
                text: "keep-visible".into(),
                prefix: Some("gemini -> claude (message): ".into()),
                strip: true,
            },
            PriorLine {
                text: "note: tail".into(),
                prefix: None,
                strip: false,
            },
        ];
        let out = filter_attributed_prior(lines, "strip_tools").unwrap();
        assert!(!out.contains("Tool:"));
        assert!(!out.contains("RUN_COMMAND"));
        assert_eq!(out, "gemini -> claude (message): keep-visible\nnote: tail");
    }
}
