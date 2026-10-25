import { useState, useId } from "react";
import { nocCardDenseStyle, nocPanelStyle } from "../panel";
import { NOC_COLORS, NOC_SURFACES, NOC_TYPOGRAPHY } from "../tokens";

export interface RadarAxis {
  key: string;
  label: string;
  max?: number;
  unit?: string;
}

export interface RadarSeries {
  id: string;
  name: string;
  color: string;
  values: Record<string, number>;
}

export interface OrionRadarChartProps {
  title?: string;
  subtitle?: string;
  axes: RadarAxis[];
  series: RadarSeries[];
  width?: number;
  height?: number;
  showLegend?: boolean;
}

/**
 * Radar / Spider-web comparison chart inspired by SetProduct Orion
 * (35-omparison_chart_dark.webp). Features concentric polygon webs,
 * glowing corner vertices, multi-series faceted fills, and interactive HUD tooltips.
 */
export function OrionRadarChart({
  title = "Telemetry Radar Profile",
  subtitle = "Multi-axis capability & performance comparison",
  axes,
  series,
  width = 460,
  height = 360,
  showLegend = true,
}: OrionRadarChartProps) {
  const [activeAxisKey, setActiveAxisKey] = useState<string | null>(null);
  const [hiddenSeriesIds, setHiddenSeriesIds] = useState<Set<string>>(new Set());
  const filterId = useId();

  const cx = width / 2;
  const cy = height / 2 + 10;
  const radius = Math.min(cx, cy) - 58;
  const numAxes = axes.length;

  if (numAxes < 3) {
    return (
      <div style={{ ...nocPanelStyle, color: NOC_SURFACES.textMuted, textAlign: "center", padding: "2rem" }}>
        Radar chart requires at least 3 axes.
      </div>
    );
  }

  const getAxisAngle = (index: number) => {
    return -Math.PI / 2 + (2 * Math.PI * index) / numAxes;
  };

  const ringLevels = [0.2, 0.4, 0.6, 0.8, 1.0];

  const getPolygonPoints = (scale: number) => {
    return axes
      .map((_, i) => {
        const angle = getAxisAngle(i);
        const x = cx + radius * scale * Math.cos(angle);
        const y = cy + radius * scale * Math.sin(angle);
        return `${x.toFixed(1)},${y.toFixed(1)}`;
      })
      .join(" ");
  };

  const getSeriesPoints = (s: RadarSeries) => {
    return axes
      .map((axis, i) => {
        const val = s.values[axis.key] ?? 0;
        const max = axis.max ?? 100;
        const ratio = Math.max(0, Math.min(1.1, val / (max || 1)));
        const angle = getAxisAngle(i);
        const x = cx + radius * ratio * Math.cos(angle);
        const y = cy + radius * ratio * Math.sin(angle);
        return `${x.toFixed(1)},${y.toFixed(1)}`;
      })
      .join(" ");
  };

  const toggleSeries = (id: string) => {
    setHiddenSeriesIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) {
        next.delete(id);
      } else {
        next.add(id);
      }
      return next;
    });
  };

  const activeAxis = axes.find((a) => a.key === activeAxisKey);

  return (
    <div
      style={{
        ...nocPanelStyle,
        position: "relative",
        display: "flex",
        flexDirection: "column",
        gap: "0.75rem",
      }}
      data-testid="orion-radar-chart"
    >
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start", gap: "1rem", flexWrap: "wrap" }}>
        <div>
          <h4 style={{ margin: 0, fontSize: "0.92rem", fontWeight: 700, color: NOC_SURFACES.textPrimary, letterSpacing: "0.02em" }}>
            {title}
          </h4>
          <p style={{ margin: "0.15rem 0 0", fontSize: NOC_TYPOGRAPHY.micro, color: NOC_SURFACES.textMuted }}>
            {subtitle}
          </p>
        </div>
        {showLegend && (
          <div style={{ display: "flex", alignItems: "center", gap: "0.5rem", flexWrap: "wrap" }}>
            {series.map((s) => {
              const isHidden = hiddenSeriesIds.has(s.id);
              return (
                <button
                  key={s.id}
                  type="button"
                  onClick={() => toggleSeries(s.id)}
                  aria-pressed={!isHidden}
                  style={{
                    background: isHidden ? "rgba(148, 163, 184, 0.08)" : `${s.color}1a`,
                    border: `1px solid ${isHidden ? "rgba(148, 163, 184, 0.2)" : s.color}`,
                    borderRadius: "999px",
                    padding: "0.2rem 0.55rem",
                    display: "inline-flex",
                    alignItems: "center",
                    gap: "0.35rem",
                    cursor: "pointer",
                    fontSize: NOC_TYPOGRAPHY.micro,
                    color: isHidden ? NOC_SURFACES.textMuted : NOC_SURFACES.textPrimary,
                    transition: "all 140ms ease",
                  }}
                >
                  <span
                    style={{
                      width: 7,
                      height: 7,
                      borderRadius: "50%",
                      backgroundColor: isHidden ? NOC_SURFACES.textMuted : s.color,
                      boxShadow: isHidden ? "none" : `0 0 6px ${s.color}`,
                    }}
                  />
                  <span>{s.name}</span>
                </button>
              );
            })}
          </div>
        )}
      </div>

      <div style={{ position: "relative", width: "100%", overflow: "hidden" }}>
        <svg
          viewBox={`0 0 ${width} ${height}`}
          width="100%"
          height="100%"
          role="img"
          aria-label={title}
          style={{ display: "block", overflow: "visible" }}
        >
          <defs>
            <filter id={`radar-glow-${filterId}`} x="-20%" y="-20%" width="140%" height="140%">
              <feGaussianBlur stdDeviation="3" result="blur" />
              <feComposite in="SourceGraphic" in2="blur" operator="over" />
            </filter>
            <radialGradient id={`radar-core-${filterId}`} cx="50%" cy="50%" r="50%">
              <stop offset="0%" stopColor="#38bdf8" stopOpacity="0.12" />
              <stop offset="100%" stopColor="#090e1a" stopOpacity="0" />
            </radialGradient>
          </defs>

          {/* Central ambient glow disc */}
          <circle cx={cx} cy={cy} r={radius} fill={`url(#radar-core-${filterId})`} />

          {/* Concentric polygon web rings */}
          {ringLevels.map((lvl) => (
            <polygon
              key={`ring-${lvl}`}
              points={getPolygonPoints(lvl)}
              fill="none"
              stroke={lvl === 1.0 ? "rgba(148, 163, 184, 0.28)" : "rgba(148, 163, 184, 0.12)"}
              strokeWidth={lvl === 1.0 ? 1.5 : 1}
              strokeDasharray={lvl < 1.0 ? "3 3" : undefined}
            />
          ))}

          {/* Radial axis rays and axis labels */}
          {axes.map((axis, i) => {
            const angle = getAxisAngle(i);
            const x2 = cx + radius * Math.cos(angle);
            const y2 = cy + radius * Math.sin(angle);
            const labelX = cx + (radius + 22) * Math.cos(angle);
            const labelY = cy + (radius + 18) * Math.sin(angle);
            const isHovered = activeAxisKey === axis.key;

            return (
              <g
                key={axis.key}
                tabIndex={0}
                role="button"
                aria-label={`Axis ${axis.label}`}
                onMouseEnter={() => setActiveAxisKey(axis.key)}
                onMouseLeave={() => setActiveAxisKey(null)}
                onFocus={() => setActiveAxisKey(axis.key)}
                onBlur={() => setActiveAxisKey(null)}
                style={{ cursor: "pointer", outline: "none" }}
              >
                <line
                  x1={cx}
                  y1={cy}
                  x2={x2}
                  y2={y2}
                  stroke={isHovered ? NOC_COLORS.cyan : "rgba(148, 163, 184, 0.22)"}
                  strokeWidth={isHovered ? 2 : 1}
                />
                <circle
                  cx={x2}
                  cy={y2}
                  r={isHovered ? 4.5 : 2.5}
                  fill={isHovered ? NOC_COLORS.cyan : NOC_SURFACES.bgPanel}
                  stroke={isHovered ? "#ffffff" : "rgba(148, 163, 184, 0.4)"}
                  strokeWidth={1.5}
                />
                <text
                  x={labelX}
                  y={labelY}
                  textAnchor="middle"
                  dominantBaseline="central"
                  fill={isHovered ? NOC_COLORS.cyan : NOC_SURFACES.textSecondary}
                  fontSize={10.5}
                  fontWeight={isHovered ? 700 : 500}
                >
                  {axis.label}
                </text>
              </g>
            );
          })}

          {/* Render series polygons */}
          {series.map((s) => {
            if (hiddenSeriesIds.has(s.id)) return null;
            const points = getSeriesPoints(s);

            return (
              <g key={s.id}>
                {/* Translucent faceted fill */}
                <polygon
                  points={points}
                  fill={s.color}
                  fillOpacity={0.22}
                  stroke={s.color}
                  strokeWidth={2}
                  strokeLinejoin="round"
                  filter={`url(#radar-glow-${filterId})`}
                />
                {/* Anchor nodes on vertices */}
                {axes.map((axis, i) => {
                  const val = s.values[axis.key] ?? 0;
                  const max = axis.max ?? 100;
                  const ratio = Math.max(0, Math.min(1.1, val / (max || 1)));
                  const angle = getAxisAngle(i);
                  const px = cx + radius * ratio * Math.cos(angle);
                  const py = cy + radius * ratio * Math.sin(angle);
                  const isHovered = activeAxisKey === axis.key;

                  return (
                    <circle
                      key={`pt-${s.id}-${axis.key}`}
                      cx={px}
                      cy={py}
                      r={isHovered ? 5 : 3.5}
                      fill={isHovered ? "#ffffff" : s.color}
                      stroke={s.color}
                      strokeWidth={2}
                      style={{ transition: "all 140ms ease" }}
                    />
                  );
                })}
              </g>
            );
          })}
        </svg>

        {/* Hover inspection HUD Card */}
        {activeAxis && (
          <div
            style={{
              ...nocCardDenseStyle,
              position: "absolute",
              bottom: "10px",
              left: "12px",
              pointerEvents: "none",
              background: "rgba(9, 14, 26, 0.94)",
              border: `1px solid ${NOC_COLORS.cyan}`,
              boxShadow: "0 4px 16px rgba(0, 0, 0, 0.6)",
              minWidth: "160px",
              zIndex: 10,
            }}
          >
            <div style={{ fontSize: NOC_TYPOGRAPHY.micro, color: NOC_COLORS.cyan, fontWeight: 700, textTransform: "uppercase" }}>
              {activeAxis.label}
            </div>
            <div style={{ display: "flex", flexDirection: "column", gap: "0.25rem", marginTop: "0.35rem" }}>
              {series
                .filter((s) => !hiddenSeriesIds.has(s.id))
                .map((s) => {
                  const val = s.values[activeAxis.key] ?? 0;
                  const max = activeAxis.max ?? 100;
                  const pct = Math.round((val / (max || 1)) * 100);
                  return (
                    <div key={s.id} style={{ display: "flex", justifyContent: "space-between", gap: "0.75rem", fontSize: "0.76rem" }}>
                      <span style={{ color: s.color, fontWeight: 600 }}>{s.name}</span>
                      <span className="tabular-nums" style={{ color: NOC_SURFACES.textPrimary, fontWeight: 700 }}>
                        {val}
                        {activeAxis.unit ? ` ${activeAxis.unit}` : ""} ({pct}%)
                      </span>
                    </div>
                  );
                })}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
