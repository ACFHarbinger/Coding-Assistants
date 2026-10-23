import {
  type ArcRingSpec,
  ConcentricArcGauge,
  Meter,
  ProvenanceBadge,
  type ProvenanceKind,
} from "../../common/telemetry/gauges";
import { nocPanelStyle } from "../../common/telemetry/panel";
import { KpiCardGrid, MonitorTileStrip } from "../../common/telemetry/status";
import { NOC_COLORS, NOC_SURFACES } from "../../common/telemetry/tokens";

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

export {
  type ArcRingSpec,
  ConcentricArcGauge,
  Meter,
  nocPanelStyle,
  ProvenanceBadge,
  type ProvenanceKind,
};

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

export function MonitorStatusStrip({
  cpuTotal,
  cpus,
  memory,
  swap,
  disks,
  gpu,
  harnessProcesses,
}: {
  cpuTotal: number;
  cpus: CpuCore[];
  memory: MemoryInfo;
  swap: SwapInfo;
  disks: DiskInfo[];
  gpu: GpuStatus;
  harnessProcesses: WatchedProcess[];
}) {
  const memPct = memory.total_bytes > 0 ? (memory.used_bytes / memory.total_bytes) * 100 : 0;
  const swapPct = swap.total_bytes > 0 ? (swap.used_bytes / swap.total_bytes) * 100 : 0;

  const tiles = [
    { id: "cpu-total", title: "CPU-Total", subtitle: `${cpuTotal.toFixed(0)}%`, percent: cpuTotal },
    ...cpus.slice(0, 8).map((c) => ({
      id: `core-${c.name}`,
      title: `Core ${c.name}`,
      subtitle: `${c.usage_percent.toFixed(0)}% · ${c.freq_mhz}MHz`,
      percent: c.usage_percent,
    })),
    { id: "mem-ram", title: "RAM-Main", subtitle: `${memPct.toFixed(0)}%`, percent: memPct },
    ...(swap.total_bytes > 0
      ? [{ id: "mem-swap", title: "Swap-Vol", subtitle: `${swapPct.toFixed(0)}%`, percent: swapPct }]
      : []),
    ...disks.slice(0, 4).map((d) => {
      const used = Math.max(0, d.total_bytes - d.available_bytes);
      const pct = d.total_bytes > 0 ? (used / d.total_bytes) * 100 : 0;
      return { id: `disk-${d.mount_point}`, title: `Vol ${d.mount_point}`, subtitle: `${pct.toFixed(0)}% used`, percent: pct };
    }),
    ...(gpu.available && gpu.gpus.length > 0
      ? gpu.gpus.map((g) => ({ id: `gpu-${g.index}`, title: `GPU-${g.index}`, subtitle: `${g.usage_percent.toFixed(0)}% core`, percent: g.usage_percent }))
      : [{ id: "gpu-na", title: "GPU-Host", subtitle: "Standby", percent: 0, available: false }]),
    ...harnessProcesses.slice(0, 4).map((p) => ({
      id: `proc-${p.pid}`,
      title: p.harness ?? p.name,
      subtitle: `#${p.pid}`,
      percent: Math.min(100, p.cpu_percent),
    })),
  ];

  return <MonitorTileStrip tiles={tiles} />;
}

export function KpiMetricStrip({
  cpuTotal,
  cpus,
  memory,
  disks,
  ioRates,
  gpu,
  harnessCount,
  processCount,
}: {
  cpuTotal: number;
  cpus: CpuCore[];
  memory: MemoryInfo;
  disks: DiskInfo[];
  ioRates: Record<string, { read: number; write: number }>;
  gpu: GpuStatus;
  harnessCount: number;
  processCount: number;
}) {
  const maxFreq = cpus.reduce((max, c) => Math.max(max, c.freq_mhz), 0);
  const busiestCore = cpus.reduce<CpuCore | null>(
    (best, c) => (!best || c.usage_percent > best.usage_percent ? c : best),
    null,
  );
  const memPct = memory.total_bytes > 0 ? (memory.used_bytes / memory.total_bytes) * 100 : 0;
  const totalReadRate = Object.values(ioRates).reduce((acc, r) => acc + (Number.isFinite(r.read) ? r.read : 0), 0);
  const totalWriteRate = Object.values(ioRates).reduce((acc, r) => acc + (Number.isFinite(r.write) ? r.write : 0), 0);
  const combinedIo = totalReadRate + totalWriteRate;
  const primaryGpu = gpu.available && gpu.gpus.length > 0 ? gpu.gpus[0] : null;

  return (
    <KpiCardGrid
      cards={[
        {
          title: "Host CPU Utilization",
          subtitle: "Global Processor Load",
          value: cpuTotal.toFixed(1),
          unit: "%",
          footer: `${cpus.length} logical core${cpus.length === 1 ? "" : "s"}`,
          accent: cpuTotal >= 90 ? NOC_COLORS.crit : NOC_COLORS.cyan,
        },
        {
          title: "Peak Core Frequency",
          subtitle: busiestCore ? `Busiest: ${busiestCore.name} (${busiestCore.usage_percent.toFixed(0)}%)` : "Clock Telemetry",
          value: maxFreq > 0 ? String(maxFreq) : "—",
          unit: maxFreq > 0 ? "MHz" : "",
          footer: maxFreq > 0 ? `${(maxFreq / 1000).toFixed(2)} GHz peak clock` : "Frequency nominal",
          accent: NOC_COLORS.orange,
        },
        {
          title: "Physical Memory Used",
          subtitle: `${formatBytes(memory.used_bytes)} of ${formatBytes(memory.total_bytes)}`,
          value: memPct.toFixed(1),
          unit: "%",
          footer: `Avail: ${formatBytes(memory.available_bytes)}`,
          accent: NOC_COLORS.pink,
        },
        {
          title: "Aggregate Disk I/O",
          subtitle: `↓ ${formatRate(totalReadRate)} · ↑ ${formatRate(totalWriteRate)}`,
          value: combinedIo > 0 ? formatBytes(combinedIo).split(" ")[0] : "0.0",
          unit: combinedIo > 0 ? `${formatBytes(combinedIo).split(" ")[1] ?? "B"}/s` : "B/s",
          footer: `${disks.length} mounted volume${disks.length === 1 ? "" : "s"}`,
          accent: NOC_COLORS.ok,
        },
        {
          title: "GPU Accelerator",
          subtitle: primaryGpu ? primaryGpu.name : "Host GPU Status",
          value: primaryGpu ? primaryGpu.usage_percent.toFixed(0) : "N/A",
          unit: primaryGpu ? "%" : "",
          footer: primaryGpu
            ? `VRAM ${formatBytes(primaryGpu.mem_used_bytes)} / ${formatBytes(primaryGpu.mem_total_bytes)}`
            : "No NVIDIA runtime",
          accent: NOC_COLORS.purple,
        },
        {
          title: "Managed Harnesses",
          subtitle: "Active Agent Sessions",
          value: String(harnessCount),
          unit: "active",
          footer: `${processCount} total watched PIDs`,
          accent: NOC_COLORS.emerald,
        },
      ]}
    />
  );
}

export function CpuCoreRadialGauge({ core, maxFreqMhz }: { core: CpuCore; maxFreqMhz: number }) {
  const size = 92;
  const center = size / 2;
  const outerR = 39;
  const innerR = 29;
  const arcSpan = 260;
  const startAngle = 140;
  const outerCirc = 2 * Math.PI * outerR;
  const innerCirc = 2 * Math.PI * innerR;
  const outerArc = (arcSpan / 360) * outerCirc;
  const innerArc = (arcSpan / 360) * innerCirc;

  const loadClamped = Math.max(0, Math.min(100, core.usage_percent));
  const freqRatio = maxFreqMhz > 0 ? Math.max(10, Math.min(100, (core.freq_mhz / maxFreqMhz) * 100)) : 50;
  const loadColor = loadClamped >= 90 ? NOC_COLORS.crit : loadClamped >= 70 ? NOC_COLORS.warn : NOC_COLORS.emerald;

  return (
    <div style={{ display: "flex", flexDirection: "column", alignItems: "center", background: "rgba(15, 23, 42, 0.55)", border: `1px solid ${NOC_SURFACES.border}`, borderRadius: "8px", padding: "0.35rem" }}>
      <div style={{ position: "relative", width: size, height: size, display: "grid", placeItems: "center" }}>
        <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`} role="img" aria-label={`${core.name} usage ${loadClamped.toFixed(0)}%`}>
          <g transform={`rotate(${startAngle} ${center} ${center})`}>
            <circle cx={center} cy={center} r={outerR} fill="none" stroke={NOC_SURFACES.track} strokeWidth={5.5} strokeDasharray={`${outerArc} ${outerCirc}`} strokeLinecap="round" />
            <circle cx={center} cy={center} r={outerR} fill="none" stroke={NOC_COLORS.crit} strokeWidth={5.5} strokeDasharray={`${(freqRatio / 100) * outerArc} ${outerCirc}`} strokeLinecap="round" />
            <circle cx={center} cy={center} r={innerR} fill="none" stroke={NOC_SURFACES.track} strokeWidth={5.5} strokeDasharray={`${innerArc} ${innerCirc}`} strokeLinecap="round" />
            <circle cx={center} cy={center} r={innerR} fill="none" stroke={loadColor} strokeWidth={5.5} strokeDasharray={`${(loadClamped / 100) * innerArc} ${innerCirc}`} strokeLinecap="round" />
          </g>
        </svg>
        <div className="tabular-nums" style={{ position: "absolute", display: "flex", flexDirection: "column", alignItems: "center", lineHeight: 1.15, textAlign: "center" }}>
          <span style={{ fontSize: "0.68rem", fontWeight: 700, color: NOC_SURFACES.textPrimary }}>{core.name}</span>
          <span style={{ fontSize: "0.6rem", color: NOC_SURFACES.textMuted }}>{core.freq_mhz}MHz</span>
          <span style={{ fontSize: "0.7rem", fontWeight: 800, color: loadColor }}>{loadClamped.toFixed(0)}%</span>
        </div>
      </div>
    </div>
  );
}
