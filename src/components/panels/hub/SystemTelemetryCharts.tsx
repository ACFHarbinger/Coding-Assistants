import {
  type DiskInfo,
  formatBytes,
  formatRate,
  type GpuStatus,
  Meter,
  nocPanelStyle,
  type WatchedProcess,
} from "./SystemGauges";

export type TelemetrySeries = {
  label: string;
  color: string;
  fill?: string;
  values: number[];
  formatter?: (val: number) => string;
};

function formatShortTime(unixSeconds: number): string {
  if (!Number.isFinite(unixSeconds) || unixSeconds <= 0) return "";
  const date = new Date(unixSeconds * 1000);
  return date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: "2-digit" });
}

/**
 * Site24x7-inspired dark NOC time-series chart card with gridlines, y-axis unit,
 * time ticks, translucent area fill, mean reference line, and legend strip.
 */
export function TelemetryTimeSeriesCard({
  title,
  unitLabel,
  series,
  timestamps,
  fixedMax,
  showAverageLine = false,
  historyCapacity = 60,
}: {
  title: string;
  unitLabel: string;
  series: TelemetrySeries[];
  timestamps: number[];
  fixedMax?: number;
  showAverageLine?: boolean;
  historyCapacity?: number;
}) {
  const svgWidth = 440;
  const svgHeight = 176;
  const padLeft = 44;
  const padRight = 14;
  const padTop = 12;
  const padBottom = 28;
  const plotW = svgWidth - padLeft - padRight;
  const plotH = svgHeight - padTop - padBottom;

  const allValues = series.flatMap((s) => s.values).filter((v) => Number.isFinite(v));
  const rawMax = allValues.length > 0 ? Math.max(...allValues) : 100;
  const yMax = fixedMax ?? (rawMax > 0 ? rawMax * 1.15 : 100);

  const yTicks = [0, 0.25, 0.5, 0.75, 1].map((ratio) => ({
    value: yMax * ratio,
    y: padTop + plotH - ratio * plotH,
  }));

  const xTickIndices = [0, 0.33, 0.66, 1].map((r) =>
    timestamps.length > 1 ? Math.round(r * (timestamps.length - 1)) : -1,
  );

  const primaryValues = series[0]?.values ?? [];
  const avgValue =
    showAverageLine && primaryValues.length > 0
      ? primaryValues.reduce((acc, v) => acc + v, 0) / primaryValues.length
      : null;
  const avgY =
    avgValue !== null && yMax > 0
      ? padTop + plotH - Math.max(0, Math.min(1, avgValue / yMax)) * plotH
      : null;

  const formatTick = (val: number) => {
    if (yMax >= 100) return String(Math.round(val));
    if (yMax >= 10) return val.toFixed(1);
    return val.toFixed(2);
  };

  return (
    <div
      style={{ ...nocPanelStyle, display: "flex", flexDirection: "column", gap: "0.55rem" }}
      data-testid="telemetry-timeseries-card"
    >
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", gap: "0.5rem" }}>
        <span style={{ fontSize: "0.84rem", fontWeight: 600, color: "#f1f5f9" }}>{title}</span>
        {series[0] && series[0].values.length > 0 && (
          <span style={{ fontSize: "0.75rem", fontWeight: 700, color: series[0].color }}>
            {series[0].formatter
              ? series[0].formatter(series[0].values[series[0].values.length - 1])
              : `${series[0].values[series[0].values.length - 1].toFixed(1)}${unitLabel === "%" ? "%" : ` ${unitLabel}`}`}
          </span>
        )}
      </div>

      <svg
        viewBox={`0 0 ${svgWidth} ${svgHeight}`}
        width="100%"
        role="img"
        aria-label={title}
        style={{ display: "block", overflow: "visible" }}
      >
        <text
          x={11}
          y={padTop + plotH / 2}
          fill="#64748b"
          fontSize="10"
          textAnchor="middle"
          transform={`rotate(-90 11 ${padTop + plotH / 2})`}
        >
          {unitLabel}
        </text>

        {yTicks.map((tick, idx) => (
          <g key={idx}>
            <line
              x1={padLeft}
              y1={tick.y}
              x2={padLeft + plotW}
              y2={tick.y}
              stroke="rgba(148, 163, 184, 0.14)"
              strokeWidth="1"
            />
            <text x={padLeft - 6} y={tick.y + 3} fill="#94a3b8" fontSize="9.5" textAnchor="end">
              {formatTick(tick.value)}
            </text>
          </g>
        ))}

        {[0, 0.33, 0.66, 1].map((ratio, idx) => {
          const x = padLeft + ratio * plotW;
          const tsIdx = xTickIndices[idx];
          const ts = tsIdx >= 0 ? timestamps[tsIdx] : undefined;
          return (
            <g key={idx}>
              <line
                x1={x}
                y1={padTop}
                x2={x}
                y2={padTop + plotH}
                stroke="rgba(148, 163, 184, 0.08)"
                strokeWidth="1"
              />
              {ts !== undefined && (
                <text
                  x={x}
                  y={padTop + plotH + 17}
                  fill="#94a3b8"
                  fontSize="9"
                  textAnchor={idx === 0 ? "start" : idx === 3 ? "end" : "middle"}
                >
                  {formatShortTime(ts)}
                </text>
              )}
            </g>
          );
        })}

        {series.map((s) => {
          if (s.values.length === 0) return null;
          const denom = Math.max(1, Math.min(historyCapacity - 1, Math.max(1, s.values.length - 1)));
          const coords = s.values.map((val, i) => {
            const x = s.values.length === 1 ? padLeft + plotW : padLeft + (i / denom) * plotW;
            const clampedRatio = yMax > 0 ? Math.max(0, Math.min(1, val / yMax)) : 0;
            const y = padTop + plotH - clampedRatio * plotH;
            return { x, y };
          });
          const linePoints = coords.map((pt) => `${pt.x.toFixed(1)},${pt.y.toFixed(1)}`).join(" ");
          const firstX = coords[0].x.toFixed(1);
          const lastX = coords[coords.length - 1].x.toFixed(1);
          const baseY = (padTop + plotH).toFixed(1);
          const areaPoints = `${firstX},${baseY} ${linePoints} ${lastX},${baseY}`;

          return (
            <g key={s.label}>
              {s.fill && <polygon points={areaPoints} fill={s.fill} />}
              <polyline
                points={linePoints}
                fill="none"
                stroke={s.color}
                strokeWidth="1.8"
                strokeLinejoin="round"
                strokeLinecap="round"
              />
            </g>
          );
        })}

        {avgY !== null && (
          <line
            x1={padLeft}
            y1={avgY}
            x2={padLeft + plotW}
            y2={avgY}
            stroke="#f97316"
            strokeWidth="1.1"
            strokeDasharray="4 3"
          />
        )}
      </svg>

      <div style={{ display: "flex", flexWrap: "wrap", gap: "0.85rem", fontSize: "0.72rem", color: "#94a3b8" }}>
        {series.map((s) => {
          const latest = s.values.length > 0 ? s.values[s.values.length - 1] : 0;
          const peak = s.values.length > 0 ? Math.max(...s.values) : 0;
          const fmt =
            s.formatter ??
            ((v: number) => `${v.toFixed(1)}${unitLabel === "%" ? "%" : ` ${unitLabel}`}`);
          return (
            <span key={s.label} style={{ display: "inline-flex", alignItems: "center", gap: "0.35rem" }}>
              <i style={{ width: 9, height: 9, borderRadius: 2, background: s.color, display: "inline-block" }} />
              <span style={{ color: "#e2e8f0", fontWeight: 600 }}>{s.label}</span>
              <span>now {fmt(latest)} · peak {fmt(peak)}</span>
            </span>
          );
        })}
        {avgValue !== null && (
          <span style={{ display: "inline-flex", alignItems: "center", gap: "0.35rem" }}>
            <i style={{ width: 9, height: 2, background: "#f97316", display: "inline-block" }} />
            <span>Avg {avgValue.toFixed(1)}%</span>
          </span>
        )}
      </div>
    </div>
  );
}

/**
 * Bottom NOC detail panels for Disks, GPU, Managed Harness Processes, and Top CPU Processes.
 */
export function SystemDetailPanels({
  disks,
  ioRates,
  gpu,
  gpuFallbackDetail,
  harnessProcesses,
  topProcesses,
}: {
  disks: DiskInfo[];
  ioRates: Record<string, { read: number; write: number }>;
  gpu: GpuStatus | undefined;
  gpuFallbackDetail: string;
  harnessProcesses: WatchedProcess[];
  topProcesses: WatchedProcess[];
}) {
  return (
    <>
      <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(300px, 1fr))", gap: "1rem" }}>
        <div style={nocPanelStyle}>
          <h3 style={{ margin: "0 0 0.85rem", fontSize: "0.95rem", color: "var(--text-main)" }}>Disks</h3>
          <div style={{ display: "flex", flexDirection: "column", gap: "0.75rem" }}>
            {disks.map((disk) => {
              const used = disk.total_bytes - disk.available_bytes;
              const percent = disk.total_bytes > 0 ? (used / disk.total_bytes) * 100 : 0;
              const rate = ioRates[disk.mount_point];
              return (
                <div key={disk.mount_point}>
                  <div style={{ display: "flex", justifyContent: "space-between", fontSize: "0.8rem", color: "var(--text-muted)", flexWrap: "wrap", gap: "0.5rem" }}>
                    <span>
                      <strong style={{ color: "var(--text-main)" }}>{disk.mount_point}</strong> · {disk.file_system}
                      {disk.name ? ` · ${disk.name}` : ""}
                    </span>
                    <span>
                      {formatBytes(used)} / {formatBytes(disk.total_bytes)} · ↓ {formatRate(rate?.read ?? NaN)} ↑ {formatRate(rate?.write ?? NaN)}
                    </span>
                  </div>
                  <div style={{ height: 7, background: "rgba(148, 163, 184, 0.14)", borderRadius: 4, marginTop: "0.3rem" }}>
                    <div
                      style={{
                        width: `${Math.min(100, percent)}%`,
                        height: "100%",
                        borderRadius: 4,
                        background: percent > 90 ? "#ef4444" : "#10b981",
                      }}
                    />
                  </div>
                </div>
              );
            })}
            {disks.length === 0 && (
              <span style={{ color: "var(--text-muted)", fontSize: "0.85rem" }}>No volumes reported yet.</span>
            )}
          </div>
        </div>

        <div style={nocPanelStyle}>
          <h3 style={{ margin: "0 0 0.85rem", fontSize: "0.95rem", color: "var(--text-main)" }}>GPU</h3>
          {gpu?.available ? (
            <div style={{ display: "flex", flexDirection: "column", gap: "0.75rem" }}>
              {gpu.gpus.map((g) => (
                <Meter
                  key={g.index}
                  label={`${g.name} · VRAM`}
                  color="#a855f7"
                  percent={g.mem_total_bytes > 0 ? (g.mem_used_bytes / g.mem_total_bytes) * 100 : 0}
                  detail={`core ${g.usage_percent.toFixed(0)}% · ${formatBytes(g.mem_used_bytes)} / ${formatBytes(g.mem_total_bytes)}`}
                />
              ))}
            </div>
          ) : (
            <span style={{ color: "var(--text-muted)", fontSize: "0.85rem" }}>{gpuFallbackDetail}</span>
          )}
        </div>
      </div>

      <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(300px, 1fr))", gap: "1rem" }}>
        <div style={nocPanelStyle}>
          <h3 style={{ margin: "0 0 0.85rem", fontSize: "0.95rem", color: "var(--text-main)" }}>Managed harness processes</h3>
          {harnessProcesses.length === 0 && (
            <span style={{ color: "var(--text-muted)", fontSize: "0.85rem" }}>No managed harness processes running.</span>
          )}
          {harnessProcesses.map((proc) => (
            <div
              key={proc.pid}
              style={{
                display: "flex",
                justifyContent: "space-between",
                alignItems: "center",
                gap: "1rem",
                borderTop: "1px solid var(--border-color)",
                padding: "0.55rem 0",
                fontSize: "0.84rem",
                flexWrap: "wrap",
              }}
            >
              <span style={{ color: "var(--text-main)", display: "inline-flex", alignItems: "center", gap: "0.45rem" }}>
                <i style={{ width: 7, height: 7, borderRadius: "50%", background: "#22c55e", display: "inline-block" }} />
                <strong>{proc.harness}</strong> · {proc.name}{" "}
                <span style={{ color: "var(--text-muted)" }}>pid {proc.pid}</span>
              </span>
              <span style={{ color: "var(--text-muted)" }}>
                CPU {proc.cpu_percent.toFixed(1)}% · {formatBytes(proc.mem_bytes)}
              </span>
            </div>
          ))}
        </div>

        <div style={nocPanelStyle}>
          <h3 style={{ margin: "0 0 0.85rem", fontSize: "0.95rem", color: "var(--text-main)" }}>Top processes by CPU</h3>
          {topProcesses.map((proc) => (
            <div
              key={proc.pid}
              style={{
                display: "flex",
                justifyContent: "space-between",
                alignItems: "center",
                gap: "1rem",
                borderTop: "1px solid var(--border-color)",
                padding: "0.55rem 0",
                fontSize: "0.84rem",
                flexWrap: "wrap",
              }}
            >
              <span style={{ color: "var(--text-main)" }}>
                {proc.name} <span style={{ color: "var(--text-muted)" }}>pid {proc.pid}</span>
              </span>
              <span style={{ color: "var(--text-muted)" }}>
                CPU {proc.cpu_percent.toFixed(1)}% · {formatBytes(proc.mem_bytes)}
              </span>
            </div>
          ))}
        </div>
      </div>
    </>
  );
}
