//! Network samples on the U23 cadence (Team Scorecard T6 / #346).
//!
//! `collect_snapshot` calls [`sample_network_connections`] on every U23
//! `system_snapshot` poll — no new poller, no new timer. For each registered
//! harness session the sampler walks `/proc/<pid>/fd`, matches socket
//! inodes against `/proc/<pid>/net/tcp[6]` (the namespace table alone is
//! *not* per-process attribution), and records destination IP/port only. No
//! reverse DNS, no payloads, no byte counts — this observes sockets, not
//! traffic or security intent.
//!
//! Attribution and the PID-reuse guard:
//!
//! * **Managed sessions** trust the registered `managed_pid`. The start time
//!   of that pid is compared against the last scan recorded in the ring; a
//!   changed start time at the same pid means the pid was recycled and the
//!   scan is recorded as `pid_reused` — a visible coverage gap, never a
//!   silent empty list.
//! * **Registered observed sessions** have no stored pid, so ownership is
//!   re-established every scan from process identity: the executable must
//!   be the session's harness binary (`HarnessId::executable`) *and* the
//!   process working directory must equal the registered workspace. A
//!   recycled pid cannot pass that check, which is why the observed path
//!   re-attributes instead of guarding on a stale start time.
//!
//! Non-Linux hosts are never sampled; the hub-side read contract reports
//! `unavailable` there (U23's missing-`nvidia-smi` honesty rule). All `/proc`
//! access goes through an injectable root so the whole pipeline is
//! unit-testable against a fake tree.

/// Entry point from `collect_snapshot`: best-effort, never fails the
/// snapshot (same degrade rule as `managed_pids`).
pub fn sample_network_connections(now: i64) {
    #[cfg(target_os = "linux")]
    {
        let home = hub::default_hub_home();
        if let Some(targets) = registered_targets() {
            let baseline: std::collections::BTreeMap<String, (u32, u64)> =
                hub::net_samples::read_samples(&home)
                    .scans
                    .into_iter()
                    .map(|scan| (scan.session_key(), (scan.pid, scan.pid_start_time)))
                    .collect();
            let (samples, scans) =
                scan_targets(std::path::Path::new("/proc"), &targets, &baseline, now);
            if !samples.is_empty() || !scans.is_empty() {
                let _ = hub::net_samples::append_samples(&home, samples, scans, now);
            }
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = now;
    }
}

/// One session to sample, derived from the registration roster.
#[cfg(target_os = "linux")]
struct SampleTarget {
    harness: String,
    workspace: String,
    session_id: String,
    mode: String,
    pid: Option<u32>,
}

/// `(harness, workspace, session_id, mode, pid)` for every registered
/// session. A store failure degrades to `None` — the snapshot must not
/// fail because the roster is unreadable.
#[cfg(target_os = "linux")]
fn registered_targets() -> Option<Vec<SampleTarget>> {
    use hub::HarnessSessionMode;
    let store = crate::commands::commands::store::open_store().ok()?;
    let sessions = store.list_harness_sessions().ok()?;
    Some(
        sessions
            .into_iter()
            .map(|session| {
                let mode = match session.mode {
                    HarnessSessionMode::Managed => "managed",
                    HarnessSessionMode::Observed => "observed",
                };
                SampleTarget {
                    harness: session.harness,
                    workspace: session.workspace,
                    session_id: session.disk_session_id,
                    mode: mode.to_string(),
                    pid: session.managed_pid,
                }
            })
            .collect(),
    )
}

/// Scan every target under `proc_root`, producing sample rows and the
/// per-session latest-scan records for the ring.
#[cfg(target_os = "linux")]
fn scan_targets(
    proc_root: &std::path::Path,
    targets: &[SampleTarget],
    baseline: &std::collections::BTreeMap<String, (u32, u64)>,
    now: i64,
) -> (
    Vec<hub::net_samples::NetworkSample>,
    Vec<hub::net_samples::NetworkScan>,
) {
    use hub::net_samples::{NetworkSample, NetworkScan, NetworkScanStatus};

    let mut samples = Vec::new();
    let mut scans = Vec::new();
    for target in targets {
        let key = format!(
            "{}\u{1}{}\u{1}{}",
            target.harness, target.workspace, target.session_id
        );
        let attributed: Vec<u32> = match target.pid {
            // Managed: the store pid is authoritative; the start-time guard
            // is what detects a recycled pid.
            Some(pid) => {
                let Some(start_time) = read_pid_start_time(proc_root, pid) else {
                    continue; // process is gone: nothing to sample or record
                };
                if let Some((_, recorded_start)) = baseline.get(&key) {
                    if *recorded_start != start_time {
                        scans.push(NetworkScan {
                            harness: target.harness.clone(),
                            workspace: target.workspace.clone(),
                            session_id: target.session_id.clone(),
                            mode: target.mode.clone(),
                            pid,
                            pid_start_time: start_time,
                            scanned_at: now,
                            fds_unreadable: true,
                            socket_count: 0,
                            status: NetworkScanStatus::PidReused,
                        });
                        continue;
                    }
                }
                vec![pid]
            }
            // Observed: ownership re-established from exe + cwd each scan.
            None => {
                let pids = attribute_observed(proc_root, target);
                if pids.is_empty() {
                    continue; // no identifiable process: not attributed
                }
                pids
            }
        };
        for pid in attributed {
            let start_time = match read_pid_start_time(proc_root, pid) {
                Some(start) => start,
                None => continue,
            };
            let (inodes, fds_unreadable) = read_socket_inodes(proc_root, pid);
            let destinations = read_tcp_destinations(proc_root, pid);
            let mut socket_count = 0u32;
            for inode in &inodes {
                if let Some((protocol, ip, port)) = destinations.get(inode) {
                    socket_count += 1;
                    samples.push(NetworkSample {
                        harness: target.harness.clone(),
                        workspace: target.workspace.clone(),
                        session_id: target.session_id.clone(),
                        mode: target.mode.clone(),
                        pid,
                        pid_start_time: start_time,
                        protocol: protocol.clone(),
                        dest_ip: ip.clone(),
                        dest_port: *port,
                        sampled_at: now,
                        fds_unreadable,
                    });
                }
            }
            scans.push(NetworkScan {
                harness: target.harness.clone(),
                workspace: target.workspace.clone(),
                session_id: target.session_id.clone(),
                mode: target.mode.clone(),
                pid,
                pid_start_time: start_time,
                scanned_at: now,
                fds_unreadable,
                socket_count,
                status: NetworkScanStatus::Sampled,
            });
        }
    }
    (samples, scans)
}

/// Pids whose executable is this session's harness binary and whose working
/// directory is the registered workspace. Multiple instances (U15) all
/// attribute.
#[cfg(target_os = "linux")]
fn attribute_observed(proc_root: &std::path::Path, target: &SampleTarget) -> Vec<u32> {
    use hub::HarnessId;
    use std::fs;
    let Ok(harness) = HarnessId::parse(&target.harness) else {
        return Vec::new();
    };
    let executable = harness.executable();
    let mut pids = Vec::new();
    let Ok(entries) = fs::read_dir(proc_root) else {
        return pids;
    };
    for entry in entries.flatten() {
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse::<u32>().ok())
        else {
            continue;
        };
        let proc = proc_root.join(pid.to_string());
        let exe_matches = fs::read_link(proc.join("exe"))
            .ok()
            .and_then(|path| {
                path.file_name()
                    .map(|name| name.to_string_lossy() == executable)
            })
            .unwrap_or(false);
        if !exe_matches {
            continue;
        }
        let cwd_matches = fs::read_link(proc.join("cwd"))
            .ok()
            .map(|path| path.to_string_lossy() == target.workspace)
            .unwrap_or(false);
        if cwd_matches {
            pids.push(pid);
        }
    }
    pids.sort_unstable();
    pids
}

/// `/proc/<pid>/stat` field 22: clock ticks since boot. The comm field may
/// contain spaces and parens, so parsing starts after the last `)`.
#[cfg(target_os = "linux")]
fn read_pid_start_time(proc_root: &std::path::Path, pid: u32) -> Option<u64> {
    let stat = std::fs::read_to_string(proc_root.join(pid.to_string()).join("stat")).ok()?;
    parse_stat_start_time(&stat)
}

#[cfg(target_os = "linux")]
fn parse_stat_start_time(stat: &str) -> Option<u64> {
    let rest = stat.rsplit_once(')')?.1;
    // Fields after the comm start at field 3 (state); starttime is field
    // 22, i.e. index 22 - 3 = 19 in the whitespace-split tail.
    rest.split_whitespace().nth(19)?.parse().ok()
}

/// Socket inodes held open by the pid, plus whether the fd table was fully
/// readable. An fd entry we cannot readlink is a coverage gap
/// (`fds_unreadable`), not a fatal error.
#[cfg(target_os = "linux")]
fn read_socket_inodes(proc_root: &std::path::Path, pid: u32) -> (Vec<u64>, bool) {
    let fd_dir = proc_root.join(pid.to_string()).join("fd");
    let entries = match std::fs::read_dir(&fd_dir) {
        Ok(entries) => entries,
        Err(_) => return (Vec::new(), true),
    };
    let mut inodes = Vec::new();
    let mut unreadable = false;
    for entry in entries.flatten() {
        match std::fs::read_link(entry.path()) {
            Ok(link) => {
                if let Some(inode) = parse_socket_inode(&link.to_string_lossy()) {
                    inodes.push(inode);
                }
            }
            Err(_) => unreadable = true,
        }
    }
    (inodes, unreadable)
}

/// `"socket:[12345]"` → `12345`. Any other fd target (file, pipe, …) is not
/// a socket and yields `None`.
#[cfg(any(target_os = "linux", test))]
fn parse_socket_inode(link: &str) -> Option<u64> {
    let inner = link.strip_prefix("socket:[")?.strip_suffix(']')?;
    inner.parse().ok()
}

/// Established (`state 01`) socket destinations from the process's
/// network-namespace `tcp` and `tcp6` tables, keyed by socket inode. The
/// table is namespace-wide; the fd match is what makes it per-process.
#[cfg(target_os = "linux")]
fn read_tcp_destinations(
    proc_root: &std::path::Path,
    pid: u32,
) -> std::collections::HashMap<u64, (String, String, u16)> {
    use std::fs;
    let net_dir = proc_root.join(pid.to_string()).join("net");
    let mut map = std::collections::HashMap::new();
    for file in ["tcp", "tcp6"] {
        let Ok(text) = fs::read_to_string(net_dir.join(file)) else {
            continue;
        };
        for row in parse_net_tcp_rows(&text, file) {
            map.insert(row.inode, (row.protocol, row.dest_ip, row.dest_port));
        }
    }
    map
}

#[cfg(target_os = "linux")]
struct TcpRow {
    inode: u64,
    protocol: String,
    dest_ip: String,
    dest_port: u16,
}

#[cfg(target_os = "linux")]
fn parse_net_tcp_rows(text: &str, protocol: &str) -> Vec<TcpRow> {
    let mut rows = Vec::new();
    for line in text.lines().skip(1) {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 10 {
            continue;
        }
        // sl local rem st tx:rx tr:tm retrnsmt uid timeout inode
        if fields[3] != "01" {
            continue; // established sockets only
        }
        let Some((ip, port)) = parse_proc_address(fields[2]) else {
            continue;
        };
        let Ok(inode) = fields[9].parse() else {
            continue;
        };
        rows.push(TcpRow {
            inode,
            protocol: protocol.to_string(),
            dest_ip: ip,
            dest_port: port,
        });
    }
    rows
}

/// `"0100007F:1F90"` → `("127.0.0.1", 6416)`. Addresses are little-endian
/// per 32-bit group; ports are big-endian hex.
#[cfg(any(target_os = "linux", test))]
fn parse_proc_address(field: &str) -> Option<(String, u16)> {
    let (ip_hex, port_hex) = field.split_once(':')?;
    let port = u16::from_str_radix(port_hex, 16).ok()?;
    let ip = if ip_hex.len() == 8 {
        decode_hex_ipv4(ip_hex)?
    } else if ip_hex.len() == 32 {
        decode_hex_ipv6(ip_hex)?
    } else {
        return None;
    };
    Some((ip, port))
}

#[cfg(any(target_os = "linux", test))]
fn decode_hex_ipv4(hex: &str) -> Option<String> {
    let raw = u32::from_str_radix(hex, 16).ok()?;
    Some(format!(
        "{}.{}.{}.{}",
        raw & 0xFF,
        (raw >> 8) & 0xFF,
        (raw >> 16) & 0xFF,
        (raw >> 24) & 0xFF
    ))
}

#[cfg(any(target_os = "linux", test))]
fn decode_hex_ipv6(hex: &str) -> Option<String> {
    if hex.len() != 32 || hex.chars().any(|c| !c.is_ascii_hexdigit()) {
        return None;
    }
    let mut groups = Vec::with_capacity(8);
    for chunk in hex.as_bytes().chunks(8) {
        let raw = u32::from_str_radix(std::str::from_utf8(chunk).ok()?, 16).ok()?;
        // Each 8-char chunk is one 32-bit word little-endian; after the
        // byte swap the two 16-bit IPv6 groups are the high and low halves
        // of that word.
        let word = raw.swap_bytes();
        groups.push(format!("{:04x}", word >> 16));
        groups.push(format!("{:04x}", word & 0xFFFF));
    }
    Some(groups.join(":"))
}

#[cfg(all(test, target_os = "linux"))]
#[path = "network_tests.rs"]
mod tests;
