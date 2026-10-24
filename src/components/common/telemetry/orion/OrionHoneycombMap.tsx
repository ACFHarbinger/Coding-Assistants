import { useState, useId, useMemo } from "react";
import { nocCardDenseStyle, nocPanelStyle } from "../panel";
import { NOC_COLORS, NOC_SURFACES, NOC_TYPOGRAPHY } from "../tokens";

export interface HexCellData {
  col: number;
  row: number;
  intensity: number; // 0 (idle) to 100 (peak)
  label?: string;
  metric?: string;
  status?: "idle" | "ok" | "active" | "warn" | "crit";
  service?: string;
}

export interface HexTrajectory {
  id: string;
  from: [number, number]; // [col, row]
  to: [number, number];
  color?: string;
  label?: string;
}

export interface OrionHoneycombMapProps {
  title?: string;
  subtitle?: string;
  columns?: number;
  rows?: number;
  cells?: HexCellData[];
  trajectories?: HexTrajectory[];
  width?: number;
  height?: number;
  hexRadius?: number;
}

export function OrionHoneycombMap({
  title = "Honeycomb Situational Matrix",
  subtitle = "Hexagonal topology & active trajectory flows",
  columns = 12,
  rows = 7,
  cells = [],
  trajectories = [],
  width = 560,
  height = 360,
  hexRadius = 18,
}: OrionHoneycombMapProps) {
  const [hoveredCoord, setHoveredCoord] = useState<string | null>(null);
  const [filterLevel, setFilterLevel] = useState<"all" | "active" | "alert">("all");
  const filterId = useId();

  // Create lookup map for cells by "col,row"
  const cellMap = useMemo(() => {
    const map = new Map<string, HexCellData>();
    cells.forEach((c) => {
      map.set(`${c.col},${c.row}`, c);
    });
    return map;
  }, [cells]);

  // Hexagon geometry: Flat-topped hexagon
  // col spacing: 1.5 * r
  // row spacing: sqrt(3) * r
  const r = hexRadius;
  const sqrt3 = Math.sqrt(3);
  const colSpacing = 1.5 * r;
  const rowSpacing = sqrt3 * r;

  const getHexCenter = (col: number, row: number) => {
    const x = 32 + col * colSpacing;
    const y = 32 + row * rowSpacing + (col % 2 === 1 ? rowSpacing / 2 : 0);
    return { x, y };
  };

  const getHexPoints = (cx: number, cy: number, radius: number) => {
    const pts: string[] = [];
    for (let i = 0; i < 6; i++) {
      const angle = (i * Math.PI) / 3;
      const px = cx + radius * Math.cos(angle);
      const py = cy + radius * Math.sin(angle);
      pts.push(`${px.toFixed(1)},${py.toFixed(1)}`);
    }
    return pts.join(" ");
  };

  const getCellFill = (intensity: number, status?: string) => {
    if (status === "crit" || intensity >= 85) return NOC_COLORS.crit;
    if (status === "warn" || intensity >= 65) return NOC_COLORS.warn;
    if (status === "active" || intensity >= 40) return NOC_COLORS.purple;
    if (status === "ok" || intensity >= 15) return NOC_COLORS.cyan;
    return "rgba(148, 163, 184, 0.08)";
  };

  const activeHoveredCell = hoveredCoord ? cellMap.get(hoveredCoord) : null;

  return (
    <div
      style={{
        ...nocPanelStyle,
        position: "relative",
        display: "flex",
        flexDirection: "column",
        gap: "0.75rem",
      }}
      data-testid="orion-honeycomb-map"
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
        <div style={{ display: "flex", alignItems: "center", gap: "0.35rem" }}>
          {(["all", "active", "alert"] as const).map((lvl) => (
            <button
              key={lvl}
              type="button"
              onClick={() => setFilterLevel(lvl)}
              aria-pressed={filterLevel === lvl}
              style={{
                background: filterLevel === lvl ? "rgba(56, 189, 248, 0.16)" : "transparent",
                border: `1px solid ${filterLevel === lvl ? NOC_COLORS.cyan : NOC_SURFACES.border}`,
                borderRadius: "4px",
                padding: "0.15rem 0.5rem",
                color: filterLevel === lvl ? NOC_COLORS.cyan : NOC_SURFACES.textMuted,
                fontSize: "0.7rem",
                fontWeight: 600,
                cursor: "pointer",
                textTransform: "capitalize",
              }}
            >
              {lvl}
            </button>
          ))}
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
            <filter id={`hex-glow-${filterId}`} x="-30%" y="-30%" width="160%" height="160%">
              <feGaussianBlur stdDeviation="3.5" result="blur" />
              <feComposite in="SourceGraphic" in2="blur" operator="over" />
            </filter>
          </defs>

          {/* Render Hexagon Grid */}
          {Array.from({ length: columns }).map((_, cIdx) =>
            Array.from({ length: rows }).map((_, rIdx) => {
              const key = `${cIdx},${rIdx}`;
              const data = cellMap.get(key);
              const intensity = data?.intensity ?? 0;
              const { x, y } = getHexCenter(cIdx, rIdx);

              // Filter logic
              if (filterLevel === "active" && intensity < 30) return null;
              if (filterLevel === "alert" && intensity < 65) return null;

              const isHovered = hoveredCoord === key;
              const fill = getCellFill(intensity, data?.status);
              const stroke = isHovered
                ? "#ffffff"
                : intensity > 10
                  ? fill
                  : "rgba(148, 163, 184, 0.14)";

              return (
                <g
                  key={key}
                  tabIndex={0}
                  role="gridcell"
                  aria-label={`Cell [${cIdx},${rIdx}] ${intensity}%`}
                  onMouseEnter={() => setHoveredCoord(key)}
                  onMouseLeave={() => setHoveredCoord(null)}
                  onFocus={() => setHoveredCoord(key)}
                  onBlur={() => setHoveredCoord(null)}
                  style={{ cursor: "pointer", outline: "none" }}
                >
                  <polygon
                    points={getHexPoints(x, y, r - 1.5)}
                    fill={fill}
                    fillOpacity={intensity > 0 ? (isHovered ? 0.95 : 0.72) : 0.25}
                    stroke={stroke}
                    strokeWidth={isHovered ? 2 : 1}
                    filter={intensity >= 65 || isHovered ? `url(#hex-glow-${filterId})` : undefined}
                  />
                  {data?.label && (
                    <text
                      x={x}
                      y={y}
                      textAnchor="middle"
                      dominantBaseline="central"
                      fill="#ffffff"
                      fontSize={8}
                      fontWeight={700}
                      pointerEvents="none"
                      style={{ textShadow: "0 1px 2px rgba(0,0,0,0.8)" }}
                    >
                      {data.label}
                    </text>
                  )}
                </g>
              );
            }),
          )}

          {/* Render Trajectory Beziers */}
          {trajectories.map((traj) => {
            const p1 = getHexCenter(traj.from[0], traj.from[1]);
            const p2 = getHexCenter(traj.to[0], traj.to[1]);
            // Compute curved control point
            const midX = (p1.x + p2.x) / 2;
            const midY = (p1.y + p2.y) / 2 - 32;
            const pathD = `M ${p1.x.toFixed(1)} ${p1.y.toFixed(1)} Q ${midX.toFixed(1)} ${midY.toFixed(1)} ${p2.x.toFixed(1)} ${p2.y.toFixed(1)}`;
            const color = traj.color || NOC_COLORS.cyan;

            return (
              <g key={traj.id} pointerEvents="none">
                <path
                  d={pathD}
                  fill="none"
                  stroke={color}
                  strokeWidth={2}
                  strokeDasharray="5 5"
                  strokeLinecap="round"
                  filter={`url(#hex-glow-${filterId})`}
                />
                {/* Anchor glowing dots at start and end */}
                <circle cx={p1.x} cy={p1.y} r={3} fill="#ffffff" stroke={color} strokeWidth={1.5} />
                <circle cx={p2.x} cy={p2.y} r={3} fill="#ffffff" stroke={color} strokeWidth={1.5} />
              </g>
            );
          })}
        </svg>

        {/* Hover inspection HUD Card */}
        {activeHoveredCell && (
          <div
            style={{
              ...nocCardDenseStyle,
              position: "absolute",
              bottom: "10px",
              left: "12px",
              background: "rgba(9, 14, 26, 0.95)",
              border: `1px solid ${getCellFill(activeHoveredCell.intensity, activeHoveredCell.status)}`,
              boxShadow: "0 6px 20px rgba(0, 0, 0, 0.7)",
              minWidth: "150px",
              pointerEvents: "none",
              zIndex: 10,
            }}
          >
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <span style={{ fontSize: "0.78rem", fontWeight: 700, color: NOC_SURFACES.textPrimary }}>
                {activeHoveredCell.service || `Hex [${activeHoveredCell.col},${activeHoveredCell.row}]`}
              </span>
              <span
                className="tabular-nums"
                style={{
                  fontSize: "0.72rem",
                  fontWeight: 700,
                  color: getCellFill(activeHoveredCell.intensity, activeHoveredCell.status),
                }}
              >
                {activeHoveredCell.intensity}%
              </span>
            </div>
            {activeHoveredCell.metric && (
              <div style={{ fontSize: "0.72rem", color: NOC_SURFACES.textSecondary, marginTop: "0.2rem" }}>
                Load: {activeHoveredCell.metric}
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
