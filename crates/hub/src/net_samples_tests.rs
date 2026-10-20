use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_HOME: AtomicUsize = AtomicUsize::new(0);

fn home() -> std::path::PathBuf {
    let index = NEXT_HOME.fetch_add(1, Ordering::SeqCst);
    std::env::temp_dir().join(format!("ca-net-samples-{}-{index}", std::process::id()))
}

fn sample(harness: &str, session: &str, at: i64) -> NetworkSample {
    NetworkSample {
        harness: harness.to_string(),
        workspace: "/tmp/ws".to_string(),
        session_id: session.to_string(),
        mode: "managed".to_string(),
        pid: 42,
        pid_start_time: 100,
        protocol: "tcp".to_string(),
        dest_ip: "1.2.3.4".to_string(),
        dest_port: 443,
        sampled_at: at,
        fds_unreadable: false,
    }
}

fn scan(harness: &str, session: &str, at: i64, fds: bool) -> NetworkScan {
    NetworkScan {
        harness: harness.to_string(),
        workspace: "/tmp/ws".to_string(),
        session_id: session.to_string(),
        mode: "managed".to_string(),
        pid: 42,
        pid_start_time: 100,
        scanned_at: at,
        fds_unreadable: fds,
        socket_count: 1,
        status: NetworkScanStatus::Sampled,
    }
}

#[test]
fn append_then_read_roundtrips() {
    let home = home();
    append_samples(
        &home,
        vec![sample("claude", "s1", 100)],
        vec![scan("claude", "s1", 100, false)],
        100,
    )
    .unwrap();
    let file = read_samples(&home);
    assert_eq!(file.version, NETWORK_SAMPLES_VERSION);
    assert_eq!(file.samples.len(), 1);
    assert_eq!(file.samples[0].dest_ip, "1.2.3.4");
    assert_eq!(file.scans.len(), 1);
    std::fs::remove_dir_all(&home).ok();
}

#[test]
fn samples_older_than_24h_are_dropped() {
    let home = home();
    let now = 1_000_000;
    append_samples(
        &home,
        vec![
            sample("claude", "s1", now - 1),
            sample("claude", "s1", now - 24 * 60 * 60 - 1),
        ],
        vec![],
        now,
    )
    .unwrap();
    let file = read_samples(&home);
    assert_eq!(file.samples.len(), 1);
    assert_eq!(file.samples[0].sampled_at, now - 1);
    std::fs::remove_dir_all(&home).ok();
}

#[test]
fn per_session_rows_are_capped_at_the_newest_300() {
    let home = home();
    let now = 1_000_000;
    // 350 rows in one session, 2 in another: the first 50 of s1 must go.
    let mut samples: Vec<NetworkSample> = (0..350)
        .map(|index| sample("claude", "s1", now - 350 + index))
        .collect();
    samples.push(sample("grok", "s2", now));
    samples.push(sample("grok", "s2", now - 1));
    append_samples(&home, samples, vec![], now).unwrap();
    let file = read_samples(&home);
    assert_eq!(file.samples.len(), 302);
    let s1: Vec<&NetworkSample> = file
        .samples
        .iter()
        .filter(|sample| sample.session_id == "s1")
        .collect();
    assert_eq!(s1.len(), 300);
    assert_eq!(s1.first().unwrap().sampled_at, now - 300);
    assert_eq!(s1.last().unwrap().sampled_at, now - 1);
    std::fs::remove_dir_all(&home).ok();
}

#[test]
fn scans_replace_the_latest_record_per_session() {
    let home = home();
    append_samples(
        &home,
        vec![],
        vec![
            scan("claude", "s1", 100, false),
            scan("claude", "s1", 200, true),
        ],
        200,
    )
    .unwrap();
    let file = read_samples(&home);
    assert_eq!(file.scans.len(), 1);
    assert_eq!(file.scans[0].scanned_at, 200);
    assert!(file.scans[0].fds_unreadable);
    std::fs::remove_dir_all(&home).ok();
}

#[test]
fn a_corrupt_ring_file_reads_as_empty_not_as_an_error() {
    let home = home();
    std::fs::create_dir_all(home.join("cache")).unwrap();
    std::fs::write(samples_path(&home), "{ not json").unwrap();
    assert_eq!(read_samples(&home), NetworkSamplesFile::default());
    // The next append repairs the file.
    append_samples(&home, vec![sample("claude", "s1", 5)], vec![], 5).unwrap();
    assert_eq!(read_samples(&home).samples.len(), 1);
    std::fs::remove_dir_all(&home).ok();
}

#[test]
fn an_unknown_version_resets_the_ring_instead_of_misreading_it() {
    let home = home();
    append_samples(&home, vec![sample("claude", "s1", 10)], vec![], 10).unwrap();
    let path = samples_path(&home);
    let text = std::fs::read_to_string(&path).unwrap().replace(
        &format!("\"version\":{NETWORK_SAMPLES_VERSION}"),
        "\"version\":99",
    );
    std::fs::write(&path, text).unwrap();
    append_samples(&home, vec![sample("claude", "s2", 20)], vec![], 20).unwrap();
    let file = read_samples(&home);
    assert_eq!(file.samples.len(), 1);
    assert_eq!(file.samples[0].session_id, "s2");
    std::fs::remove_dir_all(&home).ok();
}

#[test]
fn view_groups_coverage_and_samples_per_session() {
    let home = home();
    append_samples(
        &home,
        vec![
            sample("claude", "s1", 100),
            sample("claude", "s1", 101),
            sample("grok", "s2", 100),
        ],
        vec![
            scan("claude", "s1", 101, false),
            scan("grok", "s2", 100, true),
        ],
        101,
    )
    .unwrap();
    let view = network_samples_view(&home, 101);
    assert_eq!(view.version, NETWORK_SAMPLES_VERSION);
    assert_eq!(view.sessions.len(), 2);
    let claude = view
        .sessions
        .iter()
        .find(|session| session.harness == "claude")
        .unwrap();
    assert_eq!(claude.samples.len(), 2);
    assert!(!claude.fds_unreadable);
    assert_eq!(claude.status, NetworkScanStatus::Sampled);
    let grok = view
        .sessions
        .iter()
        .find(|session| session.harness == "grok")
        .unwrap();
    // fds_unreadable is a field on the record, not a silently empty list.
    assert!(grok.fds_unreadable);
    assert_eq!(grok.samples.len(), 1);
    std::fs::remove_dir_all(&home).ok();
}

#[test]
fn a_pid_reused_scan_is_a_visible_coverage_gap() {
    let home = home();
    let mut reused = scan("claude", "s1", 100, false);
    reused.status = NetworkScanStatus::PidReused;
    reused.socket_count = 0;
    append_samples(&home, vec![], vec![reused], 100).unwrap();
    let view = network_samples_view(&home, 100);
    assert_eq!(view.sessions.len(), 1);
    assert_eq!(view.sessions[0].status, NetworkScanStatus::PidReused);
    assert_eq!(view.sessions[0].socket_count, 0);
    std::fs::remove_dir_all(&home).ok();
}

#[cfg(not(target_os = "linux"))]
#[test]
fn non_linux_views_report_unavailable() {
    let view = network_samples_view(std::path::Path::new("/nonexistent"), 1);
    assert!(view.unavailable.is_some());
    assert!(view.sessions.is_empty());
}

#[test]
fn an_idle_append_prunes_expired_scans_and_samples() {
    let home = home();
    let at = 100;
    append_samples(
        &home,
        vec![sample("claude", "s1", at)],
        vec![scan("claude", "s1", at, false)],
        at,
    )
    .unwrap();
    // A poll that produces no rows must still run retention.
    let day_later = at + RETENTION_SECS + 1;
    append_samples(&home, vec![], vec![], day_later).unwrap();
    let file = read_samples(&home);
    assert!(file.samples.is_empty(), "expired samples must be pruned");
    assert!(file.scans.is_empty(), "expired scan records must be pruned");
    // Trusted identity outlives the coverage window.
    assert_eq!(file.identities.len(), 1);
    assert_eq!(
        trusted_identity_baseline(&file, day_later).get("claude\u{1}/tmp/ws\u{1}s1"),
        Some(&(42, 100))
    );
    std::fs::remove_dir_all(&home).ok();
}

#[test]
fn trusted_identities_expire_after_a_week_without_a_refresh() {
    let home = home();
    let at = 100;
    append_samples(&home, vec![], vec![scan("claude", "s1", at, false)], at).unwrap();
    let just_under = at + IDENTITY_RETENTION_SECS;
    append_samples(&home, vec![], vec![], just_under).unwrap();
    assert_eq!(read_samples(&home).identities.len(), 1);
    let week_later = at + IDENTITY_RETENTION_SECS + 1;
    append_samples(&home, vec![], vec![], week_later).unwrap();
    assert!(
        read_samples(&home).identities.is_empty(),
        "an unrefreshed identity must retire rather than trust a possibly recycled pid forever"
    );
    std::fs::remove_dir_all(&home).ok();
}

#[test]
fn expired_identities_are_not_used_before_the_next_pruning_write() {
    let home = home();
    append_samples(&home, vec![], vec![scan("claude", "s1", 100, false)], 100).unwrap();
    let file = read_samples(&home);
    assert!(trusted_identity_baseline(&file, 101 + IDENTITY_RETENTION_SECS).is_empty());
    std::fs::remove_dir_all(&home).ok();
}

#[test]
fn the_read_view_expires_stale_rows_without_an_append() {
    let home = home();
    let at = 100;
    append_samples(
        &home,
        vec![sample("claude", "s1", at)],
        vec![scan("claude", "s1", at, false)],
        at,
    )
    .unwrap();
    // A CLI probe reading days later must not see expired rows as current.
    let view = network_samples_view(&home, at + RETENTION_SECS + 1);
    assert!(view.sessions.is_empty());
    // Inside the window the same rows are visible.
    let fresh = network_samples_view(&home, at + RETENTION_SECS - 1);
    assert_eq!(fresh.sessions.len(), 1);
    std::fs::remove_dir_all(&home).ok();
}

#[test]
fn pid_reused_scans_refresh_the_trusted_identity_not_the_rejected_one() {
    let home = home();
    // Baseline: pid 42 started at 100.
    append_samples(&home, vec![], vec![scan("claude", "s1", 100, false)], 100).unwrap();
    // The pid was recycled: the scan records the trusted start time, not the
    // rejected one (see f30c0fc).
    let mut reused = scan("claude", "s1", 200, true);
    reused.status = NetworkScanStatus::PidReused;
    reused.pid_start_time = 100; // trusted identity, preserved by the sampler
    append_samples(&home, vec![], vec![reused], 200).unwrap();
    let file = read_samples(&home);
    assert_eq!(file.identities.len(), 1);
    assert_eq!(
        trusted_identity_baseline(&file, 200).get("claude\u{1}/tmp/ws\u{1}s1"),
        Some(&(42, 100)),
        "a rejected recycled start time must never become the trusted baseline"
    );
    std::fs::remove_dir_all(&home).ok();
}

#[test]
fn the_baseline_prefers_identities_over_scan_records() {
    let home = home();
    let at = 100;
    append_samples(&home, vec![], vec![scan("claude", "s1", at, false)], at).unwrap();
    let mut file = read_samples(&home);
    // A newer scan with a different identity loses to the trusted identity.
    let mut diverged = scan("grok", "s2", at, false);
    diverged.pid = 99;
    diverged.pid_start_time = 999;
    file.scans.push(diverged);
    let baseline = trusted_identity_baseline(&file, at);
    assert_eq!(
        baseline.get("claude\u{1}/tmp/ws\u{1}s1"),
        Some(&(42, 100)),
        "the identity entry wins over any scan record"
    );
    assert_eq!(
        baseline.get("grok\u{1}/tmp/ws\u{1}s2"),
        Some(&(99, 999)),
        "a fresh scan fills a session with no identity entry"
    );
    // Stale scans (outside the coverage window) are never trusted.
    let mut stale = scan("kimi", "s3", at, false);
    stale.scanned_at = at - RETENTION_SECS - 1;
    file.scans.push(stale);
    assert!(!trusted_identity_baseline(&file, at).contains_key("kimi\u{1}/tmp/ws\u{1}s3"));
    std::fs::remove_dir_all(&home).ok();
}

#[test]
fn concurrent_writers_never_expose_a_partial_file() {
    let home = home();
    let at = 100;
    let path = samples_path(&home);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();

    // A reader keeps parsing the raw file; with unique per-writer temps and
    // an atomic rename it must always see a complete JSON document.
    let reader = std::thread::spawn({
        let path = path.clone();
        move || {
            for _ in 0..400 {
                if let Ok(text) = std::fs::read_to_string(&path) {
                    serde_json::from_str::<NetworkSamplesFile>(&text)
                        .expect("reader saw a partial or corrupt ring file");
                }
            }
        }
    });
    let writers: Vec<_> = (0..4)
        .map(|_| {
            let home = home.clone();
            std::thread::spawn(move || {
                for round in 0..25 {
                    append_samples(
                        &home,
                        vec![sample("claude", "s1", at + round)],
                        vec![scan("claude", "s1", at + round, false)],
                        at + round,
                    )
                    .expect("concurrent append must succeed");
                }
            })
        })
        .collect();
    for writer in writers {
        writer.join().unwrap();
    }
    reader.join().unwrap();

    // Every read stays valid and no writer temp files are left behind.
    let file = read_samples(&home);
    assert!(!file.samples.is_empty());
    assert_eq!(file.identities.len(), 1);
    let leftovers: Vec<_> = std::fs::read_dir(path.parent().unwrap())
        .unwrap()
        .flatten()
        .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
        .collect();
    assert!(
        leftovers.is_empty(),
        "writer temps must be renamed or removed"
    );
    std::fs::remove_dir_all(&home).ok();
}
