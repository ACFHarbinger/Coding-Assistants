import { useState, useId } from "react";
import { nocCardDenseStyle, nocPanelStyle } from "../panel";
import { NOC_COLORS, NOC_SURFACES, NOC_TYPOGRAPHY } from "../tokens";

export interface RadialRing {
  id: string;
  label: string;
  percent: number; // 0 to 100
  color: string;
  valueText?: string;
  detail?: string;
}

export interface OrionRadialRingMeterProps {
  title?: string;
  subtitle?: string;
  rings: RadialRing[];
  centerLabel?: string;
  centerValue?: string;
  centerSubtext?: string;
  size?: number;
  strokeWidth?: number;
  trackGap?: number;
  showLegend?: boolean;
}

export function OrionRadialRingMeter({
  title = "Telemetry Arc Composite",
  subtitle = "Multi-tier concentric utilization & bandwidth arcs",
  rings,
  centerLabel = "System Health",
  centerValue = "98.2%",
  centerSubtext = "Nominal",
  size = 320,
  strokeWidth = 10,
  trackGap = 6,
  showLegend = true,
}: OrionRadialRingMeterProps) {
  const [hoveredRingId, setHoveredRingId] = useState<string | null>(null);
  const filterId = useId();

  const cx = size / 2;
  const cy = size / 2;

  // Innermost radius calculation based on ring count
  const numRings = Math.max(1, rings.length);
  const baseOuterRadius = cx - 24;

  const ringGeometries = rings.map((ring, idx) => {
    const r = baseOuterRadius - idx * (strokeWidth + trackGap);
    const circumference = 2 * Math.PI * r;
    const clampedPct = Math.max(0, Math.min(100, ring.percent));
    const strokeDash = (clampedPct / 100) * circumference;
    return {
      ...ring,
      r,
      circumference,
      strokeDash,
    };
  });

  const activeRing = rings.find((r) => r.id === hoveredRingId);

  return (
    <div
      style={{
        ...nocPanelStyle,
        position: "relative",
        display: "flex",
        flexDirection: "column",
        gap: "0.75rem",
      }}
      data-testid="orion-radial-ring-meter"
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
      </div>

      <div style={{ display: "flex", alignItems: "center", justifyContent: "center", gap: "1.5rem", flexWrap: "wrap", padding: "0.5rem 0" }}>
        {/* SVG Concentric Arc Rings */}
        <div style={{ position: "relative", width: size, height: size, flexShrink: 0 }}>
          <svg
            viewBox={`0 0 ${size} ${size}`}
            width={size}
            height={size}
            role="img"
            aria-label={title}
            style={{ display: "block", overflow: "visible" }}
          >
            <defs>
              <filter id={`ring-glow-${filterId}`} x="-30%" y="-30%" width="160%" height="160%">
                <feGaussianBlur stdDeviation="3" result="blur" />
                <feComposite in="SourceGraphic" in2="blur" operator="over" />
              </filter>
              <radialGradient id={`ring-core-glow-${filterId}`} cx="50%" cy="50%" r="50%">
                <stop offset="0%" stopColor="#38bdf8" stopOpacity="0.18" />
                <stop offset="60%" stopColor="#6366f1" stopOpacity="0.08" />
                <stop offset="100%" stopColor="#090e1a" stopOpacity="0" />
              </radialGradient>
            </defs>

            {/* Ambient center background glow */}
            <circle cx={cx} cy={cy} r={baseOuterRadius - numRings * (strokeWidth + trackGap) + 12} fill={`url(#ring-core-glow-${filterId})`} />

            {/* Background circular tick marks */}
            {[0, 45, 90, 135, 180, 225, 270, 315].map((deg) => {
              const rad = (deg * Math.PI) / 180;
              const x1 = cx + (baseOuterRadius + 6) * Math.cos(rad);
              const y1 = cy + (baseOuterRadius + 6) * Math.sin(rad);
              const x2 = cx + (baseOuterRadius + 12) * Math.cos(rad);
              const y2 = cy + (baseOuterRadius + 12) * Math.sin(rad);
              return (
                <line
                  key={`tick-${deg}`}
                  x1={x1}
                  y1={y1}
                  x2={x2}
                  y2={y2}
                  stroke="rgba(148, 163, 184, 0.2)"
                  strokeWidth={1}
                />
              );
            })}

            {/* Render concentric rings */}
            {ringGeometries.map((g) => {
              const isHovered = hoveredRingId === g.id;
              const isDimmed = hoveredRingId !== null && !isHovered;

              return (
                <g
                  key={g.id}
                  opacity={isDimmed ? 0.25 : 1}
                  onMouseEnter={() => setHoveredRingId(g.id)}
                  onMouseLeave={() => setHoveredRingId(null)}
                  style={{ cursor: "pointer", transition: "opacity 160ms ease" }}
                >
                  {/* Background Track Circle */}
                  <circle
                    cx={cx}
                    cy={cy}
                    r={g.r}
                    fill="none"
                    stroke="rgba(148, 163, 184, 0.12)"
                    strokeWidth={strokeWidth}
                  />

                  {/* Active Foreground Progress Arc */}
                  <circle
                    cx={cx}
                    cy={cy}
                    r={g.r}
                    fill="none"
                    stroke={g.color}
                    strokeWidth={isHovered ? strokeWidth + 2 : strokeWidth}
                    strokeDasharray={`${g.strokeDash} ${g.circumference}`}
                    strokeLinecap="round"
                    transform={`rotate(-90 ${cx} ${cy})`}
                    filter={isHovered ? `url(#ring-glow-${filterId})` : undefined}
                    style={{ transition: "stroke-dasharray 240ms ease, stroke-width 140ms ease" }}
                  />
                </g>
              );
            })}
          </svg>

          {/* Center Hub Readout */}
          <div
            style={{
              position: "absolute",
              top: "50%",
              left: "50%",
              transform: "translate(-50%, -50%)",
              textAlign: "center",
              pointerEvents: "none",
              display: "flex",
              flexDirection: "column",
              alignItems: "center",
              justifyContent: "center",
            }}
          >
            <span style={{ fontSize: "0.68rem", color: NOC_SURFACES.textMuted, textTransform: "uppercase", letterSpacing: "0.06em", fontWeight: 600 }}>
              {activeRing ? activeRing.label : centerLabel}
            </span>
            <span
              className="tabular-nums"
              style={{
                fontSize: "1.45rem",
                fontWeight: 800,
                color: activeRing ? activeRing.color : NOC_SURFACES.textPrimary,
                letterSpacing: "-0.02em",
                margin: "0.1rem 0",
              }}
            >
              {activeRing ? `${activeRing.percent}%` : centerValue}
            </span>
            <span style={{ fontSize: "0.72rem", color: activeRing ? NOC_COLORS.cyan : NOC_COLORS.ok, fontWeight: 600 }}>
              {activeRing?.valueText || centerSubtext}
            </span>
          </div>
        </div>

        {/* Legend Panel */}
        {showLegend && (
          <div style={{ display: "flex", flexDirection: "column", gap: "0.55rem", flex: 1, minWidth: "180px" }}>
            {rings.map((ring) => {
              const isHovered = hoveredRingId === ring.id;
              return (
                <div
                  key={ring.id}
                  tabIndex={0}
                  role="button"
                  aria-label={`${ring.label}: ${ring.percent}%`}
                  onMouseEnter={() => setHoveredRingId(ring.id)}
                  onMouseLeave={() => setHoveredRingId(null)}
                  onFocus={() => setHoveredRingId(ring.id)}
                  onBlur={() => setHoveredRingId(null)}
                  style={{
                    ...nocCardDenseStyle,
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "space-between",
                    gap: "0.6rem",
                    cursor: "pointer",
                    border: isHovered ? `1px solid ${ring.color}` : `1px solid ${NOC_SURFACES.border}`,
                    background: isHovered ? "rgba(15, 23, 42, 0.9)" : "rgba(15, 23, 42, 0.6)",
                    transition: "all 140ms ease",
                    outline: "none",
                  }}
                >
                  <div style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
                    <span
                      style={{
                        width: 9,
                        height: 9,
                        borderRadius: "50%",
                        backgroundColor: ring.color,
                        boxShadow: `0 0 6px ${ring.color}`,
                        flexShrink: 0,
                      }}
                    />
                    <div>
                      <div style={{ fontSize: "0.78rem", fontWeight: 600, color: NOC_SURFACES.textPrimary }}>
                        {ring.label}
                      </div>
                      {ring.detail && (
                        <div style={{ fontSize: "0.68rem", color: NOC_SURFACES.textMuted }}>
                          {ring.detail}
                        </div>
                      )}
                    </div>
                  </div>
                  <div style={{ textAlign: "right" }}>
                    <span className="tabular-nums" style={{ fontSize: "0.85rem", fontWeight: 700, color: ring.color }}>
                      {ring.percent}%
                    </span>
                    {ring.valueText && (
                      <div className="tabular-nums" style={{ fontSize: "0.68rem", color: NOC_SURFACES.textSecondary }}>
                        {ring.valueText}
                      </div>
                    )}
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}
