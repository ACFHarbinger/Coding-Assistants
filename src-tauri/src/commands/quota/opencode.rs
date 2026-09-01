//! OpenCode Go quota adapter.
//!
//! OpenCode ships no `ogc-usage` CLI subcommand of its own; the usage-budget
//! readout comes from the third-party `opencode-go-usage` plugin, which
//! registers the `/ogc-usage` slash command and an `ogc_usage` tool. Running
//! `opencode run "/ogc-usage"` headlessly invokes that slash command and prints
//! the plugin's real answer to stdout. (The bare form `opencode ogc-usage` is
//! parsed by the CLI as a project-directory positional, so it must be passed
//! through `run`.)
//!
//! Captured live from a real invocation, 2026-08-30 — not assumed from docs:
//!
//! ```text
//! OpenCode Go Usage:
//! - Rolling: 0.7% (resets in 4h 56m)
//! - Weekly: 52.8% (resets in 4h 18m)
//! - Monthly: 76.4% (resets in 19d)
//! ```
//!
//! The plugin (and the model it runs through) can vary the dash prefix and the
//! reset-duration formatting, so the parser below is tolerant of both.

use super::quota_codex::{now_unix, unavailable_quota, ProviderQuota, ProviderQuotaWindow};
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};

const HARNESS_TITLE: &str = "OpenCode Go";
const AGENT_ID: &str = "opencode";
const PROVIDER: &str = "opencode-go";

fn unavailable(detail: impl Into<String>) -> ProviderQuota {
    unavailable_quota(AGENT_ID, PROVIDER, HARNESS_TITLE, detail)
}

fn terminate_child(child: &mut Child) {
    // `kill` does not reap the process. Always wait as well so repeated quota
    // refreshes cannot accumulate zombies after success or timeout.
    let _ = child.kill();
    let _ = child.wait();
}

/// Parse a duration like "4h 56m", "19d", "2d 18h" into seconds.
fn reset_seconds(text: &str) -> Option<i64> {
    let mut total = 0_i64;
    let mut saw = false;
    for part in text.split_whitespace() {
        let (amount, unit) = part.split_at(part.len() - 1);
        let amount: i64 = amount.parse().ok()?;
        match unit {
            "s" => total += amount,
            "m" => total += amount * 60,
            "h" => total += amount * 3_600,
            "d" => total += amount * 86_400,
            _ => return None,
        }
        saw = true;
    }
    if saw {
        Some(total)
    } else {
        None
    }
}

/// One line of `opencode run "/ogc-usage"` output. The printed number is a
/// used percentage (`Rolling: 52.8%` = 52.8% of the budget consumed), so it
/// maps directly onto `used_percent` with `remaining_percent` as its
/// complement. Returns `None` for anything that isn't a recognized row.
fn parse_usage_line(line: &str) -> Option<ProviderQuotaWindow> {
    let line = line.trim();
    // The plugin prints `  Rolling:  0.7% (resets in 4h 56m)` but the model
    // driving `/ogc-usage` may reformat rows as `- Rolling: 0.7% …`. Strip a
    // single leading list marker so both shapes parse.
    let line = match line.strip_prefix("- ") {
        Some(rest) => rest.trim(),
        None => line,
    };
    let (label, rest) = if let Some(rest) = line.strip_prefix("Rolling:") {
        ("Rolling", rest)
    } else if let Some(rest) = line.strip_prefix("Weekly:") {
        ("Weekly", rest)
    } else {
        ("Monthly", line.strip_prefix("Monthly:")?)
    };
    let used: i32 = rest.split('%').next()?.trim().parse::<f64>().ok()?.round() as i32;
    let used = used.clamp(0, 100);
    let resets_in = rest
        .rsplit_once("resets in")
        .map(|(_, after)| after.trim_end_matches(')').trim())
        .and_then(reset_seconds);
    Some(ProviderQuotaWindow {
        label: format!("{label} (OpenCode Go)"),
        family: Some("OpenCode Go".into()),
        used_percent: used,
        remaining_percent: 100 - used,
        resets_at: resets_in.map(|secs| now_unix() + secs),
        window_minutes: None,
    })
}

fn run_opencode_quota(mut command: Command, timeout: std::time::Duration) -> ProviderQuota {
    let mut child = match command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(child) => child,
        Err(error) => {
            return unavailable(format!(
                "opencode unavailable: {error} — opencode run \"/ogc-usage\" requires the opencode-usage plugin"
            ))
        }
    };
    let Some(stdout) = child.stdout.take() else {
        terminate_child(&mut child);
        return unavailable("opencode produced no stdout for /ogc-usage");
    };
    // Read on a dedicated thread and wait with a timeout, mirroring the Codex
    // adapter: `opencode run` spins up a full session and can take a while
    // (or, with no provider/credentials configured, sit waiting forever). We
    // are already inside a `spawn_blocking` task, but a bounded wait keeps the
    // quota fetch from hanging the refresh call indefinitely.
    let (tx, rx) = std::sync::mpsc::channel();
    let reader = std::thread::spawn(move || {
        let windows: Vec<ProviderQuotaWindow> = BufReader::new(stdout)
            .lines()
            .take(500)
            .map_while(Result::ok)
            .filter_map(|line| parse_usage_line(&line))
            .collect();
        let _ = tx.send(windows);
    });
    let windows = match rx.recv_timeout(timeout) {
        Ok(windows) => windows,
        Err(_) => {
            terminate_child(&mut child);
            // Do not join the reader on timeout: a descendant may still hold
            // the inherited stdout pipe even after the direct child is dead.
            return unavailable(
                "opencode run \"/ogc-usage\" did not answer within 30s (is the opencode-usage plugin installed and authenticated?)",
            );
        }
    };
    terminate_child(&mut child);
    let _ = reader.join();

    if windows.is_empty() {
        return unavailable(
            "opencode run \"/ogc-usage\" returned no recognizable quota rows (opencode-usage plugin not installed or not configured)",
        );
    }

    ProviderQuota {
        agent_id: AGENT_ID.into(),
        provider: PROVIDER.into(),
        harness_title: HARNESS_TITLE.into(),
        status: "ok".into(),
        detail: None,
        windows,
        fetched_at: now_unix(),
        balance: None,
    }
}

pub(crate) fn opencode_quota() -> ProviderQuota {
    let mut command = Command::new("opencode");
    command.args(["run", "/ogc-usage"]);
    run_opencode_quota(command, std::time::Duration::from_secs(30))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_real_sample_output() {
        // Captured from a live `opencode run "/ogc-usage"`, 2026-08-30.
        let sample = "OpenCode Go Usage:\n\
             - Rolling: 0.7% (resets in 4h 56m)\n\
             - Weekly: 52.8% (resets in 4h 18m)\n\
             - Monthly: 76.4% (resets in 19d)\n";
        let windows: Vec<_> = sample.lines().filter_map(parse_usage_line).collect();
        assert_eq!(windows.len(), 3, "{windows:?}");
        assert_eq!(windows[0].label, "Rolling (OpenCode Go)");
        assert_eq!(windows[0].used_percent, 1);
        assert_eq!(windows[0].remaining_percent, 99);
        assert_eq!(windows[1].used_percent, 53);
        assert_eq!(windows[2].used_percent, 76);
    }

    #[test]
    fn resets_at_is_computed_from_the_human_duration() {
        let window = parse_usage_line("- Weekly: 52.8% (resets in 4h 18m)").unwrap();
        assert!(window.resets_at.is_some());
        let before = now_unix();
        let after = now_unix() + 4 * 3_600 + 18 * 60 + 5;
        assert!(
            (before..after).contains(&window.resets_at.unwrap()),
            "resets_at {} should be ~4h18m after now",
            window.resets_at.unwrap()
        );
    }

    #[test]
    fn blank_and_malformed_lines_are_skipped() {
        assert!(parse_usage_line("").is_none());
        assert!(parse_usage_line("OpenCode Go Usage:").is_none());
        assert!(parse_usage_line("- Rolling: nope% (resets in 4h)").is_none());
        assert!(parse_usage_line("not a quota line").is_none());
    }

    #[test]
    fn reset_duration_parsing() {
        assert_eq!(reset_seconds("4h 56m"), Some(4 * 3_600 + 56 * 60));
        assert_eq!(reset_seconds("19d"), Some(19 * 86_400));
        assert_eq!(reset_seconds("2d 18h"), Some(2 * 86_400 + 18 * 3_600));
        assert_eq!(reset_seconds("45s"), Some(45));
        assert_eq!(reset_seconds(""), None);
        assert_eq!(reset_seconds("bogus"), None);
    }

    #[test]
    fn missing_binary_degrades_without_panicking() {
        let quota = run_opencode_quota(
            Command::new("/definitely/missing/coding-assistants-opencode"),
            std::time::Duration::from_millis(20),
        );
        assert_eq!(quota.status, "unavailable");
        assert!(quota
            .detail
            .as_deref()
            .unwrap_or_default()
            .contains("unavailable"));
    }
}
