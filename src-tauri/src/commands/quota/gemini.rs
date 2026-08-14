//! Antigravity CLI (`agy`) quota adapter.
//!
//! `agy` exposes no documented usage-budget subcommand or flag (`agy
//! --help` lists none) and its interactive session's own quota display
//! isn't persisted anywhere locally readable (checked directly against
//! `~/.gemini/antigravity-cli/{cli.log,history.jsonl}` and the rest of
//! that directory, 2026-08-14) — it's computed live inside `agy` itself.
//! But `agy --print "/usage"` *does* run that slash command
//! non-interactively and print its real, tab-separated table to stdout —
//! confirmed directly against a live invocation, not assumed from
//! `--help` text alone:
//!
//! ```text
//! Gemini Models | Weekly Limit Remaining | 0% | 2026-08-17T13:45:22Z
//! Gemini Models | Five Hour Limit Remaining | disabled
//! Claude and GPT models | Weekly Limit Remaining | 76% | 2026-08-21T04:37:35Z
//! Claude and GPT models | Five Hour Limit Remaining | 29% | 2026-08-14T10:55:36Z
//! ```
//! (fields are actually tab-separated, not `|`-separated — shown this way
//! here only because rustdoc/clippy reject literal tabs in doc comments)
//!
//! This previously returned hardcoded placeholder percentages presented
//! as real quota — the owner caught the exact fabrication live (34%/100%
//! shown vs. the real session's 0.00%/disabled). This spawns a real `agy`
//! session and parses its real answer instead.

use super::quota_codex::{now_unix, unavailable_quota, ProviderQuota, ProviderQuotaWindow};
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

const HARNESS_TITLE: &str = "Google Antigravity CLI";

fn unavailable(detail: impl Into<String>) -> ProviderQuota {
    unavailable_quota("gemini", "google", HARNESS_TITLE, detail)
}

/// One `/usage` table row: `<family>\t<label>\t<percent or "disabled">\t<resets_at ISO 8601 or empty>`.
fn parse_usage_line(line: &str) -> Option<ProviderQuotaWindow> {
    let mut fields = line.split('\t');
    let family = fields.next()?.trim();
    let label = fields.next()?.trim();
    let percent_field = fields.next()?.trim();
    let resets_field = fields.next().unwrap_or("").trim();
    if family.is_empty() || label.is_empty() {
        return None;
    }

    // "disabled" happens when the parent weekly limit is already
    // exhausted, so the finer-grained window can't be evaluated — that's
    // functionally "0% remaining", not a distinct state this struct has
    // a field for.
    //
    // The label is "... Limit Remaining" and the printed number is that
    // remaining percentage directly (confirmed against the real session:
    // "Weekly Limit Remaining 0.00%" rendered as exhausted/red, "76%
    // remaining" as mostly-full/green) — not a "used" percentage.
    let (used_percent, remaining_percent) = if percent_field.eq_ignore_ascii_case("disabled") {
        (100, 0)
    } else {
        let remaining = percent_field.trim_end_matches('%').parse::<i32>().ok()?;
        ((100 - remaining).max(0), remaining)
    };
    let resets_at = (!resets_field.is_empty())
        .then(|| chrono::DateTime::parse_from_rfc3339(resets_field).ok())
        .flatten()
        .map(|dt| dt.timestamp());

    Some(ProviderQuotaWindow {
        label: label.to_string(),
        family: Some(family.to_string()),
        used_percent,
        remaining_percent,
        resets_at,
        window_minutes: None,
    })
}

pub(crate) fn gemini_quota() -> ProviderQuota {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let child = Command::new("agy")
        .args([
            "--output-format",
            "text",
            "--print-timeout",
            "25s",
            "--print",
            "/usage",
        ])
        .current_dir(&home)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn();
    let mut child = match child {
        Ok(child) => child,
        Err(error) => return unavailable(format!("agy unavailable: {error}")),
    };
    let Some(stdout) = child.stdout.take() else {
        let _ = child.kill();
        return unavailable("agy produced no stdout");
    };

    let windows: Vec<ProviderQuotaWindow> = BufReader::new(stdout)
        .lines()
        .take(200)
        .map_while(Result::ok)
        .filter_map(|line| parse_usage_line(&line))
        .collect();
    let _ = child.wait();

    if windows.is_empty() {
        return unavailable(
            "agy --print \"/usage\" returned no recognizable quota rows; run `agy` and log in if unauthenticated",
        );
    }

    ProviderQuota {
        agent_id: "gemini".into(),
        provider: "google".into(),
        harness_title: HARNESS_TITLE.into(),
        status: "ok".into(),
        detail: None,
        windows,
        fetched_at: now_unix(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_percent_row_with_a_reset_timestamp() {
        // Real sample: an exhausted weekly limit prints "0%" and the
        // owner's screenshot showed it rendered as 0.00% remaining, not
        // 0% used — the printed number is already "remaining".
        let window =
            parse_usage_line("Gemini Models\tWeekly Limit Remaining\t0%\t2026-08-17T13:45:22Z")
                .unwrap();
        assert_eq!(window.family.as_deref(), Some("Gemini Models"));
        assert_eq!(window.label, "Weekly Limit Remaining");
        assert_eq!(window.remaining_percent, 0);
        assert_eq!(window.used_percent, 100);
        assert!(window.resets_at.is_some());
    }

    #[test]
    fn a_mostly_full_limit_parses_used_as_the_complement_of_remaining() {
        let window = parse_usage_line(
            "Claude and GPT models\tWeekly Limit Remaining\t76%\t2026-08-21T04:37:35Z",
        )
        .unwrap();
        assert_eq!(window.remaining_percent, 76);
        assert_eq!(window.used_percent, 24);
    }

    #[test]
    fn disabled_maps_to_zero_remaining_with_no_reset() {
        let window =
            parse_usage_line("Gemini Models\tFive Hour Limit Remaining\tdisabled\t").unwrap();
        assert_eq!(window.used_percent, 100);
        assert_eq!(window.remaining_percent, 0);
        assert_eq!(window.resets_at, None);
    }

    #[test]
    fn real_sample_output_round_trips() {
        // Captured directly from a live `agy --output-format text --print
        // "/usage"` call, 2026-08-14 — the exact output that motivated
        // this fix.
        let sample = "Gemini Models\tWeekly Limit Remaining\t0%\t2026-08-17T13:45:22Z\n\
             Gemini Models\tFive Hour Limit Remaining\tdisabled\t\n\
             Claude and GPT models\tWeekly Limit Remaining\t76%\t2026-08-21T04:37:35Z\n\
             Claude and GPT models\tFive Hour Limit Remaining\t29%\t2026-08-14T10:55:36Z";
        let windows: Vec<_> = sample.lines().filter_map(parse_usage_line).collect();
        assert_eq!(windows.len(), 4);
        assert_eq!(windows[0].remaining_percent, 0);
        assert_eq!(windows[1].remaining_percent, 0);
        assert_eq!(windows[2].remaining_percent, 76);
        assert_eq!(windows[3].remaining_percent, 29);
    }

    #[test]
    fn blank_and_malformed_lines_are_skipped_not_erroring() {
        assert!(parse_usage_line("").is_none());
        assert!(parse_usage_line("only one field").is_none());
        assert!(parse_usage_line("Family\tLabel\tnot-a-percent\t").is_none());
    }
}
