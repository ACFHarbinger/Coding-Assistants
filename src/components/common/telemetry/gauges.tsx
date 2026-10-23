import type React from "react";
import { NOC_COLORS, NOC_SURFACES, NOC_TYPOGRAPHY, type NocTone, resolveNocToneColor } from "./tokens";

export type ArcRingSpec = {
  label: string;
  percent: number;
  color: string;
  detail: string;
  unknown?: boolean;
};

export type ProvenanceKind = "measured" | "estimated" | "unknown";

export function ProvenanceBadge({ provenance }: { provenance: ProvenanceKind }) {
  const styles: Record<ProvenanceKind, { bg: string; border: string; fg: string }> = {
    measured: {
      bg: "rgba(16, 185, 129, 0.14)",
      border: "rgba(16, 185, 129, 0.36)",
      fg: NOC_COLORS.ok,
    },
    estimated: {
      bg: "rgba(234, 179, 8, 0.14)",
      border: "rgba(234, 179, 8, 0.36)",
      fg: NOC_COLORS.warn,
    },
    unknown: {
      bg: "rgba(148, 163, 184, 0.14)",
      border: "rgba(148, 163, 184, 0.32)",
      fg: NOC_COLORS.idle,
    },
  };
  const s = styles[provenance] ?? styles.unknown;
  return (
    <span
      style={{
        display: "inline-flex",
        alignItems: "center",
        padding: "0.12rem 0.42rem",
        borderRadius: "4px",
        fontSize: "0.68rem",
        fontWeight: 700,
        textTransform: "uppercase",
        letterSpacing: "0.04em",
        background: s.bg,
        border: `1px solid ${s.border}`,
        color: s.fg,
      }}
    >
      {provenance}
    </span>
  );
}

/**
 * Shared horizontal meter bar with explicit semantic tone or threshold policy (CODEX-UI-5 / KIMI-UI-1).
 * When `provenance === "unknown"` or `unknown === true`, renders an idle track and "—" without a colored fill.
 */
export function Meter({
  label,
  percent,
  detail,
  color = "var(--primary)",
  tone,
  critThreshold = 90,
  warnThreshold,
  provenance,
  unknown = false,
}: {
  label: string;
  percent: number;
  detail: string;
  color?: string;
  tone?: NocTone;
  critThreshold?: number | null;
  warnThreshold?: number | null;
  provenance?: ProvenanceKind;
  unknown?: boolean;
}) {
  const isUnknown = unknown || provenance === "unknown" || !Number.isFinite(percent);
  const clamped = isUnknown ? 0 : Math.max(0, Math.min(100, percent));

  let fillColor = tone ? resolveNocToneColor(tone) : color;
  if (!isUnknown && !tone) {
    if (critThreshold !== null && clamped > critThreshold) {
      fillColor = NOC_COLORS.crit;
    } else if (warnThreshold !== undefined && warnThreshold !== null && clamped >= warnThreshold) {
      fillColor = NOC_COLORS.warn;
    }
  }

  return (
    <div>
      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          gap: "0.5rem",
          fontSize: "0.78rem",
          color: "var(--text-muted)",
        }}
      >
        <span style={{ color: NOC_SURFACES.textSecondary, fontWeight: 600, display: "inline-flex", alignItems: "center", gap: "0.4rem" }}>
          {label}
          {provenance && <ProvenanceBadge provenance={provenance} />}
        </span>
        <span className="tabular-nums" style={{ color: isUnknown ? NOC_COLORS.idle : "var(--text-muted)" }}>
          {isUnknown ? "—" : detail}
        </span>
      </div>
      <div
        style={{
          height: 7,
          background: NOC_SURFACES.gridline,
          borderRadius: 4,
          marginTop: "0.28rem",
          overflow: "hidden",
        }}
      >
        {!isUnknown && (
          <div
            style={{
              width: `${clamped}%`,
              height: "100%",
              borderRadius: 4,
              background: fillColor,
            }}
          />
        )}
      </div>
    </div>
  );
}

/**
 * Multi-ring concentric arc gauge (V2 / #373).
 * Rings marked `unknown: true` render as an idle track with no colored fill (KIMI-UI-1 / GROK-UI-3).
 */
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
  const strokeWidth = size < 120 ? 8 : 11;
  const ringGap = size < 120 ? 11 : 15;
  const outerRadius = center - strokeWidth;
  const arcSpanDeg = 265;
  const startAngleDeg = 137.5;

  return (
    <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: "0.55rem" }}>
      <div style={{ position: "relative", width: size, height: size, display: "grid", placeItems: "center" }}>
        {rings.map((ring) => {
          const isUnknown = Boolean(ring.unknown) || !Number.isFinite(ring.percent);
          const clamped = isUnknown ? 0 : Math.round(Math.max(0, Math.min(100, ring.percent)));
          return (
            <progress
              key={ring.label}
              className="ca-progress-ring"
              value={clamped}
              max={100}
              aria-label={`${title} ${ring.label}`}
              style={{
                "--ring-value": clamped,
                position: "absolute",
                width: 1,
                height: 1,
                opacity: 0,
                pointerEvents: "none",
              } as React.CSSProperties}
            />
          );
        })}
        <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`} role="img" aria-label={`${title} radial gauge`}>
          {rings.map((ring, idx) => {
            const radius = Math.max(16, outerRadius - idx * ringGap);
            const circumference = 2 * Math.PI * radius;
            const arcLength = (arcSpanDeg / 360) * circumference;
            const isUnknown = Boolean(ring.unknown) || !Number.isFinite(ring.percent);
            const clamped = isUnknown ? 0 : Math.max(0, Math.min(100, ring.percent));
            const filledLength = (clamped / 100) * arcLength;
            return (
              <g key={ring.label} transform={`rotate(${startAngleDeg} ${center} ${center})`}>
                <circle
                  cx={center}
                  cy={center}
                  r={radius}
                  fill="none"
                  stroke={NOC_SURFACES.track}
                  strokeWidth={strokeWidth}
                  strokeDasharray={`${arcLength} ${circumference}`}
                  strokeLinecap="round"
                />
                {!isUnknown && clamped > 0 && (
                  <circle
                    cx={center}
                    cy={center}
                    r={radius}
                    fill="none"
                    stroke={ring.color}
                    strokeWidth={strokeWidth}
                    strokeDasharray={`${filledLength} ${circumference}`}
                    strokeLinecap="round"
                  />
                )}
              </g>
            );
          })}
        </svg>
        <div
          className="tabular-nums"
          style={{
            position: "absolute",
            display: "flex",
            flexDirection: "column",
            alignItems: "center",
            justifyContent: "center",
            textAlign: "center",
            pointerEvents: "none",
            padding: "0 0.5rem",
          }}
        >
          <span style={{ fontSize: size < 120 ? "1rem" : "1.35rem", fontWeight: 800, color: NOC_SURFACES.textPrimary, lineHeight: 1.05 }}>
            {centerPrimary}
          </span>
          {centerSecondary && (
            <span style={{ fontSize: size < 120 ? "0.7rem" : "0.85rem", fontWeight: 700, color: NOC_COLORS.cyan, marginTop: "0.1rem" }}>
              {centerSecondary}
            </span>
          )}
          <span
            style={{
              fontSize: size < 120 ? "0.66rem" : "0.78rem",
              fontWeight: 700,
              color: NOC_SURFACES.textSecondary,
              marginTop: "0.2rem",
              textTransform: "uppercase",
              letterSpacing: "0.04em",
            }}
          >
            {title}
          </span>
        </div>
      </div>
      <div
        style={{
          display: "flex",
          flexWrap: "wrap",
          justifyContent: "center",
          gap: "0.65rem",
          fontSize: NOC_TYPOGRAPHY.micro,
          color: NOC_SURFACES.textMuted,
        }}
      >
        {rings.map((ring) => (
          <span key={ring.label} className="tabular-nums" style={{ display: "inline-flex", alignItems: "center", gap: "0.3rem" }}>
            <i
              style={{
                width: 8,
                height: 8,
                borderRadius: "50%",
                background: ring.unknown ? NOC_COLORS.idle : ring.color,
                display: "inline-block",
              }}
            />
            <strong style={{ color: NOC_SURFACES.textSecondary }}>{ring.label}:</strong> {ring.unknown ? "—" : ring.detail}
          </span>
        ))}
      </div>
    </div>
  );
}
