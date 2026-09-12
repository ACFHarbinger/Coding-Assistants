import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "../../../lib/tauri";

export type CpuCore = { name: string; usage_percent: number; freq_mhz: number };
export type MemoryInfo = { total_bytes: number; used_bytes: number; available_bytes: number };
export type SwapInfo = { total_bytes: number; used_bytes: number };
export type DiskInfo = {
  name: string;
  mount_point: string;
  file_system: string;
  total_bytes: number;
  available_bytes: number;
  read_bytes_total: number;
  written_bytes_total: number;
};
export type GpuInfo = {
  index: number;
  name: string;
  usage_percent: number;
  mem_used_bytes: number;
  mem_total_bytes: number;
};
export type GpuStatus = { available: boolean; detail: string | null; gpus: GpuInfo[] };
export type WatchedProcess = {
  pid: number;
  name: string;
  cpu_percent: number;
  mem_bytes: number;
  harness: string | null;
  workspace: string | null;
};
export type SystemSnapshot = {
  fetched_at: number;
  cpu_total_percent: number;
  cpus: CpuCore[];
  memory: MemoryInfo;
  swap: SwapInfo;
  disks: DiskInfo[];
  gpu: GpuStatus;
  processes: WatchedProcess[];
};

const POLL_MS = 3000;
const HISTORY = 60;

export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const rank = Math.min(units.length - 1, Math.floor(Math.log(bytes) / Math.log(1024)));
  const value = bytes / 1024 ** rank;
  return `${value >= 100 ? Math.round(value) : value.toFixed(1)} ${units[rank]}`;
}

export function formatRate(bytesPerSecond: number): string {
  if (!Number.isFinite(bytesPerSecond) || bytesPerSecond <= 0) return "—";
  return `${formatBytes(bytesPerSecond)}/s`;
}

function Meter({ label, percent, detail }: { label: string; percent: number; detail: string }) {
  const clamped = Math.max(0, Math.min(100, percent));
  return (
    <div>
      <div style={{ display: "flex", justifyContent: "space-between", fontSize: "0.8rem", color: "var(--text-muted)" }}>
        <span>{label}</span>
        <span>{detail}</span>
      </div>
      <div style={{ height: 8, background: "rgba(255,255,255,0.08)", borderRadius: 4, marginTop: "0.3rem" }}>
        <div style={{ width: `${clamped}%`, height: "100%", borderRadius: 4, background: clamped > 90 ? "#ef4444" : "var(--primary)" }} />
      </div>
    </div>
  );
}

export function Sparkline({ values, label }: { values: number[]; label: string }) {
  const width = 120;
  const height = 28;
  if (values.length < 2) return null;
  const points = values
    .map((value, i) => {
      const x = (i / (HISTORY - 1)) * width;
      const y = height - (Math.max(0, Math.min(100, value)) / 100) * height;
      return `${x.toFixed(1)},${y.toFixed(1)}`;
    })
    .join(" ");
  return (
    <svg width={width} height={height} role="img" aria-label={label}>
      <polyline points={points} fill="none" stroke="var(--primary)" strokeWidth="1.5" />
    </svg>
  );
}

const cardStyle: React.CSSProperties = {
  border: "1px solid var(--border-color)",
  borderRadius: "12px",
  padding: "1.25rem",
  background: "rgba(0,0,0,0.3)",
};

export default function SystemTab() {
  const [snapshot, setSnapshot] = useState<SystemSnapshot | null>(null);
  const [error, setError] = useState("");
  const [cpuHistory, setCpuHistory] = useState<number[]>([]);
  const [memHistory, setMemHistory] = useState<number[]>([]);
  const [ioRates, setIoRates] = useState<Record<string, { read: number; write: number }>>({});
  const previous = useRef<SystemSnapshot | null>(null);
  const inFlight = useRef(false);

  const refresh = useCallback(async () => {
    if (inFlight.current) return;
    inFlight.current = true;
    try {
      const next = await invoke<SystemSnapshot>("system_snapshot");
      setSnapshot(next);
      setError("");
      setCpuHistory((history) => [...history, next.cpu_total_percent].slice(-HISTORY));
      const memPercent =
        next.memory.total_bytes > 0 ? (next.memory.used_bytes / next.memory.total_bytes) * 100 : 0;
      setMemHistory((history) => [...history, memPercent].slice(-HISTORY));
      const prev = previous.current;
      if (prev) {
        const dt = Math.max(1, next.fetched_at - prev.fetched_at);
        const rates: Record<string, { read: number; write: number }> = {};
        for (const disk of next.disks) {
          const old = prev.disks.find((d) => d.mount_point === disk.mount_point);
          if (old) {
            rates[disk.mount_point] = {
              read: Math.max(0, (disk.read_bytes_total - old.read_bytes_total) / dt),
              write: Math.max(0, (disk.written_bytes_total - old.written_bytes_total) / dt),
            };
          }
        }
        setIoRates(rates);
      }
      previous.current = next;
    } catch (cause) {
      setError(String(cause));
    } finally {
      inFlight.current = false;
    }
  }, []);

  useEffect(() => {
    void refresh();
    const timer = window.setInterval(() => void refresh(), POLL_MS);
    return () => window.clearInterval(timer);
  }, [refresh]);

  const memPercent =
    snapshot && snapshot.memory.total_bytes > 0
      ? (snapshot.memory.used_bytes / snapshot.memory.total_bytes) * 100
      : 0;
  const harnessProcesses = (snapshot?.processes ?? []).filter((p) => p.harness);
  const topProcesses = (snapshot?.processes ?? []).filter((p) => !p.harness);

  return (
    <div className="fade-in" style={{ display: "flex", flexDirection: "column", gap: "1.5rem" }}>
      <div style={{ ...cardStyle, display: "flex", justifyContent: "space-between", gap: "1rem", alignItems: "center", flexWrap: "wrap" }}>
        <div>
          <h3 style={{ margin: 0, color: "var(--text-main)" }}>System Resources</h3>
          <p style={{ margin: "0.4rem 0 0", color: "var(--text-muted)", fontSize: "0.85rem" }}>
            Live host telemetry, refreshed every {POLL_MS / 1000}s. Read-only — no process control.
            {snapshot && <> Last fetch: {new Date(snapshot.fetched_at * 1000).toLocaleTimeString()}</>}
          </p>
        </div>
        <button className="btn-secondary" onClick={() => void refresh()}>Refresh now</button>
      </div>
      {error && <div style={{ ...cardStyle, color: "#ef4444" }}>{error}</div>}

      <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(220px, 1fr))", gap: "1rem" }}>
        <div style={cardStyle}>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <strong style={{ color: "var(--text-main)" }}>CPU · {snapshot ? snapshot.cpu_total_percent.toFixed(1) : "—"}%</strong>
            <Sparkline values={cpuHistory} label="Total CPU usage history" />
          </div>
          <div style={{ marginTop: "0.6rem", display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(90px, 1fr))", gap: "0.5rem" }}>
            {(snapshot?.cpus ?? []).map((cpu) => (
              <div key={cpu.name} style={{ fontSize: "0.75rem", color: "var(--text-muted)" }}>
                <div style={{ display: "flex", justifyContent: "space-between" }}>
                  <span>{cpu.name}</span>
                  <span>{cpu.usage_percent.toFixed(0)}%</span>
                </div>
                <div style={{ height: 5, background: "rgba(255,255,255,0.08)", borderRadius: 3, marginTop: "0.2rem" }}>
                  <div style={{ width: `${Math.min(100, cpu.usage_percent)}%`, height: "100%", borderRadius: 3, background: "var(--primary)" }} />
                </div>
              </div>
            ))}
          </div>
        </div>

        <div style={{ ...cardStyle, display: "flex", flexDirection: "column", gap: "0.9rem" }}>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <strong style={{ color: "var(--text-main)" }}>Memory · {memPercent.toFixed(1)}%</strong>
            <Sparkline values={memHistory} label="Memory usage history" />
          </div>
          {snapshot && (
            <>
              <Meter label="RAM" percent={memPercent} detail={`${formatBytes(snapshot.memory.used_bytes)} / ${formatBytes(snapshot.memory.total_bytes)}`} />
              {snapshot.swap.total_bytes > 0 && (
                <Meter
                  label="Swap"
                  percent={(snapshot.swap.used_bytes / snapshot.swap.total_bytes) * 100}
                  detail={`${formatBytes(snapshot.swap.used_bytes)} / ${formatBytes(snapshot.swap.total_bytes)}`}
                />
              )}
            </>
          )}
        </div>
      </div>

      <div style={cardStyle}>
        <h3 style={{ margin: "0 0 1rem", color: "var(--text-main)" }}>Disks</h3>
        <div style={{ display: "flex", flexDirection: "column", gap: "0.75rem" }}>
          {(snapshot?.disks ?? []).map((disk) => {
            const used = disk.total_bytes - disk.available_bytes;
            const percent = disk.total_bytes > 0 ? (used / disk.total_bytes) * 100 : 0;
            const rate = ioRates[disk.mount_point];
            return (
              <div key={disk.mount_point}>
                <div style={{ display: "flex", justifyContent: "space-between", fontSize: "0.8rem", color: "var(--text-muted)", flexWrap: "wrap", gap: "0.5rem" }}>
                  <span><strong style={{ color: "var(--text-main)" }}>{disk.mount_point}</strong> · {disk.file_system}{disk.name ? ` · ${disk.name}` : ""}</span>
                  <span>{formatBytes(used)} / {formatBytes(disk.total_bytes)} · ↓ {formatRate(rate?.read ?? NaN)} ↑ {formatRate(rate?.write ?? NaN)}</span>
                </div>
                <div style={{ height: 7, background: "rgba(255,255,255,0.08)", borderRadius: 4, marginTop: "0.3rem" }}>
                  <div style={{ width: `${Math.min(100, percent)}%`, height: "100%", borderRadius: 4, background: percent > 90 ? "#ef4444" : "var(--primary)" }} />
                </div>
              </div>
            );
          })}
          {(snapshot?.disks ?? []).length === 0 && <span style={{ color: "var(--text-muted)", fontSize: "0.85rem" }}>No volumes reported yet.</span>}
        </div>
      </div>

      <div style={cardStyle}>
        <h3 style={{ margin: "0 0 1rem", color: "var(--text-main)" }}>GPU</h3>
        {snapshot?.gpu.available ? (
          <div style={{ display: "flex", flexDirection: "column", gap: "0.75rem" }}>
            {snapshot.gpu.gpus.map((gpu) => (
              <Meter
                key={gpu.index}
                label={`${gpu.name} · VRAM`}
                percent={gpu.mem_total_bytes > 0 ? (gpu.mem_used_bytes / gpu.mem_total_bytes) * 100 : 0}
                detail={`core ${gpu.usage_percent.toFixed(0)}% · ${formatBytes(gpu.mem_used_bytes)} / ${formatBytes(gpu.mem_total_bytes)}`}
              />
            ))}
          </div>
        ) : (
          <span style={{ color: "var(--text-muted)", fontSize: "0.85rem" }}>
            {snapshot?.gpu.detail ?? "GPU telemetry unavailable on this host."}
          </span>
        )}
      </div>

      <div style={cardStyle}>
        <h3 style={{ margin: "0 0 1rem", color: "var(--text-main)" }}>Managed harness processes</h3>
        {harnessProcesses.length === 0 && <span style={{ color: "var(--text-muted)", fontSize: "0.85rem" }}>No managed harness processes running.</span>}
        {harnessProcesses.map((proc) => (
          <div key={proc.pid} style={{ display: "flex", justifyContent: "space-between", gap: "1rem", borderTop: "1px solid var(--border-color)", padding: "0.5rem 0", fontSize: "0.85rem", flexWrap: "wrap" }}>
            <span style={{ color: "var(--text-main)" }}><strong>{proc.harness}</strong> · {proc.name} <span style={{ color: "var(--text-muted)" }}>pid {proc.pid}</span></span>
            <span style={{ color: "var(--text-muted)" }}>CPU {proc.cpu_percent.toFixed(1)}% · {formatBytes(proc.mem_bytes)}</span>
          </div>
        ))}
      </div>

      <div style={cardStyle}>
        <h3 style={{ margin: "0 0 1rem", color: "var(--text-main)" }}>Top processes by CPU</h3>
        {topProcesses.map((proc) => (
          <div key={proc.pid} style={{ display: "flex", justifyContent: "space-between", gap: "1rem", borderTop: "1px solid var(--border-color)", padding: "0.5rem 0", fontSize: "0.85rem", flexWrap: "wrap" }}>
            <span style={{ color: "var(--text-main)" }}>{proc.name} <span style={{ color: "var(--text-muted)" }}>pid {proc.pid}</span></span>
            <span style={{ color: "var(--text-muted)" }}>CPU {proc.cpu_percent.toFixed(1)}% · {formatBytes(proc.mem_bytes)}</span>
          </div>
        ))}
      </div>
    </div>
  );
}
