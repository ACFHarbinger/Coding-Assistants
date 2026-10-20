use super::*;
use hub::net_samples::NetworkScanStatus;
use std::os::unix::fs::symlink;
use std::path::Path;

/// Build a fake `/proc` tree: one process directory per pid with `exe`,
/// `cwd`, `stat`, `fd/`, and `net/tcp[6]`. Symlink targets are dangling on
/// purpose — `read_link` never touches the target.
struct FakeProc {
    root: std::path::PathBuf,
}

impl FakeProc {
    fn new(base: &Path) -> Self {
        let root = base.join("proc");
        Self { root }
    }

    fn add_process(&self, pid: u32, exe: &str, cwd: &str, start_time: u64) -> std::path::PathBuf {
        let dir = self.root.join(pid.to_string());
        std::fs::create_dir_all(dir.join("fd")).unwrap();
        std::fs::create_dir_all(dir.join("net")).unwrap();
        symlink(exe, dir.join("exe")).unwrap();
        symlink(cwd, dir.join("cwd")).unwrap();
        // comm "(name) with spaces" exercises the rsplit_once(')') parse.
        // Tail fields after ')': [0]=state, then fields 4..21 as numbers,
        // start_time at index 19 (field 22).
        std::fs::write(
            dir.join("stat"),
            format!("{pid} ((claude code)) R 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 {start_time} 20 21"),
        )
        .unwrap();
        dir
    }

    fn add_fd(&self, pid: u32, name: &str, target: &str) {
        symlink(
            target,
            self.root.join(pid.to_string()).join("fd").join(name),
        )
        .unwrap();
    }

    fn set_tcp(&self, pid: u32, tcp: &str) {
        std::fs::write(self.root.join(pid.to_string()).join("net").join("tcp"), tcp).unwrap();
    }

    fn set_tcp6(&self, pid: u32, tcp: &str) {
        std::fs::write(
            self.root.join(pid.to_string()).join("net").join("tcp6"),
            tcp,
        )
        .unwrap();
    }
}

fn scratch(tag: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("ca-net-sampler-{}-{tag}", std::process::id()))
}

fn managed_target(pid: Option<u32>) -> SampleTarget {
    SampleTarget {
        harness: "claude".to_string(),
        workspace: "/tmp/ws-a".to_string(),
        session_id: "sess-1".to_string(),
        mode: "managed".to_string(),
        pid,
    }
}

fn observed_target() -> SampleTarget {
    SampleTarget {
        harness: "vibe".to_string(),
        workspace: "/tmp/ws-b".to_string(),
        session_id: "sess-2".to_string(),
        mode: "observed".to_string(),
        pid: None,
    }
}

const TCP_HEADER: &str =
    "  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode\n";

#[test]
fn parses_socket_inodes_and_ignores_other_fd_targets() {
    assert_eq!(parse_socket_inode("socket:[405931]"), Some(405931));
    assert_eq!(parse_socket_inode("pipe:[123]"), None);
    assert_eq!(parse_socket_inode("/home/user/notes.txt"), None);
    assert_eq!(parse_socket_inode("socket:[abc]"), None);
}

#[test]
fn parses_stat_start_time_after_a_comm_with_parens_and_spaces() {
    let stat = "123 ((claude code)) R 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 9999 21";
    assert_eq!(parse_stat_start_time(stat), Some(9999));
    assert_eq!(parse_stat_start_time("no closing paren"), None);
}

#[test]
fn decodes_proc_addresses_little_endian_per_group() {
    // 0100007F = 127.0.0.1; 1F90 = 8080.
    assert_eq!(
        parse_proc_address("0100007F:1F90"),
        Some(("127.0.0.1".to_string(), 8080))
    );
    // 0100A8C0 = 192.168.0.1.
    assert_eq!(
        parse_proc_address("0100A8C0:0035"),
        Some(("192.168.0.1".to_string(), 53))
    );
    // IPv6 ::1: every 32-bit word little-endian, so the final word 1 is
    // "01000000"; decoded back to 16-bit groups.
    assert_eq!(
        parse_proc_address("00000000000000000000000001000000:0500"),
        Some(("0000:0000:0000:0000:0000:0000:0000:0001".to_string(), 1280))
    );
    assert_eq!(parse_proc_address("0100:1F90"), None); // wrong ip length
}

#[test]
fn tcp_rows_keep_established_sockets_only() {
    let text = format!(
        "{TCP_HEADER}   0: 0100007F:1F90 0A00000A:01BB 01 00000000:00000000 00:00000000 00000000  1000        0 405931 2 0000000000000000 100 0 0 10 0\n\
            1: 0100007F:1F90 0A00000A:01BB 06 00000000:00000000 00:00000000 00000000  1000        0 405932 2 0000000000000000 100 0 0 10 0\n"
    );
    let rows = parse_net_tcp_rows(&text, "tcp");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].inode, 405931);
    assert_eq!(rows[0].dest_ip, "10.0.0.10");
    assert_eq!(rows[0].dest_port, 443);
}

#[test]
fn managed_pid_produces_samples_matching_fd_inodes_to_tcp_rows() {
    let base = scratch("managed");
    std::fs::remove_dir_all(&base).ok();
    let proc = FakeProc::new(&base);
    proc.add_process(42, "/usr/bin/claude", "/tmp/ws-a", 100);
    proc.add_fd(42, "3", "socket:[405931]");
    proc.add_fd(42, "4", "pipe:[7]");
    proc.set_tcp(
        42,
        &format!(
            "{TCP_HEADER}   0: 0100007F:1F90 0A00000A:01BB 01 00000000:00000000 00:00000000 00000000  1000        0 405931 2 0000000000000000 100 0 0 10 0\n"
        ),
    );

    let (samples, scans) = scan_targets(
        &proc.root,
        &[managed_target(Some(42))],
        &Default::default(),
        1_000,
    );
    assert_eq!(samples.len(), 1);
    assert_eq!(samples[0].dest_ip, "10.0.0.10");
    assert_eq!(samples[0].dest_port, 443);
    assert_eq!(samples[0].pid_start_time, 100);
    assert!(!samples[0].fds_unreadable);
    assert_eq!(scans.len(), 1);
    assert_eq!(scans[0].socket_count, 1);
    assert_eq!(scans[0].status, NetworkScanStatus::Sampled);
    std::fs::remove_dir_all(&base).ok();
}

#[test]
fn a_recycled_managed_pid_is_a_visible_pid_reused_gap() {
    let base = scratch("reused");
    std::fs::remove_dir_all(&base).ok();
    let proc = FakeProc::new(&base);
    // Baseline recorded start time 100; the pid now shows start time 555.
    proc.add_process(42, "/usr/bin/claude", "/tmp/ws-a", 555);
    proc.add_fd(42, "3", "socket:[405931]");
    proc.set_tcp(
        42,
        &format!(
            "{TCP_HEADER}   0: 0100007F:1F90 0A00000A:01BB 01 00000000:00000000 00:00000000 00000000  1000        0 405931 2 0000000000000000 100 0 0 10 0\n"
        ),
    );

    let mut baseline = std::collections::BTreeMap::new();
    baseline.insert(
        "claude\u{1}/tmp/ws-a\u{1}sess-1".to_string(),
        (42u32, 100u64),
    );

    let (samples, scans) = scan_targets(&proc.root, &[managed_target(Some(42))], &baseline, 1_000);
    assert!(samples.is_empty());
    assert_eq!(scans.len(), 1);
    assert_eq!(scans[0].status, NetworkScanStatus::PidReused);
    assert_eq!(scans[0].socket_count, 0);
    assert!(scans[0].fds_unreadable);
    let next_baseline = scans
        .iter()
        .map(|scan| (scan.session_key(), (scan.pid, scan.pid_start_time)))
        .collect();
    let (next_samples, next_scans) = scan_targets(
        &proc.root,
        &[managed_target(Some(42))],
        &next_baseline,
        1_001,
    );
    assert!(
        next_samples.is_empty(),
        "a rejected PID must stay rejected on the next scan"
    );
    assert_eq!(next_scans[0].status, NetworkScanStatus::PidReused);
    std::fs::remove_dir_all(&base).ok();
}

#[test]
fn observed_sessions_attribute_by_exe_and_cwd_each_scan() {
    let base = scratch("observed");
    std::fs::remove_dir_all(&base).ok();
    let proc = FakeProc::new(&base);
    // Right harness binary, right workspace: attributes.
    proc.add_process(100, "/usr/local/bin/vibe", "/tmp/ws-b", 700);
    proc.add_fd(100, "3", "socket:[55]");
    proc.set_tcp6(
        100,
        &format!(
            "{TCP_HEADER}   0: 00000000000000000000000000000000:01BB 0000012A000000000000000001000000:0035 01 00000000:00000000 00:00000000 00000000  1000        0 55 1 0000000000000000 100 0 0 10 0\n"
        ),
    );
    // Right binary, wrong workspace: must not attribute.
    proc.add_process(101, "/usr/local/bin/vibe", "/tmp/other", 701);
    // Right workspace, wrong binary: must not attribute.
    proc.add_process(102, "/usr/bin/make", "/tmp/ws-b", 702);
    // A different harness binary in the same workspace: must not attribute.
    proc.add_process(103, "/usr/bin/claude", "/tmp/ws-b", 703);

    let (samples, scans) =
        scan_targets(&proc.root, &[observed_target()], &Default::default(), 1_000);
    assert_eq!(scans.len(), 1);
    assert_eq!(scans[0].pid, 100);
    assert_eq!(samples.len(), 1);
    assert_eq!(samples[0].protocol, "tcp6");
    assert_eq!(
        samples[0].dest_ip,
        "2a01:0000:0000:0000:0000:0000:0000:0001"
    );
    assert_eq!(samples[0].dest_port, 53);
    std::fs::remove_dir_all(&base).ok();
}

#[test]
fn an_unreadable_fd_table_is_a_field_not_an_empty_list() {
    let base = scratch("unreadable");
    std::fs::remove_dir_all(&base).ok();
    let proc = FakeProc::new(&base);
    let dir = proc.add_process(42, "/usr/bin/claude", "/tmp/ws-a", 100);
    // `fd` exists but is a regular file, so read_dir fails: the fd table is
    // unreadable. The scan must still be recorded with fds_unreadable.
    std::fs::remove_dir(dir.join("fd")).unwrap();
    std::fs::write(dir.join("fd"), "not a directory").unwrap();
    proc.set_tcp(
        42,
        &format!(
            "{TCP_HEADER}   0: 0100007F:1F90 0A00000A:01BB 01 00000000:00000000 00:00000000 00000000  1000        0 405931 2 0000000000000000 100 0 0 10 0\n"
        ),
    );

    let (samples, scans) = scan_targets(
        &proc.root,
        &[managed_target(Some(42))],
        &Default::default(),
        1_000,
    );
    assert!(samples.is_empty());
    assert_eq!(scans.len(), 1);
    assert!(scans[0].fds_unreadable);
    assert_eq!(scans[0].socket_count, 0);
    assert_eq!(scans[0].status, NetworkScanStatus::Sampled);
    std::fs::remove_dir_all(&base).ok();
}

#[test]
fn a_dead_managed_pid_is_skipped_without_a_scan_record() {
    let base = scratch("dead");
    std::fs::remove_dir_all(&base).ok();
    let proc = FakeProc::new(&base);
    std::fs::create_dir_all(&proc.root).unwrap();
    let (samples, scans) = scan_targets(
        &proc.root,
        &[managed_target(Some(42))],
        &Default::default(),
        1_000,
    );
    assert!(samples.is_empty());
    assert!(scans.is_empty());
    std::fs::remove_dir_all(&base).ok();
}
