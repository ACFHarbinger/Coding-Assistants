import type React from "react";
import { NOC_COLORS, NOC_ELEVATION, NOC_SURFACES, NOC_TYPOGRAPHY } from "../../common/telemetry/tokens";

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

export type ArcRingSpec = {
  label: string;
  percent: number;
  color: string;
  detail: string;
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

export function Meter({
  label,
  percent,
  detail,
  color = "var(--primary)",
}: {
  label: string;
  percent: number;
  detail: string;
  color?: string;
}) {
  const clamped = Math.max(0, Math.min(100, percent));
  return (
    <div>
      <div style={{ display: "flex", justifyContent: "space-between", fontSize: "0.78rem", color: "var(--text-muted)" }}>
        <span style={{ color: NOC_SURFACES.textSecondary, fontWeight: 600 }}>{label}</span>
        <span className="tabular-nums">{detail}</span>
      </div>
      <div style={{ height: 7, background: NOC_SURFACES.gridline, borderRadius: 4, marginTop: "0.28rem", overflow: "hidden" }}>
        <div
          style={{
            width: `${clamped}%`,
            height: "100%",
            borderRadius: 4,
            background: clamped > 90 ? NOC_COLORS.crit : color,
          }}
        />
      </div>
    </div>
  );
}

export const nocPanelStyle: React.CSSProperties = {
  background: "linear-gradient(180deg, rgba(15, 23, 42, 0.92) 0%, rgba(9, 14, 26, 0.96) 100%)",
  border: `1px solid ${NOC_SURFACES.border}`,
  borderRadius: "10px",
  padding: "1rem 1.15rem",
  boxShadow: NOC_ELEVATION.elev2,
};

function statusTone(percent: number, available = true): { bg: string; border: string; text: string } {
  if (!available || !Number.isFinite(percent)) {
    return { bg: "rgba(51, 65, 85, 0.65)", border: "rgba(100, 116, 139, 0.4)", text: "#cbd5e1" };
  }
  if (percent >= 90) {
    return { bg: "rgba(239, 68, 68, 0.88)", border: "#f87171", text: "#ffffff" };
  }
  if (percent >= 72) {
    return { bg: "rgba(234, 179, 8, 0.88)", border: "#facc15", text: NOC_SURFACES.bgPanel };
  }
  return { bg: "rgba(16, 185, 129, 0.88)", border: "#34d399", text: "#052e16" };
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

  const tiles: { id: string; title: string; subtitle: string; percent: number; available?: boolean }[] = [
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

  return (
    <div style={nocPanelStyle} data-testid="monitor-status-strip">
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "0.65rem", gap: "0.75rem", flexWrap: "wrap" }}>
        <span style={{ fontSize: "0.82rem", fontWeight: 600, color: NOC_SURFACES.textSecondary, letterSpacing: "0.02em" }}>
          Current Status of All Monitors
        </span>
        <div style={{ display: "flex", gap: "0.85rem", fontSize: NOC_TYPOGRAPHY.micro, color: "var(--text-muted)" }}>
          <span style={{ display: "inline-flex", alignItems: "center", gap: "0.3rem" }}>
            <i style={{ width: 8, height: 8, borderRadius: 2, background: NOC_COLORS.ok, display: "inline-block" }} /> Optimal (&lt;72%)
          </span>
          <span style={{ display: "inline-flex", alignItems: "center", gap: "0.3rem" }}>
            <i style={{ width: 8, height: 8, borderRadius: 2, background: NOC_COLORS.warn, display: "inline-block" }} /> Elevated (72–89%)
          </span>
          <span style={{ display: "inline-flex", alignItems: "center", gap: "0.3rem" }}>
            <i style={{ width: 8, height: 8, borderRadius: 2, background: NOC_COLORS.crit, display: "inline-block" }} /> Critical (≥90%)
          </span>
        </div>
      </div>
      <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(92px, 1fr))", gap: "0.4rem" }}>
        {tiles.map((tile) => {
          const tone = statusTone(tile.percent, tile.available ?? true);
          return (
            <div
              key={tile.id}
              title={`${tile.title}: ${tile.subtitle}`}
              style={{
                background: tone.bg,
                border: `1px solid ${tone.border}`,
                color: tone.text,
                borderRadius: "5px",
                padding: "0.35rem 0.45rem",
                display: "flex",
                flexDirection: "column",
                justifyContent: "center",
                minHeight: "42px",
                overflow: "hidden",
              }}
            >
              <div style={{ fontSize: "0.73rem", fontWeight: 700, whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>
                {tile.title}
              </div>
              <div style={{ fontSize: "0.66rem", fontWeight: 600, opacity: 0.9, whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>
                {tile.subtitle}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
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

  const cards = [
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
  ];

  return (
    <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(175px, 1fr))", gap: "0.75rem" }} data-testid="kpi-metric-strip">
      {cards.map((card) => (
        <div
          key={card.title}
          style={{
            ...nocPanelStyle,
            padding: "0.85rem 1rem",
            display: "flex",
            flexDirection: "column",
            justifyContent: "space-between",
            textAlign: "center",
            borderTop: `2px solid ${card.accent}`,
          }}
        >
          <div style={{ fontSize: "0.76rem", fontWeight: 600, color: NOC_SURFACES.textSecondary, whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>
            {card.title}
          </div>
          <div style={{ fontSize: "0.68rem", color: NOC_SURFACES.textMuted, marginTop: "0.2rem", whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>
            {card.subtitle}
          </div>
          <div className="tabular-nums" style={{ margin: "0.55rem 0 0.45rem", display: "flex", alignItems: "baseline", justifyContent: "center", gap: "0.2rem" }}>
            <span style={{ fontSize: "1.65rem", fontWeight: 800, color: NOC_SURFACES.textPrimary, letterSpacing: "-0.02em", lineHeight: 1 }}>
              {card.value}
            </span>
            {card.unit && <span style={{ fontSize: "0.75rem", fontWeight: 600, color: card.accent }}>{card.unit}</span>}
          </div>
          <div style={{ fontSize: "0.68rem", color: NOC_SURFACES.textMuted, whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>
            {card.footer}
          </div>
        </div>
      ))}
    </div>
  );
}

export function ConcentricArcGauge({
  title,
  centerPrimary,
  centerSecondary,
  rings,
  size = 168,
}: {
  title: string;
  centerPrimary: string;
  centerSecondary?: string;
  rings: ArcRingSpec[];
  size?: number;
}) {
  const center = size / 2;
  const strokeWidth = 11;
  const ringGap = 15;
  const outerRadius = center - strokeWidth;
  const arcSpanDeg = 265;
  const startAngleDeg = 137.5;

  return (
    <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: "0.55rem" }}>
      <div style={{ position: "relative", width: size, height: size, display: "grid", placeItems: "center" }}>
        {rings.map((ring) => {
          const clamped = Math.round(Math.max(0, Math.min(100, ring.percent)));
          return (
            <progress
              key={ring.label}
              className="ca-progress-ring"
              value={clamped}
              max={100}
              aria-label={`${title} ${ring.label}`}
              style={{ "--ring-value": clamped, position: "absolute", width: 1, height: 1, opacity: 0, pointerEvents: "none" } as React.CSSProperties}
            />
          );
        })}
        <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`} role="img" aria-label={`${title} radial gauge`}>
          {rings.map((ring, idx) => {
            const radius = Math.max(18, outerRadius - idx * ringGap);
            const circumference = 2 * Math.PI * radius;
            const arcLength = (arcSpanDeg / 360) * circumference;
            const clamped = Math.max(0, Math.min(100, ring.percent));
            const filledLength = (clamped / 100) * arcLength;
            return (
              <g key={ring.label} transform={`rotate(${startAngleDeg} ${center} ${center})`}>
                <circle cx={center} cy={center} r={radius} fill="none" stroke={NOC_SURFACES.track} strokeWidth={strokeWidth} strokeDasharray={`${arcLength} ${circumference}`} strokeLinecap="round" />
                {clamped > 0 && (
                  <circle cx={center} cy={center} r={radius} fill="none" stroke={ring.color} strokeWidth={strokeWidth} strokeDasharray={`${filledLength} ${circumference}`} strokeLinecap="round" />
                )}
              </g>
            );
          })}
        </svg>
        <div className="tabular-nums" style={{ position: "absolute", display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", textAlign: "center", pointerEvents: "none", padding: "0 0.5rem" }}>
          <span style={{ fontSize: "1.35rem", fontWeight: 800, color: NOC_SURFACES.textPrimary, lineHeight: 1.05 }}>{centerPrimary}</span>
          {centerSecondary && <span style={{ fontSize: "0.85rem", fontWeight: 700, color: NOC_COLORS.cyan, marginTop: "0.1rem" }}>{centerSecondary}</span>}
          <span style={{ fontSize: "0.78rem", fontWeight: 700, color: NOC_SURFACES.textSecondary, marginTop: "0.25rem", textTransform: "uppercase", letterSpacing: "0.04em" }}>{title}</span>
        </div>
      </div>
      <div style={{ display: "flex", flexWrap: "wrap", justifyContent: "center", gap: "0.65rem", fontSize: NOC_TYPOGRAPHY.micro, color: NOC_SURFACES.textMuted }}>
        {rings.map((ring) => (
          <span key={ring.label} className="tabular-nums" style={{ display: "inline-flex", alignItems: "center", gap: "0.3rem" }}>
            <i style={{ width: 8, height: 8, borderRadius: "50%", background: ring.color, display: "inline-block" }} />
            <strong style={{ color: NOC_SURFACES.textSecondary }}>{ring.label}:</strong> {ring.detail}
          </span>
        ))}
      </div>
    </div>
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
