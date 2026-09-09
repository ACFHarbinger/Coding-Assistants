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

fn make_window(label: &str, used: i32, resets_secs: Option<i64>) -> ProviderQuotaWindow {
    let used = used.clamp(0, 100);
    ProviderQuotaWindow {
        label: format!("{label} (OpenCode Go)"),
        family: Some("OpenCode Go".into()),
        used_percent: used,
        remaining_percent: 100 - used,
        resets_at: resets_secs.map(|secs| now_unix() + secs),
        window_minutes: None,
    }
}

/// Byte offset + rounded value of every `N%` / `N.N%` token in `text`.
fn percent_positions(text: &str) -> Vec<(usize, i32)> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            // walk back over an optional number (digits and one '.')
            let mut start = i;
            while start > 0 && (bytes[start - 1].is_ascii_digit() || bytes[start - 1] == b'.') {
                start -= 1;
            }
            if start < i {
                if let Ok(value) = text[start..i].trim_matches('.').parse::<f64>() {
                    out.push((start, value.round() as i32));
                }
            }
        }
        i += 1;
    }
    out
}

/// Extract the OpenCode Go quota rows from the **whole** `/ogc-usage` output.
///
/// `opencode run "/ogc-usage"` always routes through a model turn, which
/// reformats the `ogc_usage` tool result unpredictably — sometimes the clean
/// list this adapter was written for, sometimes prose such as
/// `Monthly quota exceeded (100.2%). Rolling and weekly usage are at 0%.` or
/// `OpenCode Go usage: Rolling 0%, Weekly 0%, Monthly 100.2% (resets in 9d 11h)`.
/// The strict per-line parser is tried first (keeps the clean path exact);
/// on fewer than two rows, fall back to pairing each label with the nearest
/// percentage that follows it (then the nearest overall).
fn windows_from_report(text: &str) -> Vec<ProviderQuotaWindow> {
    let strict: Vec<ProviderQuotaWindow> = text.lines().filter_map(parse_usage_line).collect();
    if strict.len() >= 2 {
        return strict;
    }

    let lower = text.to_lowercase();
    let pcts = percent_positions(&lower);
    if pcts.is_empty() {
        return strict;
    }

    let mut out = Vec::new();
    for (label, key) in [
        ("Rolling", "rolling"),
        ("Weekly", "weekly"),
        ("Monthly", "monthly"),
    ] {
        let Some(label_pos) = lower.find(key) else {
            continue;
        };
        let after = pcts.iter().find(|(pos, _)| *pos >= label_pos);
        let nearest = pcts.iter().min_by_key(|(pos, _)| pos.abs_diff(label_pos));
        let Some(&(_, used)) = after.or(nearest) else {
            continue;
        };
        let resets = lower[label_pos..]
            .split_once("resets in")
            .map(|(_, rest)| rest.trim_start())
            .and_then(|rest| rest.split([')', ',', '\n']).next())
            .and_then(reset_seconds);
        out.push(make_window(label, used, resets));
    }
    out
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
        let text: String = BufReader::new(stdout)
            .lines()
            .take(500)
            .map_while(Result::ok)
            .collect::<Vec<_>>()
            .join("\n");
        let _ = tx.send(text);
    });
    let text = match rx.recv_timeout(timeout) {
        Ok(text) => text,
        Err(_) => {
            terminate_child(&mut child);
            // Do not join the reader on timeout: a descendant may still hold
            // the inherited stdout pipe even after the direct child is dead.
            return unavailable(
                "opencode run \"/ogc-usage\" did not answer within 30s (is the opencode-go-usage plugin installed and authenticated?)",
            );
        }
    };
    terminate_child(&mut child);
    let _ = reader.join();

    let windows = windows_from_report(&text);
    if windows.is_empty() {
        // Distinguish "ran but the model reformatted the answer past
        // recognition" from "produced nothing at all" — the earlier message
        // wrongly blamed a missing plugin in the first case.
        let snippet: String = text
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty() && !line.starts_with('⚠') && !line.starts_with('>'))
            .unwrap_or("")
            .chars()
            .take(120)
            .collect();
        return unavailable(if snippet.is_empty() {
            "opencode run \"/ogc-usage\" produced no output (is the opencode-go-usage plugin installed and authenticated?)".to_string()
        } else {
            format!(
                "opencode ran /ogc-usage but no quota rows could be read from its reply (got: \"{snippet}\")"
            )
        });
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

pub(crate) fn opencode_quota(allow_metered: bool) -> ProviderQuota {
    // `opencode run "/ogc-usage"` always routes through a model turn.
    if !allow_metered {
        return unavailable(super::quota_codex::METERED_PROBE_DISABLED_DETAIL);
    }
    let mut command = Command::new("opencode");
    command.args(["run", "/ogc-usage"]);
    run_opencode_quota(command, std::time::Duration::from_secs(30))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metered_probe_off_short_circuits_without_running_opencode() {
        let quota = opencode_quota(false);
        assert_eq!(quota.status, "unavailable");
        assert_eq!(quota.agent_id, AGENT_ID);
        let detail = quota.detail.unwrap();
        assert!(detail.contains("Allow metered usage probes"), "{detail}");
        assert!(quota.windows.is_empty());
    }

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
    fn whole_text_fallback_reads_prose_reformatted_by_the_model() {
        // Real shapes seen from `deepseek-*` driving `/ogc-usage`.
        let prose = "Monthly quota exceeded (100.2%). Rolling and weekly usage are at 0%.";
        let w = windows_from_report(prose);
        let pct = |label: &str| {
            w.iter()
                .find(|win| win.label.starts_with(label))
                .map(|win| win.used_percent)
        };
        assert_eq!(pct("Rolling"), Some(0));
        assert_eq!(pct("Weekly"), Some(0));
        assert_eq!(pct("Monthly"), Some(100));

        let one_liner =
            "OpenCode Go usage: Rolling 0%, Weekly 0%, Monthly 100.2% (resets in 9d 11h)";
        let w2 = windows_from_report(one_liner);
        assert_eq!(w2.len(), 3);
        assert_eq!(w2[2].used_percent, 100);
        assert!(w2[2].resets_at.is_some());
    }

    #[test]
    fn whole_text_fallback_defers_to_the_strict_list_when_it_is_clean() {
        let clean = "OpenCode Go Usage:\n\
             - Rolling: 1% (resets in 5h)\n\
             - Weekly: 2% (resets in 4d 15h)\n\
             - Monthly: 3% (resets in 9d)";
        let w = windows_from_report(clean);
        assert_eq!(w.len(), 3);
        assert_eq!(w[0].used_percent, 1);
        assert_eq!(w[1].used_percent, 2);
        assert_eq!(w[2].used_percent, 3);
    }

    #[test]
    fn percent_positions_finds_each_figure() {
        let got = percent_positions("a 0% then 100.2% end");
        assert_eq!(
            got.iter().map(|(_, v)| *v).collect::<Vec<_>>(),
            vec![0, 100]
        );
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
