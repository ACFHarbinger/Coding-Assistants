import { useState, useId, useMemo } from "react";
import { nocCardDenseStyle, nocPanelStyle } from "../panel";
import { NOC_COLORS, NOC_SURFACES, NOC_TYPOGRAPHY } from "../tokens";

export interface OrbitalBubble {
  id: string;
  label: string;
  category: string;
  value: number;
  unit?: string;
  color: string;
  orbitIndex?: number; // 0 = inner, 1 = mid, 2 = outer
  angle?: number; // degrees 0-360
}

export interface OrionOrbitalBubbleChartProps {
  title?: string;
  subtitle?: string;
  bubbles: OrbitalBubble[];
  width?: number;
  height?: number;
  centerLabel?: string;
  centerValue?: string;
}

export function OrionOrbitalBubbleChart({
  title = "Orbital Volume Distribution",
  subtitle = "Spherical 3D cluster by resource volume & orbit tier",
  bubbles,
  width = 500,
  height = 400,
  centerLabel = "Total Volume",
  centerValue,
}: OrionOrbitalBubbleChartProps) {
  const [activeBubbleId, setActiveBubbleId] = useState<string | null>(null);
  const filterId = useId();

  const cx = width / 2;
  const cy = height / 2;
  const maxRadius = Math.min(cx, cy) - 40;

  const orbitRadii = [maxRadius * 0.38, maxRadius * 0.65, maxRadius * 0.92];

  const totalValue = useMemo(() => {
    return bubbles.reduce((acc, b) => acc + (b.value || 0), 0);
  }, [bubbles]);

  const valueRange = useMemo(() => {
    const vals = bubbles.map((b) => b.value);
    const min = Math.min(...vals, 1);
    const max = Math.max(...vals, 10);
    return { min, max };
  }, [bubbles]);

  // Compute 3D bubble positions along orbits
  const bubbleLayout = useMemo(() => {
    return bubbles.map((b, idx) => {
      const orbitIdx = b.orbitIndex !== undefined ? b.orbitIndex % orbitRadii.length : idx % orbitRadii.length;
      const rOrbit = orbitRadii[orbitIdx];
      // Distribute evenly along orbit if angle not explicitly set
      const angleDeg = b.angle !== undefined ? b.angle : (idx * (360 / Math.max(1, bubbles.length))) + (orbitIdx * 35);
      const angleRad = (angleDeg * Math.PI) / 180;

      // Scale bubble radius between 12px and 34px
      const norm = (b.value - valueRange.min) / (valueRange.max - valueRange.min || 1);
      const r = 12 + norm * 22;

      const bx = cx + rOrbit * Math.cos(angleRad);
      const by = cy + rOrbit * Math.sin(angleRad);

      return {
        ...b,
        x: bx,
        y: by,
        r,
        orbitRadius: rOrbit,
        angleRad,
        sharePct: totalValue > 0 ? Math.round((b.value / totalValue) * 100) : 0,
      };
    });
  }, [bubbles, orbitRadii, cx, cy, totalValue, valueRange]);

  const activeBubble = bubbleLayout.find((b) => b.id === activeBubbleId);

  return (
    <div
      style={{
        ...nocPanelStyle,
        position: "relative",
        display: "flex",
        flexDirection: "column",
        gap: "0.75rem",
      }}
      data-testid="orion-orbital-bubble-chart"
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
            {bubbles.length} Clusters &bull; {centerValue || totalValue.toLocaleString()}
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
            <filter id={`bubble-glow-${filterId}`} x="-40%" y="-40%" width="180%" height="180%">
              <feGaussianBlur stdDeviation="5" result="blur" />
              <feComposite in="SourceGraphic" in2="blur" operator="over" />
            </filter>

            {/* 3D Sphere Radial Gradients for each bubble */}
            {bubbles.map((b) => (
              <radialGradient
                key={`grad-${b.id}`}
                id={`sphere-grad-${filterId}-${b.id}`}
                cx="32%"
                cy="30%"
                r="70%"
                fx="26%"
                fy="24%"
              >
                <stop offset="0%" stopColor="#ffffff" stopOpacity="0.88" />
                <stop offset="22%" stopColor={b.color} stopOpacity="0.95" />
                <stop offset="68%" stopColor={b.color} stopOpacity="0.65" />
                <stop offset="100%" stopColor="#090e1a" stopOpacity="0.85" />
              </radialGradient>
            ))}

            {/* Center Core Hub Gradient */}
            <radialGradient id={`center-core-${filterId}`} cx="35%" cy="32%" r="65%">
              <stop offset="0%" stopColor="#38bdf8" stopOpacity="0.9" />
              <stop offset="50%" stopColor="#6366f1" stopOpacity="0.7" />
              <stop offset="100%" stopColor="#090e1a" stopOpacity="0.9" />
            </radialGradient>
          </defs>

          {/* Concentric orbital track guides */}
          {orbitRadii.map((rTrack, i) => (
            <circle
              key={`orbit-track-${i}`}
              cx={cx}
              cy={cy}
              r={rTrack}
              fill="none"
              stroke="rgba(148, 163, 184, 0.12)"
              strokeWidth={1}
              strokeDasharray={i % 2 === 1 ? "4 4" : undefined}
            />
          ))}

          {/* Center Hub Core */}
          <circle
            cx={cx}
            cy={cy}
            r={24}
            fill={`url(#center-core-${filterId})`}
            stroke="rgba(56, 189, 248, 0.5)"
            strokeWidth={1.5}
            filter={`url(#bubble-glow-${filterId})`}
          />
          <text
            x={cx}
            y={cy - 4}
            textAnchor="middle"
            fill={NOC_SURFACES.textPrimary}
            fontSize={7}
            fontWeight={700}
            style={{ textTransform: "uppercase" }}
          >
            {centerLabel.slice(0, 5)}
          </text>
          <text
            x={cx}
            y={cy + 8}
            textAnchor="middle"
            fill={NOC_COLORS.cyan}
            fontSize={7.5}
            fontWeight={600}
            className="tabular-nums"
          >
            {bubbles.length}
          </text>

          {/* Hover ray to center when bubble active */}
          {activeBubble && (
            <line
              x1={cx}
              y1={cy}
              x2={activeBubble.x}
              y2={activeBubble.y}
              stroke={activeBubble.color}
              strokeWidth={1.5}
              strokeDasharray="3 3"
              opacity={0.8}
            />
          )}

          {/* 3D Sphere Bubbles */}
          {bubbleLayout.map((b) => {
            const isHovered = activeBubbleId === b.id;
            return (
              <g
                key={b.id}
                tabIndex={0}
                role="button"
                aria-label={`${b.label}: ${b.value} ${b.unit || ""}`}
                onMouseEnter={() => setActiveBubbleId(b.id)}
                onMouseLeave={() => setActiveBubbleId(null)}
                onFocus={() => setActiveBubbleId(b.id)}
                onBlur={() => setActiveBubbleId(null)}
                style={{ cursor: "pointer", outline: "none" }}
              >
                {/* Outer luminous glow aura on hover */}
                {isHovered && (
                  <circle
                    cx={b.x}
                    cy={b.y}
                    r={b.r + 8}
                    fill={b.color}
                    fillOpacity={0.28}
                    filter={`url(#bubble-glow-${filterId})`}
                  />
                )}

                {/* 3D Shaded sphere body */}
                <circle
                  cx={b.x}
                  cy={b.y}
                  r={b.r}
                  fill={`url(#sphere-grad-${filterId}-${b.id})`}
                  stroke={isHovered ? "#ffffff" : b.color}
                  strokeWidth={isHovered ? 2 : 1}
                  style={{ transition: "all 140ms ease" }}
                />

                {/* Center text on larger bubbles */}
                {b.r >= 16 && (
                  <text
                    x={b.x}
                    y={b.y}
                    textAnchor="middle"
                    dominantBaseline="central"
                    fill="#ffffff"
                    fontSize={b.r >= 22 ? 10 : 8}
                    fontWeight={700}
                    className="tabular-nums"
                    pointerEvents="none"
                    style={{ textShadow: "0 1px 3px rgba(0,0,0,0.8)" }}
                  >
                    {b.sharePct}%
                  </text>
                )}

                {/* Label text beneath bubble */}
                <text
                  x={b.x}
                  y={b.y + b.r + 11}
                  textAnchor="middle"
                  fill={isHovered ? "#ffffff" : NOC_SURFACES.textSecondary}
                  fontSize={8.5}
                  fontWeight={isHovered ? 700 : 500}
                  pointerEvents="none"
                >
                  {b.label}
                </text>
              </g>
            );
          })}
        </svg>

        {/* Hover inspection HUD Card */}
        {activeBubble && (
          <div
            style={{
              ...nocCardDenseStyle,
              position: "absolute",
              bottom: "10px",
              left: "12px",
              background: "rgba(9, 14, 26, 0.94)",
              border: `1px solid ${activeBubble.color}`,
              boxShadow: "0 6px 20px rgba(0, 0, 0, 0.7)",
              minWidth: "160px",
              pointerEvents: "none",
              zIndex: 10,
            }}
          >
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", gap: "0.5rem" }}>
              <span style={{ fontSize: "0.8rem", fontWeight: 700, color: NOC_SURFACES.textPrimary }}>
                {activeBubble.label}
              </span>
              <span style={{ fontSize: "0.68rem", color: activeBubble.color, fontWeight: 700 }}>
                {activeBubble.category}
              </span>
            </div>
            <div style={{ marginTop: "0.25rem", display: "flex", justifyContent: "space-between", gap: "0.5rem", fontSize: "0.74rem" }}>
              <span style={{ color: NOC_SURFACES.textMuted }}>Volume:</span>
              <span className="tabular-nums" style={{ color: NOC_SURFACES.textPrimary, fontWeight: 700 }}>
                {activeBubble.value.toLocaleString()} {activeBubble.unit || ""}
              </span>
            </div>
            <div style={{ display: "flex", justifyContent: "space-between", gap: "0.5rem", fontSize: "0.74rem" }}>
              <span style={{ color: NOC_SURFACES.textMuted }}>Share:</span>
              <span className="tabular-nums" style={{ color: NOC_COLORS.cyan, fontWeight: 700 }}>
                {activeBubble.sharePct}% of total
              </span>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
