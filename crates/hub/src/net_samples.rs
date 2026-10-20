//! Local-only network-sample ring (Team Scorecard T6 / #346).
//!
//! Managed harness sessions and registered observed sessions get their
//! sockets sampled on the existing U23 `system_snapshot` cadence (see
//! `src-tauri/src/commands/system/network.rs` for the `/proc` walk). This
//! module owns retention and the versioned read contract:
//!
//! * **Local-only.** The ring lives in `<hub home>/cache/network-samples.json`.
//!   The sync policy classifies `cache/` as `Exclude`, so samples never ride
//!   a cloud-sync snapshot (owner answer to Kimi's Q7: network samples stay
//!   local-only, like U23 snapshots). This module must not store them in
//!   `hub.db`, which syncs as `HubDatabase`.
//! * **Capped ring (O18, owner-confirmed).** The last 24 hours or 300 rows
//!   per session, whichever is smaller — a fixed internal bound, not an
//!   owner setting.
//! * **Versioned read contract.** `NetworkSamplesView` is the stable shape
//!   Kimi's #347 consistency probe reads; `NETWORK_SAMPLES_VERSION` bumps
//!   only on a breaking change. `fds_unreadable` is an explicit field, never
//!   a silently empty sample list (the U23 honesty rule).
//!
//! Writes are atomic (temp file + rename) so a concurrent CLI reader never
//! sees a partial file. Concurrent writers are last-writer-wins, which is
//! acceptable for advisory observability.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Bump only on a breaking change to the read contract.
pub const NETWORK_SAMPLES_VERSION: u32 = 1;

/// Fixed retention window (O18): samples older than this are dropped.
const RETENTION_SECS: i64 = 24 * 60 * 60;
/// Fixed per-session row cap (O18: "a few hundred").
const MAX_ROWS_PER_SESSION: usize = 300;
/// Relative to the hub home; `cache/` is a sync `Exclude` category.
const RING_RELATIVE_PATH: &str = "cache/network-samples.json";

/// One attributed socket observation: destination IP and port only. No
/// reverse DNS, no payloads, no byte counts — this observes sockets, not
/// traffic or security intent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkSample {
    pub harness: String,
    pub workspace: String,
    /// Registered session's disk session id.
    pub session_id: String,
    /// `managed` | `observed`.
    pub mode: String,
    pub pid: u32,
    /// `/proc/<pid>/stat` field 22 (clock ticks since boot) at sample time;
    /// recorded so a later PID reuse is detectable.
    pub pid_start_time: u64,
    /// `tcp` | `tcp6`.
    pub protocol: String,
    pub dest_ip: String,
    pub dest_port: u16,
    /// Unix seconds.
    pub sampled_at: i64,
    /// The `/proc/<pid>/fd` walk hit an unreadable entry: this scan's socket
    /// list may be incomplete. A field on the record, never a silent gap.
    pub fds_unreadable: bool,
}

/// Why a session was or was not sampled on the last scan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkScanStatus {
    /// Sockets were enumerated (possibly zero of them).
    Sampled,
    /// The PID still exists but its start time no longer matches the
    /// previously recorded one — the pid was recycled, so nothing was
    /// attributed. Visible as a coverage gap, not as "no network".
    PidReused,
}

impl NetworkScanStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Sampled => "sampled",
            Self::PidReused => "pid_reused",
        }
    }
}

/// The latest scan of one session — the coverage record that keeps "scanned,
/// zero sockets, fd table fully readable" distinguishable from "not scanned".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkScan {
    pub harness: String,
    pub workspace: String,
    pub session_id: String,
    pub mode: String,
    pub pid: u32,
    pub pid_start_time: u64,
    /// Unix seconds of the scan.
    pub scanned_at: i64,
    pub fds_unreadable: bool,
    /// Sockets attributed to this session in this scan.
    pub socket_count: u32,
    pub status: NetworkScanStatus,
}

impl NetworkScan {
    /// Stable per-session key: `(harness, workspace, session_id)`.
    pub fn session_key(&self) -> String {
        format!(
            "{}\u{1}{}\u{1}{}",
            self.harness, self.workspace, self.session_id
        )
    }
}

/// Persisted ring shape. `scans` holds the latest scan per session key.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NetworkSamplesFile {
    pub version: u32,
    pub samples: Vec<NetworkSample>,
    pub scans: Vec<NetworkScan>,
}

impl Default for NetworkSamplesFile {
    fn default() -> Self {
        Self {
            version: NETWORK_SAMPLES_VERSION,
            samples: Vec::new(),
            scans: Vec::new(),
        }
    }
}

/// One session's projection in the read contract: latest coverage record
/// plus the retained sample rows.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NetworkSessionSamples {
    pub harness: String,
    pub workspace: String,
    pub session_id: String,
    pub mode: String,
    pub pid: u32,
    pub pid_start_time: u64,
    pub last_scanned_at: i64,
    pub fds_unreadable: bool,
    pub socket_count: u32,
    pub status: NetworkScanStatus,
    pub samples: Vec<NetworkSample>,
}

/// Versioned read contract for #347 (and any later panel). Grouped by
/// session, samples newest-last, sessions sorted by harness then workspace.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NetworkSamplesView {
    pub version: u32,
    pub generated_at: i64,
    /// `Some(detail)` on platforms with no sampler (non-Linux: same honesty
    /// rule as U23's missing `nvidia-smi`); `None` on Linux.
    pub unavailable: Option<String>,
    pub sessions: Vec<NetworkSessionSamples>,
}

/// Ring file path under the hub home.
pub fn samples_path(home: &Path) -> PathBuf {
    home.join(RING_RELATIVE_PATH)
}

/// Tolerant read: a missing or corrupt file yields the empty ring rather
/// than an error — observability must not break on a bad read. The corrupt
/// file is left in place; the next append overwrites it.
pub fn read_samples(home: &Path) -> NetworkSamplesFile {
    let Ok(text) = fs::read_to_string(samples_path(home)) else {
        return NetworkSamplesFile::default();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

/// Append new samples and latest-scan records, then enforce the O18 cap and
/// write atomically. Errors are returned; callers degrade (the U23 snapshot
/// must not fail because the ring is unwritable).
pub fn append_samples(
    home: &Path,
    new_samples: Vec<NetworkSample>,
    new_scans: Vec<NetworkScan>,
    now: i64,
) -> Result<(), String> {
    let mut file = read_samples(home);
    if file.version != NETWORK_SAMPLES_VERSION {
        // A shape change resets the ring rather than misreading old rows.
        file = NetworkSamplesFile::default();
    }

    let mut scans: BTreeMap<String, NetworkScan> = file
        .scans
        .into_iter()
        .map(|scan| (scan.session_key(), scan))
        .collect();
    for scan in new_scans {
        scans.insert(scan.session_key(), scan);
    }

    file.samples.extend(new_samples);
    let cutoff = now - RETENTION_SECS;
    let mut per_session: BTreeMap<String, Vec<NetworkSample>> = BTreeMap::new();
    for sample in file.samples {
        if sample.sampled_at < cutoff {
            continue;
        }
        let key = format!(
            "{}\u{1}{}\u{1}{}",
            sample.harness, sample.workspace, sample.session_id
        );
        per_session.entry(key).or_default().push(sample);
    }
    let mut samples = Vec::new();
    for (_, mut rows) in per_session {
        if rows.len() > MAX_ROWS_PER_SESSION {
            // Keep the newest rows of this session.
            let excess = rows.len() - MAX_ROWS_PER_SESSION;
            rows.drain(..excess);
        }
        samples.extend(rows);
    }

    let file = NetworkSamplesFile {
        version: NETWORK_SAMPLES_VERSION,
        samples,
        scans: scans.into_values().collect(),
    };

    let path = samples_path(home);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let text = serde_json::to_string(&file).map_err(|error| error.to_string())?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, text).map_err(|error| error.to_string())?;
    fs::rename(&tmp, &path).map_err(|error| error.to_string())?;
    Ok(())
}

/// The versioned read contract over the ring: per-session coverage plus
/// retained samples, or an explicit `unavailable` on platforms with no
/// sampler.
pub fn network_samples_view(home: &Path, now: i64) -> NetworkSamplesView {
    let unavailable = platform_unavailable();
    let file = read_samples(home);
    let mut scans: BTreeMap<String, NetworkScan> = file
        .scans
        .into_iter()
        .map(|scan| (scan.session_key(), scan))
        .collect();
    let mut grouped: BTreeMap<String, Vec<NetworkSample>> = BTreeMap::new();
    for sample in file.samples {
        let key = format!(
            "{}\u{1}{}\u{1}{}",
            sample.harness, sample.workspace, sample.session_id
        );
        grouped.entry(key).or_default().push(sample);
    }

    let mut keys: Vec<String> = scans.keys().cloned().collect();
    for key in grouped.keys() {
        if !scans.contains_key(key) && !keys.contains(key) {
            keys.push(key.clone());
        }
    }
    keys.sort();

    let mut sessions = Vec::new();
    for key in keys {
        let scan = scans.remove(&key).or_else(|| {
            grouped
                .get(&key)
                .and_then(|rows| rows.last())
                .map(|sample| NetworkScan {
                    harness: sample.harness.clone(),
                    workspace: sample.workspace.clone(),
                    session_id: sample.session_id.clone(),
                    mode: sample.mode.clone(),
                    pid: sample.pid,
                    pid_start_time: sample.pid_start_time,
                    scanned_at: sample.sampled_at,
                    fds_unreadable: sample.fds_unreadable,
                    socket_count: 0,
                    status: NetworkScanStatus::Sampled,
                })
        });
        let Some(scan) = scan else {
            continue;
        };
        sessions.push(NetworkSessionSamples {
            harness: scan.harness.clone(),
            workspace: scan.workspace.clone(),
            session_id: scan.session_id.clone(),
            mode: scan.mode.clone(),
            pid: scan.pid,
            pid_start_time: scan.pid_start_time,
            last_scanned_at: scan.scanned_at,
            fds_unreadable: scan.fds_unreadable,
            socket_count: scan.socket_count,
            status: scan.status,
            samples: grouped.remove(&key).unwrap_or_default(),
        });
    }

    NetworkSamplesView {
        version: NETWORK_SAMPLES_VERSION,
        generated_at: now,
        unavailable,
        sessions,
    }
}

/// Non-Linux hosts are never sampled (no `/proc`), the same way U23 reports
/// a missing `nvidia-smi`: an explicit `unavailable`, never a silent zero.
#[cfg(target_os = "linux")]
fn platform_unavailable() -> Option<String> {
    None
}

#[cfg(not(target_os = "linux"))]
fn platform_unavailable() -> Option<String> {
    Some("network samples need Linux /proc; this platform has no sampler".to_string())
}

#[cfg(test)]
#[path = "net_samples_tests.rs"]
mod tests;
