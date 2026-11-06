import { useState, useId, useMemo } from "react";
import { nocCardDenseStyle, nocPanelStyle } from "../panel";
import { NOC_COLORS, NOC_SURFACES, NOC_TYPOGRAPHY } from "../tokens";

export interface MatrixRow {
  id: string;
  label: string;
  values: number[]; // Length matches columns
}

export interface OrionDotMatrixHeatmapProps {
  title?: string;
  subtitle?: string;
  columnLabels: string[];
  rows: MatrixRow[];
  width?: number;
  height?: number;
  unit?: string;
  showEqualizer?: boolean;
}

export function OrionDotMatrixHeatmap({
  title = "Telemetry Dynamics Matrix",
  subtitle = "Activity density scatter & pulse equalizer distribution",
  columnLabels,
  rows,
  width = 560,
  height = 360,
  unit = "req/s",
  showEqualizer = true,
}: OrionDotMatrixHeatmapProps) {
  const [hoveredCell, setHoveredCell] = useState<{ rowIdx: number; colIdx: number } | null>(null);
  const filterId = useId();

  const padLeft = 72;
  const padRight = 24;
  const padTop = 32;
  const equalizerHeight = showEqualizer ? 64 : 0;
  const padBottom = 28 + equalizerHeight;

  const matrixW = width - padLeft - padRight;
  const matrixH = height - padTop - padBottom;

  const numCols = Math.max(1, columnLabels.length);
  const numRows = Math.max(1, rows.length);

  const colStep = matrixW / numCols;
  const rowStep = matrixH / numRows;

  // Find max value across all cells for scaling
  const maxValue = useMemo(() => {
    let max = 1;
    rows.forEach((r) => {
      r.values.forEach((v) => {
        if (v > max) max = v;
      });
    });
    return max;
  }, [rows]);

  // Aggregate sum per column for the equalizer bars
  const columnSums = useMemo(() => {
    const sums = new Array(numCols).fill(0);
    rows.forEach((r) => {
      r.values.forEach((v, cIdx) => {
        if (cIdx < numCols) {
          sums[cIdx] += v;
        }
      });
    });
    return sums;
  }, [rows, numCols]);

  const maxColSum = Math.max(...columnSums, 1);

  const getDotColor = (intensity: number) => {
    if (intensity >= 0.85) return NOC_COLORS.crit;
    if (intensity >= 0.6) return NOC_COLORS.purple;
    if (intensity >= 0.35) return NOC_COLORS.indigo;
    if (intensity >= 0.1) return NOC_COLORS.cyan;
    return "rgba(148, 163, 184, 0.25)";
  };

  const activeCellData = useMemo(() => {
    if (!hoveredCell) return null;
    const row = rows[hoveredCell.rowIdx];
    if (!row) return null;
    const val = row.values[hoveredCell.colIdx] ?? 0;
    const colLabel = columnLabels[hoveredCell.colIdx] || "";
    return {
      rowLabel: row.label,
      colLabel,
      value: val,
      pct: Math.round((val / maxValue) * 100),
    };
  }, [hoveredCell, rows, columnLabels, maxValue]);

  return (
    <div
      style={{
        ...nocPanelStyle,
        position: "relative",
        display: "flex",
        flexDirection: "column",
        gap: "0.75rem",
      }}
      data-testid="orion-dot-matrix-heatmap"
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
        <div style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
          <span className="tabular-nums" style={{ fontSize: "0.75rem", fontWeight: 700, color: NOC_COLORS.cyan }}>
            Max: {maxValue} {unit}
          </span>
        </div>
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
            <filter id={`dot-glow-${filterId}`} x="-40%" y="-40%" width="180%" height="180%">
              <feGaussianBlur stdDeviation="3.5" result="blur" />
              <feComposite in="SourceGraphic" in2="blur" operator="over" />
            </filter>
            <linearGradient id={`eq-bar-grad-${filterId}`} x1="0" y1="1" x2="0" y2="0">
              <stop offset="0%" stopColor="#38bdf8" stopOpacity="0.4" />
              <stop offset="60%" stopColor="#818cf8" stopOpacity="0.8" />
              <stop offset="100%" stopColor="#c084fc" stopOpacity="1" />
            </linearGradient>
          </defs>

          {/* Column Guideline Ticks & Labels */}
          {columnLabels.map((lbl, cIdx) => {
            const x = padLeft + cIdx * colStep + colStep / 2;
            return (
              <g key={`col-lbl-${cIdx}`}>
                <line
                  x1={x}
                  y1={padTop - 6}
                  x2={x}
                  y2={padTop + matrixH}
                  stroke="rgba(148, 163, 184, 0.08)"
                  strokeWidth={1}
                  strokeDasharray="2 3"
                />
                <text
                  x={x}
                  y={padTop - 12}
                  textAnchor="middle"
                  fill={NOC_SURFACES.textMuted}
                  fontSize={8.5}
                  fontWeight={500}
                >
                  {lbl}
                </text>
              </g>
            );
          })}

          {/* Row Labels & Guideline Ticks */}
          {rows.map((row, rIdx) => {
            const y = padTop + rIdx * rowStep + rowStep / 2;
            return (
              <g key={`row-lbl-${row.id}`}>
                <line
                  x1={padLeft - 6}
                  y1={y}
                  x2={padLeft + matrixW}
                  y2={y}
                  stroke="rgba(148, 163, 184, 0.08)"
                  strokeWidth={1}
                />
                <text
                  x={padLeft - 12}
                  y={y}
                  textAnchor="end"
                  dominantBaseline="central"
                  fill={NOC_SURFACES.textSecondary}
                  fontSize={9.5}
                  fontWeight={500}
                >
                  {row.label}
                </text>
              </g>
            );
          })}

          {/* Dot Matrix Cells */}
          {rows.map((row, rIdx) =>
            row.values.map((val, cIdx) => {
              if (cIdx >= numCols) return null;
              const cx = padLeft + cIdx * colStep + colStep / 2;
              const cy = padTop + rIdx * rowStep + rowStep / 2;
              const intensity = Math.max(0, Math.min(1, val / maxValue));
              const radius = 2.5 + intensity * 6.5;
              const isHovered = hoveredCell?.rowIdx === rIdx && hoveredCell?.colIdx === cIdx;
              const color = getDotColor(intensity);

              return (
                <g
                  key={`dot-${rIdx}-${cIdx}`}
                  tabIndex={0}
                  role="gridcell"
                  aria-label={`${row.label} at ${columnLabels[cIdx]}: ${val} ${unit}`}
                  onMouseEnter={() => setHoveredCell({ rowIdx: rIdx, colIdx: cIdx })}
                  onMouseLeave={() => setHoveredCell(null)}
                  onFocus={() => setHoveredCell({ rowIdx: rIdx, colIdx: cIdx })}
                  onBlur={() => setHoveredCell(null)}
                  style={{ cursor: "pointer", outline: "none" }}
                >
                  {/* Subtle hover ring */}
                  {isHovered && (
                    <circle
                      cx={cx}
                      cy={cy}
                      r={radius + 5}
                      fill="none"
                      stroke={NOC_COLORS.cyan}
                      strokeWidth={1.5}
                      filter={`url(#dot-glow-${filterId})`}
                    />
                  )}
                  {/* Dot */}
                  <circle
                    cx={cx}
                    cy={cy}
                    r={isHovered ? radius + 1.5 : radius}
                    fill={color}
                    fillOpacity={intensity > 0 ? (isHovered ? 1 : 0.85) : 0.3}
                    filter={intensity >= 0.6 || isHovered ? `url(#dot-glow-${filterId})` : undefined}
                    style={{ transition: "all 140ms ease" }}
                  />
                </g>
              );
            }),
          )}

          {/* Equalizer Audio / Activity Pulse Bars at Bottom */}
          {showEqualizer && (
            <g>
              <line
                x1={padLeft}
                y1={padTop + matrixH + 12}
                x2={padLeft + matrixW}
                y2={padTop + matrixH + 12}
                stroke="rgba(148, 163, 184, 0.2)"
                strokeWidth={1}
              />
              <text
                x={padLeft - 12}
                y={padTop + matrixH + 32}
                textAnchor="end"
                dominantBaseline="central"
                fill={NOC_COLORS.cyan}
                fontSize={8.5}
                fontWeight={600}
                style={{ textTransform: "uppercase" }}
              >
                Pulse
              </text>
              {columnSums.map((colSum, cIdx) => {
                const x = padLeft + cIdx * colStep + colStep / 2;
                const ratio = Math.max(0.06, Math.min(1, colSum / maxColSum));
                const barH = ratio * (equalizerHeight - 20);
                const barY = padTop + matrixH + equalizerHeight - barH + 6;
                const isHoveredCol = hoveredCell?.colIdx === cIdx;

                return (
                  <rect
                    key={`eq-bar-${cIdx}`}
                    x={x - 4}
                    y={barY}
                    width={8}
                    height={barH}
                    rx={2}
                    fill={isHoveredCol ? "#ffffff" : `url(#eq-bar-grad-${filterId})`}
                    opacity={isHoveredCol ? 1 : 0.85}
                    style={{ transition: "all 140ms ease" }}
                  />
                );
              })}
            </g>
          )}
        </svg>

        {/* Hover inspection HUD Card */}
        {activeCellData && (
          <div
            style={{
              ...nocCardDenseStyle,
              position: "absolute",
              top: "10px",
              right: "12px",
              background: "rgba(9, 14, 26, 0.94)",
              border: `1px solid ${getDotColor(activeCellData.value / maxValue)}`,
              boxShadow: "0 6px 20px rgba(0, 0, 0, 0.7)",
              minWidth: "150px",
              pointerEvents: "none",
              zIndex: 10,
            }}
          >
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <span style={{ fontSize: "0.8rem", fontWeight: 700, color: NOC_SURFACES.textPrimary }}>
                {activeCellData.rowLabel}
              </span>
              <span style={{ fontSize: "0.7rem", color: NOC_COLORS.cyan, fontWeight: 600 }}>
                {activeCellData.colLabel}
              </span>
            </div>
            <div style={{ marginTop: "0.25rem", display: "flex", justifyContent: "space-between", fontSize: "0.75rem" }}>
              <span style={{ color: NOC_SURFACES.textMuted }}>Throughput:</span>
              <span className="tabular-nums" style={{ color: NOC_SURFACES.textPrimary, fontWeight: 700 }}>
                {activeCellData.value} {unit} ({activeCellData.pct}%)
              </span>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
