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
