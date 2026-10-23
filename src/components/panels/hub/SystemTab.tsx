import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "../../../lib/tauri";
import { NOC_COLORS } from "../../common/telemetry/tokens";
import {
  ConcentricArcGauge,
  type CpuCore,
  CpuCoreRadialGauge,
  type DiskInfo,
  formatBytes,
  formatRate,
  type GpuInfo,
  type GpuStatus,
  KpiMetricStrip,
  type MemoryInfo,
  Meter,
  MonitorStatusStrip,
  nocPanelStyle,
  type SwapInfo,
  type SystemSnapshot,
  type WatchedProcess,
} from "./SystemGauges";
import { SystemDetailPanels, TelemetryTimeSeriesCard } from "./SystemTelemetryCharts";

export {
  type CpuCore,
  type DiskInfo,
  formatBytes,
  formatRate,
  type GpuInfo,
  type GpuStatus,
  type MemoryInfo,
  Meter,
  type SwapInfo,
  type SystemSnapshot,
  type WatchedProcess,
};

const POLL_MS = 3000;
const HISTORY = 60;

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

export default function SystemTab() {
  const [snapshot, setSnapshot] = useState<SystemSnapshot | null>(null);
  const [error, setError] = useState("");
  const [cpuHistory, setCpuHistory] = useState<number[]>([]);
  const [memHistory, setMemHistory] = useState<number[]>([]);
  const [swapHistory, setSwapHistory] = useState<number[]>([]);
  const [readRateHistory, setReadRateHistory] = useState<number[]>([]);
  const [writeRateHistory, setWriteRateHistory] = useState<number[]>([]);
  const [timestamps, setTimestamps] = useState<number[]>([]);
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
      setTimestamps((prevTs) => [...prevTs, next.fetched_at].slice(-HISTORY));
      setCpuHistory((history) => [...history, next.cpu_total_percent].slice(-HISTORY));
      const memPercent =
        next.memory.total_bytes > 0 ? (next.memory.used_bytes / next.memory.total_bytes) * 100 : 0;
      const swapPercent =
        next.swap.total_bytes > 0 ? (next.swap.used_bytes / next.swap.total_bytes) * 100 : 0;
      setMemHistory((history) => [...history, memPercent].slice(-HISTORY));
      setSwapHistory((history) => [...history, swapPercent].slice(-HISTORY));

      const prev = previous.current;
      let totalRead = 0;
      let totalWrite = 0;
      if (prev) {
        const dt = Math.max(1, next.fetched_at - prev.fetched_at);
        const rates: Record<string, { read: number; write: number }> = {};
        for (const disk of next.disks) {
          const old = prev.disks.find((d) => d.mount_point === disk.mount_point);
          if (old) {
            const r = Math.max(0, (disk.read_bytes_total - old.read_bytes_total) / dt);
            const w = Math.max(0, (disk.written_bytes_total - old.written_bytes_total) / dt);
            rates[disk.mount_point] = { read: r, write: w };
            totalRead += r;
            totalWrite += w;
          }
        }
        setIoRates(rates);
      }
      setReadRateHistory((history) => [...history, totalRead / 1024].slice(-HISTORY));
      setWriteRateHistory((history) => [...history, totalWrite / 1024].slice(-HISTORY));
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
  const swapPercent =
    snapshot && snapshot.swap.total_bytes > 0
      ? (snapshot.swap.used_bytes / snapshot.swap.total_bytes) * 100
      : 0;
  const maxFreqMhz = (snapshot?.cpus ?? []).reduce((max, c) => Math.max(max, c.freq_mhz), 0);
  const harnessProcesses = (snapshot?.processes ?? []).filter((p) => p.harness);
  const topProcesses = (snapshot?.processes ?? []).filter((p) => !p.harness);

  const totalDiskBytes = (snapshot?.disks ?? []).reduce((acc, d) => acc + d.total_bytes, 0);
  const usedDiskBytes = (snapshot?.disks ?? []).reduce(
    (acc, d) => acc + Math.max(0, d.total_bytes - d.available_bytes),
    0,
  );
  const totalDiskPct = totalDiskBytes > 0 ? (usedDiskBytes / totalDiskBytes) * 100 : 0;
  const primaryGpu = snapshot?.gpu.available && snapshot.gpu.gpus.length > 0 ? snapshot.gpu.gpus[0] : null;
  const primaryGpuVramPct =
    primaryGpu && primaryGpu.mem_total_bytes > 0
      ? (primaryGpu.mem_used_bytes / primaryGpu.mem_total_bytes) * 100
      : 0;

  return (
    <div className="fade-in" style={{ display: "flex", flexDirection: "column", gap: "1rem" }}>
      <div
        style={{
          ...nocPanelStyle,
          display: "flex",
          justifyContent: "space-between",
          gap: "1rem",
          alignItems: "center",
          flexWrap: "wrap",
          padding: "0.9rem 1.15rem",
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: "0.85rem", flexWrap: "wrap" }}>
          <div style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
            <span
              className="pulse-indicator"
              style={{
                width: 9,
                height: 9,
                background: error ? "#ef4444" : "#10b981",
                display: "inline-block",
              }}
            />
            <h3 style={{ margin: 0, fontSize: "1.05rem", color: "var(--text-main)", letterSpacing: "0.01em" }}>
              System Resources
            </h3>
          </div>
          <span style={{ color: "var(--text-muted)", fontSize: "0.8rem" }}>
            ↻ Live host telemetry ({POLL_MS / 1000}s cadence · read-only)
            {snapshot && <> · Last updated {new Date(snapshot.fetched_at * 1000).toLocaleTimeString()}</>}
          </span>
        </div>
        <button
          className="btn-secondary"
          style={{ marginTop: 0, padding: "0.45rem 0.95rem", fontSize: "0.82rem" }}
          onClick={() => void refresh()}
        >
          Refresh now
        </button>
      </div>

      {error && (
        <div style={{ ...nocPanelStyle, borderColor: "rgba(239, 68, 68, 0.45)", color: "#ef4444" }}>
          {error}
        </div>
      )}

      {snapshot && (
        <>
          <MonitorStatusStrip
            cpuTotal={snapshot.cpu_total_percent}
            cpus={snapshot.cpus}
            memory={snapshot.memory}
            swap={snapshot.swap}
            disks={snapshot.disks}
            gpu={snapshot.gpu}
            harnessProcesses={harnessProcesses}
          />
          <KpiMetricStrip
            cpuTotal={snapshot.cpu_total_percent}
            cpus={snapshot.cpus}
            memory={snapshot.memory}
            disks={snapshot.disks}
            ioRates={ioRates}
            gpu={snapshot.gpu}
            harnessCount={harnessProcesses.length}
            processCount={snapshot.processes.length}
          />
        </>
      )}

      <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(280px, 1fr))", gap: "1rem" }}>
        <div style={{ ...nocPanelStyle, display: "flex", flexDirection: "column", gap: "0.85rem" }}>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", gap: "0.5rem" }}>
            <div>
              <strong style={{ color: "var(--text-main)", fontSize: "0.9rem" }}>
                CPU · {snapshot ? snapshot.cpu_total_percent.toFixed(1) : "—"}%
              </strong>
              <div style={{ fontSize: "0.72rem", color: "var(--text-muted)", marginTop: "0.15rem" }}>
                Outer ring: clock frequency · Inner ring: core load %
              </div>
            </div>
            <Sparkline values={cpuHistory} label="Total CPU usage history" />
          </div>
          <div
            style={{
              display: "grid",
              gridTemplateColumns: "repeat(auto-fill, minmax(92px, 1fr))",
              gap: "0.5rem",
              maxHeight: "235px",
              overflowY: "auto",
              paddingRight: "0.2rem",
            }}
          >
            {(snapshot?.cpus ?? []).map((cpu) => (
              <CpuCoreRadialGauge key={cpu.name} core={cpu} maxFreqMhz={maxFreqMhz} />
            ))}
          </div>
        </div>

        <div style={{ ...nocPanelStyle, display: "flex", flexDirection: "column", justifyContent: "space-between", gap: "0.85rem" }}>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <strong style={{ color: "var(--text-main)", fontSize: "0.9rem" }}>
              Memory · {memPercent.toFixed(1)}%
            </strong>
            <Sparkline values={memHistory} label="Memory usage history" />
          </div>
          <ConcentricArcGauge
            title="Mem"
            centerPrimary={`${memPercent.toFixed(0)}%`}
            centerSecondary={snapshot && snapshot.swap.total_bytes > 0 ? `${swapPercent.toFixed(0)}% swp` : undefined}
            rings={[
              {
                label: "RAM",
                percent: memPercent,
                color: NOC_COLORS.pink,
                detail: snapshot
                  ? `${formatBytes(snapshot.memory.used_bytes)} / ${formatBytes(snapshot.memory.total_bytes)}`
                  : "—",
              },
              ...(snapshot && snapshot.swap.total_bytes > 0
                ? [
                    {
                      label: "Swap",
                      percent: swapPercent,
                      color: NOC_COLORS.teal,
                      detail: `${formatBytes(snapshot.swap.used_bytes)} / ${formatBytes(snapshot.swap.total_bytes)}`,
                    },
                  ]
                : []),
            ]}
          />
          {snapshot && (
            <div style={{ display: "flex", flexDirection: "column", gap: "0.55rem" }}>
              <Meter
                label="RAM"
                percent={memPercent}
                color={NOC_COLORS.pink}
                detail={`${formatBytes(snapshot.memory.used_bytes)} / ${formatBytes(snapshot.memory.total_bytes)}`}
              />
              {snapshot.swap.total_bytes > 0 && (
                <Meter
                  label="Swap"
                  percent={swapPercent}
                  color={NOC_COLORS.teal}
                  detail={`${formatBytes(snapshot.swap.used_bytes)} / ${formatBytes(snapshot.swap.total_bytes)}`}
                />
              )}
            </div>
          )}
        </div>

        <div style={{ ...nocPanelStyle, display: "flex", flexDirection: "column", justifyContent: "space-between", gap: "0.85rem" }}>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <strong style={{ color: "var(--text-main)", fontSize: "0.9rem" }}>GPU &amp; Storage Dials</strong>
            <span style={{ fontSize: "0.72rem", color: snapshot?.gpu.available ? NOC_COLORS.ok : "var(--text-muted)" }}>
              {snapshot?.gpu.available ? "Accelerator Online" : "Host Volumes"}
            </span>
          </div>
          <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(130px, 1fr))", gap: "0.5rem", alignItems: "center" }}>
            <ConcentricArcGauge
              title="GPU"
              size={142}
              centerPrimary={primaryGpu ? `${primaryGpu.usage_percent.toFixed(0)}%` : "N/A"}
              centerSecondary={primaryGpu ? `${primaryGpuVramPct.toFixed(0)}% mem` : undefined}
              rings={[
                {
                  label: "Core",
                  percent: primaryGpu ? primaryGpu.usage_percent : 0,
                  color: NOC_COLORS.ok,
                  detail: primaryGpu ? `${primaryGpu.usage_percent.toFixed(0)}%` : "idle",
                },
                {
                  label: "VRAM",
                  percent: primaryGpuVramPct,
                  color: NOC_COLORS.purple,
                  detail: primaryGpu ? formatBytes(primaryGpu.mem_used_bytes) : "—",
                },
              ]}
            />
            <ConcentricArcGauge
              title="Storage"
              size={142}
              centerPrimary={`${totalDiskPct.toFixed(0)}%`}
              centerSecondary={`${(snapshot?.disks ?? []).length} vol`}
              rings={(snapshot?.disks ?? []).slice(0, 3).map((d, idx) => {
                const used = Math.max(0, d.total_bytes - d.available_bytes);
                const pct = d.total_bytes > 0 ? (used / d.total_bytes) * 100 : 0;
                const palette = [NOC_COLORS.emerald, NOC_COLORS.cyan, NOC_COLORS.amber];
                return { label: d.mount_point, percent: pct, color: palette[idx % palette.length], detail: `${pct.toFixed(0)}%` };
              })}
            />
          </div>
          <div style={{ background: "rgba(0, 0, 0, 0.25)", borderRadius: "6px", padding: "0.55rem 0.7rem", fontSize: "0.76rem", display: "grid", gap: "0.25rem" }}>
            <div style={{ display: "flex", justifyContent: "space-between", color: "var(--text-muted)" }}>
              <span>Volume Health Status:</span>
              <strong style={{ color: totalDiskPct >= 90 ? NOC_COLORS.crit : NOC_COLORS.ok }}>{totalDiskPct >= 90 ? "HIGH USAGE" : "OK"}</strong>
            </div>
            <div style={{ display: "flex", justifyContent: "space-between", color: "var(--text-muted)" }}>
              <span>Used Space:</span>
              <strong style={{ color: "var(--noc-text-secondary)" }}>{totalDiskPct.toFixed(2)}% ({formatBytes(usedDiskBytes)} / {formatBytes(totalDiskBytes)})</strong>
            </div>
            <div style={{ display: "flex", justifyContent: "space-between", color: "var(--text-muted)", gap: "0.5rem" }}>
              <span>Accelerator State:</span>
              <span style={{ color: "var(--noc-text-secondary)", textAlign: "right" }}>
                {snapshot?.gpu.available ? `${snapshot.gpu.gpus.length} GPU(s) active` : "Standby (host CPU mode)"}
              </span>
            </div>
          </div>
        </div>
      </div>

      <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(290px, 1fr))", gap: "1rem" }}>
        <TelemetryTimeSeriesCard
          title="CPU Usage of Host Processor"
          unitLabel="%"
          fixedMax={100}
          showAverageLine
          timestamps={timestamps}
          series={[{ label: "Total CPU %", color: NOC_COLORS.cyan, fill: "rgba(56, 189, 248, 0.24)", values: cpuHistory }]}
        />
        <TelemetryTimeSeriesCard
          title="Memory & Swap Utilization"
          unitLabel="%"
          fixedMax={100}
          timestamps={timestamps}
          series={[
            { label: "RAM Used %", color: NOC_COLORS.pink, fill: "rgba(236, 72, 153, 0.22)", values: memHistory },
            { label: "Swap Used %", color: NOC_COLORS.teal, values: swapHistory },
          ]}
        />
        <TelemetryTimeSeriesCard
          title="Disk I/O Throughput (Read / Write)"
          unitLabel="KB/s"
          timestamps={timestamps}
          series={[
            { label: "Read Rate", color: NOC_COLORS.ok, fill: "rgba(16, 185, 129, 0.18)", values: readRateHistory, formatter: (v) => formatRate(v * 1024) },
            { label: "Write Rate", color: NOC_COLORS.amber, values: writeRateHistory, formatter: (v) => formatRate(v * 1024) },
          ]}
        />
      </div>

      <SystemDetailPanels
        disks={snapshot?.disks ?? []}
        ioRates={ioRates}
        gpu={snapshot?.gpu}
        gpuFallbackDetail={snapshot?.gpu.detail ?? "GPU telemetry unavailable on this host."}
        harnessProcesses={harnessProcesses}
        topProcesses={topProcesses}
      />
    </div>
  );
}
