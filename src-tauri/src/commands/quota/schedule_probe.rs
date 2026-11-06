//! Classify live provider quota for the S6 in-app schedule bridge (#355).
//!
//! Policy (owner "Both" usage source): a measurable session/weekly percent
//! window is the hard stop. Status `unavailable`, empty windows, balance-only
//! rows, and local-usage-only rows are honest unknowns — they do **not**
//! pause a task. Inventing 0% remaining from those shapes is forbidden.
//! Extracting this probe into `crates/cli` is S5c / #358, not this slice.

use super::quota_codex::ProviderQuota;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuotaProbeState {
    Available,
    Exhausted {
        window_label: String,
        remaining_percent: i32,
    },
    Unavailable {
        reason: String,
    },
}

pub fn classify_quota(quota: &ProviderQuota) -> QuotaProbeState {
    if quota.status != "ok" {
        return QuotaProbeState::Unavailable {
            reason: quota
                .detail
                .clone()
                .unwrap_or_else(|| format!("quota status is {}", quota.status)),
        };
    }
    if quota.windows.is_empty() {
        return QuotaProbeState::Unavailable {
            reason: quota
                .detail
                .clone()
                .unwrap_or_else(|| "no session/weekly percent window to enforce".into()),
        };
    }
    match quota
        .windows
        .iter()
        .find(|window| window.remaining_percent <= 0)
    {
        Some(window) => QuotaProbeState::Exhausted {
            window_label: window.label.clone(),
            remaining_percent: window.remaining_percent,
        },
        None => QuotaProbeState::Available,
    }
}

/// Exhausted wins (hard stop). Unavailable never invents a stop: if no
/// agent is exhausted, the task proceeds even when every probe is unknown.
pub fn target_quota_state(states: &[QuotaProbeState]) -> QuotaProbeState {
    if let Some(exhausted) = states.iter().find_map(|state| match state {
        QuotaProbeState::Exhausted {
            window_label,
            remaining_percent,
        } => Some(QuotaProbeState::Exhausted {
            window_label: window_label.clone(),
            remaining_percent: *remaining_percent,
        }),
        _ => None,
    }) {
        return exhausted;
    }
    if states
        .iter()
        .any(|state| matches!(state, QuotaProbeState::Available))
    {
        return QuotaProbeState::Available;
    }
    let reason = states
        .iter()
        .find_map(|state| match state {
            QuotaProbeState::Unavailable { reason } => Some(reason.clone()),
            _ => None,
        })
        .unwrap_or_else(|| "no live quota measurement".into());
    QuotaProbeState::Unavailable { reason }
}

#[cfg(test)]
mod tests {
    use super::super::quota_codex::{now_unix, ProviderQuota, ProviderQuotaWindow};
    use super::*;

    fn quota(
        status: &str,
        windows: Vec<ProviderQuotaWindow>,
        detail: Option<&str>,
    ) -> ProviderQuota {
        ProviderQuota {
            agent_id: "cursor".into(),
            provider: "cursor".into(),
            harness_title: "Cursor Agent".into(),
            status: status.into(),
            detail: detail.map(str::to_string),
            windows,
            fetched_at: now_unix(),
            balance: None,
            balance_info: None,
            local_usage: None,
        }
    }

    fn window(label: &str, remaining: i32) -> ProviderQuotaWindow {
        ProviderQuotaWindow {
            label: label.into(),
            family: None,
            used_percent: 100 - remaining.max(0),
            remaining_percent: remaining,
            resets_at: None,
            window_minutes: Some(300),
        }
    }

    #[test]
    fn remaining_zero_is_exhausted() {
        let state = classify_quota(&quota("ok", vec![window("5h", 0)], None));
        assert_eq!(
            state,
            QuotaProbeState::Exhausted {
                window_label: "5h".into(),
                remaining_percent: 0,
            }
        );
    }

    #[test]
    fn remaining_positive_is_available() {
        let state = classify_quota(&quota("ok", vec![window("5h", 40), window("7d", 70)], None));
        assert_eq!(state, QuotaProbeState::Available);
    }

    #[test]
    fn unavailable_status_is_not_a_stop() {
        let state = classify_quota(&quota(
            "unavailable",
            vec![],
            Some("no machine-readable usage surface"),
        ));
        assert!(matches!(state, QuotaProbeState::Unavailable { .. }));
    }

    #[test]
    fn ok_without_percent_windows_is_unavailable() {
        let state = classify_quota(&quota("ok", vec![], Some("local usage only")));
        assert!(matches!(state, QuotaProbeState::Unavailable { .. }));
    }

    #[test]
    fn exhausted_wins_over_unavailable_and_available() {
        let states = vec![
            QuotaProbeState::Unavailable {
                reason: "unknown".into(),
            },
            QuotaProbeState::Available,
            QuotaProbeState::Exhausted {
                window_label: "7d".into(),
                remaining_percent: 0,
            },
        ];
        assert_eq!(
            target_quota_state(&states),
            QuotaProbeState::Exhausted {
                window_label: "7d".into(),
                remaining_percent: 0,
            }
        );
    }

    #[test]
    fn all_unavailable_does_not_invent_exhaustion() {
        let states = vec![QuotaProbeState::Unavailable {
            reason: "probe skipped".into(),
        }];
        assert_eq!(
            target_quota_state(&states),
            QuotaProbeState::Unavailable {
                reason: "probe skipped".into(),
            }
        );
    }
}
