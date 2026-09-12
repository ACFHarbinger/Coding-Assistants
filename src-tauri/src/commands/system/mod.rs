//! Host system resource snapshot (U23 / #316, D7).
//!
//! Read-only observability for the resource pressure of running several
//! agent harnesses at once: total + per-core CPU, RAM, swap, per-volume disk
//! space + I/O counters, NVIDIA GPU/VRAM, and per-process CPU/memory with
//! the app's own managed harness PIDs attributed by harness and workspace.
//!
//! CPU/RAM/swap/disks/processes come from `sysinfo` (MIT, pinned — the
//! standard library cannot enumerate these cross-platform, so this is the
//! DEPENDENCY_POLICY rule-1 exception with the rule-3 single-purpose
//! justification: one crate for all host telemetry). One persistent
//! `System` lives behind a process-wide lock so CPU percentages and I/O
//! counters are deltas against the previous refresh rather than a
//! first-read zero; the 200 ms two-read trick is unnecessary because the
//! frontend polls every few seconds anyway.
//!
//! GPU is an `nvidia-smi` subprocess with explicit argv on a dedicated
//! thread plus `recv_timeout`, mirroring `hub::github::run_gh` —
//! best-effort only. A missing binary or non-NVIDIA host yields
//! `available: false` with an actionable detail, never a snapshot failure.
//! No process control (kill/renice/affinity) exists here or anywhere else
//! in v1; the command is free/local and takes no `allow_metered_quota_probes`
//! gate.

use serde::Serialize;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const NVIDIA_TIMEOUT: Duration = Duration::from_secs(8);
const TOP_PROCESSES: usize = 8;
const MIB: u64 = 1024 * 1024;

#[derive(Debug, Clone, Serialize)]
pub struct CpuCore {
    pub name: String,
    pub usage_percent: f32,
    pub freq_mhz: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemoryInfo {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SwapInfo {
    pub total_bytes: u64,
    pub used_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiskInfo {
    pub name: String,
    pub mount_point: String,
    pub file_system: String,
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub read_bytes_total: u64,
    pub written_bytes_total: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct GpuInfo {
    pub index: u32,
    pub name: String,
    pub usage_percent: f32,
    pub mem_used_bytes: u64,
    pub mem_total_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct GpuStatus {
    pub available: bool,
    pub detail: Option<String>,
    pub gpus: Vec<GpuInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WatchedProcess {
    pub pid: u32,
    pub name: String,
    pub cpu_percent: f32,
    pub mem_bytes: u64,
    /// Harness id (e.g. `kimi`) when this pid is a managed session.
    pub harness: Option<String>,
    pub workspace: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemSnapshot {
    pub fetched_at: i64,
    pub cpu_total_percent: f32,
    pub cpus: Vec<CpuCore>,
    pub memory: MemoryInfo,
    pub swap: SwapInfo,
    pub disks: Vec<DiskInfo>,
    pub gpu: GpuStatus,
    /// Managed harness processes first, then top CPU consumers.
    pub processes: Vec<WatchedProcess>,
}

struct RefreshState {
    sys: sysinfo::System,
    disks: sysinfo::Disks,
}

static STATE: OnceLock<Mutex<RefreshState>> = OnceLock::new();

fn state() -> Result<std::sync::MutexGuard<'static, RefreshState>, String> {
    STATE
        .get_or_init(|| {
            Mutex::new(RefreshState {
                sys: sysinfo::System::new_all(),
                disks: sysinfo::Disks::new_with_refreshed_list(),
            })
        })
        .lock()
        .map_err(|_| "system telemetry lock is poisoned".to_string())
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn mib_to_bytes(mib: f32) -> u64 {
    if mib.is_finite() && mib >= 0.0 {
        (mib as f64 * MIB as f64).round() as u64
    } else {
        0
    }
}

/// Parse one `nvidia-smi --format=csv,noheader,nounits` line. The GPU name
/// itself may contain commas, so the index is the first field, the three
/// numeric readings are the last three, and everything between is the name.
fn parse_gpu_line(line: &str) -> Option<GpuInfo> {
    let fields: Vec<&str> = line.split(',').map(str::trim).collect();
    if fields.len() < 5 {
        return None;
    }
    let index: u32 = fields[0].parse().ok()?;
    let total = fields.len();
    let usage: f32 = fields[total - 3].parse().ok()?;
    let mem_used = mib_to_bytes(fields[total - 2].parse().ok()?);
    let mem_total = mib_to_bytes(fields[total - 1].parse().ok()?);
    let name = fields[1..total - 3].join(", ").trim().to_string();
    if name.is_empty() || !(0.0..=100.0).contains(&usage) {
        return None;
    }
    Some(GpuInfo {
        index,
        name,
        usage_percent: usage,
        mem_used_bytes: mem_used,
        mem_total_bytes: mem_total,
    })
}

fn parse_gpu_csv(stdout: &str) -> Vec<GpuInfo> {
    stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter_map(parse_gpu_line)
        .collect()
}

/// Best-effort NVIDIA query on a dedicated thread with a timeout. Any
/// transport, auth-shape, or parse failure degrades to `available: false` —
/// never a snapshot failure.
fn nvidia_status() -> GpuStatus {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let output = std::process::Command::new("nvidia-smi")
            .args([
                "--query-gpu=index,name,utilization.gpu,memory.used,memory.total",
                "--format=csv,noheader,nounits",
            ])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output();
        let _ = tx.send(output);
    });
    match rx.recv_timeout(NVIDIA_TIMEOUT) {
        Ok(Ok(output)) if output.status.success() => {
            let gpus = parse_gpu_csv(&String::from_utf8_lossy(&output.stdout));
            if gpus.is_empty() {
                GpuStatus {
                    available: false,
                    detail: Some("nvidia-smi answered but reported no GPUs".to_string()),
                    gpus: Vec::new(),
                }
            } else {
                GpuStatus {
                    available: true,
                    detail: None,
                    gpus,
                }
            }
        }
        Ok(Ok(output)) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            GpuStatus {
                available: false,
                detail: Some(format!(
                    "nvidia-smi failed: {}",
                    stderr.trim().chars().take(200).collect::<String>()
                )),
                gpus: Vec::new(),
            }
        }
        Ok(Err(error)) => GpuStatus {
            available: false,
            detail: Some(format!(
                "nvidia-smi not found ({error}); GPU telemetry needs an NVIDIA host"
            )),
            gpus: Vec::new(),
        },
        Err(_) => GpuStatus {
            available: false,
            detail: Some("nvidia-smi timed out".to_string()),
            gpus: Vec::new(),
        },
    }
}

/// `(pid, harness, workspace)` for every registered managed session. A store
/// failure degrades to an empty list — the snapshot must not fail because
/// the roster is unreadable.
fn managed_pids() -> Vec<(u32, String, String)> {
    let Ok(store) = super::store::open_store() else {
        return Vec::new();
    };
    let Ok(sessions) = store.list_harness_sessions() else {
        return Vec::new();
    };
    sessions
        .into_iter()
        .filter_map(|session| {
            session
                .managed_pid
                .map(|pid| (pid, session.harness, session.workspace))
        })
        .collect()
}

fn collect_snapshot() -> Result<SystemSnapshot, String> {
    // Scope the global lock to the sysinfo reads: the `nvidia-smi` call
    // below can block for seconds and must not hold it.
    let (cpu_total, cpus, memory, swap, disks, processes) = {
        let mut state = state()?;
        state.sys.refresh_memory();
        state.sys.refresh_cpu_all();
        state
            .sys
            .refresh_processes(sysinfo::ProcessesToUpdate::All, true);
        state.disks.refresh(false);

        let cpus: Vec<CpuCore> = state
            .sys
            .cpus()
            .iter()
            .map(|cpu| CpuCore {
                name: cpu.name().to_string(),
                usage_percent: cpu.cpu_usage(),
                freq_mhz: cpu.frequency(),
            })
            .collect();

        // Pseudo-filesystems (overlay, tmpfs artefacts) report zero totals
        // and would clutter the UI with meaningless rows.
        let mut disks: Vec<DiskInfo> = state
            .disks
            .list()
            .iter()
            .filter(|disk| disk.total_space() > 0)
            .map(|disk| {
                let usage = disk.usage();
                DiskInfo {
                    name: disk.name().to_string_lossy().into_owned(),
                    mount_point: disk.mount_point().to_string_lossy().into_owned(),
                    file_system: disk.file_system().to_string_lossy().into_owned(),
                    total_bytes: disk.total_space(),
                    available_bytes: disk.available_space(),
                    read_bytes_total: usage.total_read_bytes,
                    written_bytes_total: usage.total_written_bytes,
                }
            })
            .collect();
        disks.sort_by(|a, b| a.mount_point.cmp(&b.mount_point));

        let managed = managed_pids();
        let mut seen = std::collections::HashSet::new();
        let mut processes: Vec<WatchedProcess> = Vec::new();
        for (pid, harness, workspace) in &managed {
            let pid_sys = sysinfo::Pid::from_u32(*pid);
            if let Some(process) = state.sys.process(pid_sys) {
                seen.insert(*pid);
                processes.push(WatchedProcess {
                    pid: *pid,
                    name: process.name().to_string_lossy().into_owned(),
                    cpu_percent: process.cpu_usage(),
                    mem_bytes: process.memory(),
                    harness: Some(harness.clone()),
                    workspace: Some(workspace.clone()),
                });
            }
        }
        let mut top: Vec<WatchedProcess> = state
            .sys
            .processes()
            .iter()
            .filter(|(pid, _)| !seen.contains(&pid.as_u32()))
            .map(|(pid, process)| WatchedProcess {
                pid: pid.as_u32(),
                name: process.name().to_string_lossy().into_owned(),
                cpu_percent: process.cpu_usage(),
                mem_bytes: process.memory(),
                harness: None,
                workspace: None,
            })
            .collect();
        top.sort_by(|a, b| {
            b.cpu_percent
                .partial_cmp(&a.cpu_percent)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        processes.extend(top.into_iter().take(TOP_PROCESSES));

        (
            state.sys.global_cpu_usage(),
            cpus,
            MemoryInfo {
                total_bytes: state.sys.total_memory(),
                used_bytes: state.sys.used_memory(),
                available_bytes: state.sys.available_memory(),
            },
            SwapInfo {
                total_bytes: state.sys.total_swap(),
                used_bytes: state.sys.used_swap(),
            },
            disks,
            processes,
        )
    };

    Ok(SystemSnapshot {
        fetched_at: now_unix(),
        cpu_total_percent: cpu_total,
        cpus,
        memory,
        swap,
        disks,
        gpu: nvidia_status(),
        processes,
    })
}

/// Async + `spawn_blocking`: `collect_snapshot` blocks on sysinfo refreshes
/// and the `nvidia-smi` read, so it stays off the webview dispatch thread
/// (same class as `hub_get_provider_quotas`).
#[tauri::command]
pub async fn system_snapshot() -> Result<SystemSnapshot, String> {
    tauri::async_runtime::spawn_blocking(collect_snapshot)
        .await
        .map_err(|error| format!("system snapshot task panicked: {error}"))?
}

#[cfg(test)]
#[path = "system_tests.rs"]
mod tests;
