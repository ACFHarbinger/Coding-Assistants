import { nocPanelStyle } from "./panel";
import { NOC_COLORS, NOC_SURFACES, NOC_TYPOGRAPHY } from "./tokens";

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
        <span style={{ fontSize: "0.84rem", fontWeight: 600, color: NOC_SURFACES.textPrimary }}>{title}</span>
        {series[0] && series[0].values.length > 0 && (
          <span className="tabular-nums" style={{ fontSize: "0.75rem", fontWeight: 700, color: series[0].color }}>
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
          fill={NOC_SURFACES.textMuted}
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
              stroke={NOC_SURFACES.gridline}
              strokeWidth="1"
            />
            <text x={padLeft - 6} y={tick.y + 3} fill={NOC_SURFACES.textMuted} fontSize="9.5" textAnchor="end">
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
                stroke={NOC_SURFACES.gridlineSubtle}
                strokeWidth="1"
              />
              {ts !== undefined && (
                <text
                  x={x}
                  y={padTop + plotH + 17}
                  fill={NOC_SURFACES.textMuted}
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
            stroke={NOC_COLORS.orange}
            strokeWidth="1.1"
            strokeDasharray="4 3"
          />
        )}
      </svg>

      <div style={{ display: "flex", flexWrap: "wrap", gap: "0.85rem", fontSize: NOC_TYPOGRAPHY.micro, color: NOC_SURFACES.textMuted }}>
        {series.map((s) => {
          const latest = s.values.length > 0 ? s.values[s.values.length - 1] : 0;
          const peak = s.values.length > 0 ? Math.max(...s.values) : 0;
          const fmt =
            s.formatter ??
            ((v: number) => `${v.toFixed(1)}${unitLabel === "%" ? "%" : ` ${unitLabel}`}`);
          return (
            <span key={s.label} className="tabular-nums" style={{ display: "inline-flex", alignItems: "center", gap: "0.35rem" }}>
              <i style={{ width: 9, height: 9, borderRadius: 2, background: s.color, display: "inline-block" }} />
              <span style={{ color: NOC_SURFACES.textSecondary, fontWeight: 600 }}>{s.label}</span>
              <span>now {fmt(latest)} · peak {fmt(peak)}</span>
            </span>
          );
        })}
        {avgValue !== null && (
          <span className="tabular-nums" style={{ display: "inline-flex", alignItems: "center", gap: "0.35rem" }}>
            <i style={{ width: 9, height: 2, background: NOC_COLORS.orange, display: "inline-block" }} />
            <span>Avg {avgValue.toFixed(1)}%</span>
          </span>
        )}
      </div>
    </div>
  );
}
