import type React from "react";
import { ProvenanceBadge, type ProvenanceKind } from "./gauges";
import { nocPanelStyle } from "./panel";
import { NOC_COLORS, NOC_RADIUS, NOC_SURFACES, NOC_TYPOGRAPHY, type NocTone, resolveNocToneColor } from "./tokens";

export type MonitorTileItem = {
  id: string;
  title: string;
  subtitle: string;
  percent?: number;
  tone?: "ok" | "warn" | "crit" | "idle";
  available?: boolean;
};

export type KpiCardItem = {
  title: string;
  subtitle: string;
  value: string;
  unit?: string;
  footer: string;
  accent?: string;
  provenance?: ProvenanceKind;
  unknown?: boolean;
};

function resolveTileTone(tile: MonitorTileItem): { bg: string; border: string; text: string } {
  if (tile.available === false) {
    return { bg: "rgba(51, 65, 85, 0.65)", border: "rgba(100, 116, 139, 0.4)", text: "#cbd5e1" };
  }
  const explicitTone = tile.tone;
  if (explicitTone === "crit" || (explicitTone === undefined && (tile.percent ?? 0) >= 90)) {
    return { bg: "rgba(239, 68, 68, 0.88)", border: "#f87171", text: "#ffffff" };
  }
  if (explicitTone === "warn" || (explicitTone === undefined && (tile.percent ?? 0) >= 72)) {
    return { bg: "rgba(234, 179, 8, 0.88)", border: "#facc15", text: NOC_SURFACES.bgPanel };
  }
  if (explicitTone === "idle" || (explicitTone === undefined && !Number.isFinite(tile.percent))) {
    return { bg: "rgba(51, 65, 85, 0.65)", border: "rgba(100, 116, 139, 0.4)", text: "#cbd5e1" };
  }
  return { bg: "rgba(16, 185, 129, 0.88)", border: "#34d399", text: "#052e16" };
}

/**
 * Generic NOC traffic-light tile strip usable across System, Work Dashboard, and Channels.
 */
export function MonitorTileStrip({
  title = "Current Status of All Monitors",
  tiles,
  legendLabels = { ok: "Optimal (<72%)", warn: "Elevated (72–89%)", crit: "Critical (≥90%)" },
  testId = "monitor-status-strip",
}: {
  title?: string;
  tiles: MonitorTileItem[];
  legendLabels?: { ok: string; warn: string; crit: string };
  testId?: string;
}) {
  return (
    <div style={nocPanelStyle} data-testid={testId}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "0.65rem", gap: "0.75rem", flexWrap: "wrap" }}>
        <span style={{ fontSize: "0.82rem", fontWeight: 600, color: NOC_SURFACES.textSecondary, letterSpacing: "0.02em" }}>
          {title}
        </span>
        <div style={{ display: "flex", gap: "0.85rem", fontSize: NOC_TYPOGRAPHY.micro, color: "var(--text-muted)", flexWrap: "wrap" }}>
          <span style={{ display: "inline-flex", alignItems: "center", gap: "0.3rem" }}>
            <i style={{ width: 8, height: 8, borderRadius: 2, background: NOC_COLORS.ok, display: "inline-block" }} /> {legendLabels.ok}
          </span>
          <span style={{ display: "inline-flex", alignItems: "center", gap: "0.3rem" }}>
            <i style={{ width: 8, height: 8, borderRadius: 2, background: NOC_COLORS.warn, display: "inline-block" }} /> {legendLabels.warn}
          </span>
          <span style={{ display: "inline-flex", alignItems: "center", gap: "0.3rem" }}>
            <i style={{ width: 8, height: 8, borderRadius: 2, background: NOC_COLORS.crit, display: "inline-block" }} /> {legendLabels.crit}
          </span>
        </div>
      </div>
      <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(92px, 1fr))", gap: "0.4rem" }}>
        {tiles.map((tile) => {
          const tone = resolveTileTone(tile);
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

/**
 * Generic KPI callout card strip (KIMI-UI-1 / GROK-UI-3 / CODEX-UI-3).
 * Unknown items render as "—" in `--noc-idle` and never zero-fill.
 */
export function KpiCardGrid({
  cards,
  testId = "kpi-metric-strip",
}: {
  cards: KpiCardItem[];
  testId?: string;
}) {
  return (
    <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(170px, 1fr))", gap: "0.75rem" }} data-testid={testId}>
      {cards.map((card) => {
        const isUnknown = Boolean(card.unknown) || card.provenance === "unknown";
        const accent = isUnknown ? NOC_COLORS.idle : card.accent ?? NOC_COLORS.cyan;
        return (
          <div
            key={card.title}
            style={{
              ...nocPanelStyle,
              padding: "0.85rem 1rem",
              display: "flex",
              flexDirection: "column",
              justifyContent: "space-between",
              textAlign: "center",
              borderTop: `2px solid ${accent}`,
              minWidth: 0,
            }}
          >
            <div style={{ display: "flex", justifyContent: "center", alignItems: "center", gap: "0.35rem", fontSize: "0.76rem", fontWeight: 600, color: NOC_SURFACES.textSecondary, whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>
              <span>{card.title}</span>
              {card.provenance && <ProvenanceBadge provenance={card.provenance} />}
            </div>
            <div style={{ fontSize: "0.68rem", color: NOC_SURFACES.textMuted, marginTop: "0.2rem", whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>
              {card.subtitle}
            </div>
            <div className="tabular-nums" style={{ margin: "0.55rem 0 0.45rem", display: "flex", alignItems: "baseline", justifyContent: "center", gap: "0.2rem" }}>
              <span style={{ fontSize: "1.65rem", fontWeight: 800, color: isUnknown ? NOC_COLORS.idle : NOC_SURFACES.textPrimary, letterSpacing: "-0.02em", lineHeight: 1 }}>
                {isUnknown ? "—" : card.value}
              </span>
              {!isUnknown && card.unit && <span style={{ fontSize: "0.75rem", fontWeight: 600, color: accent }}>{card.unit}</span>}
            </div>
            <div style={{ fontSize: "0.68rem", color: NOC_SURFACES.textMuted, whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>
              {card.footer}
            </div>
          </div>
        );
      })}
    </div>
  );
}

/**
 * Staleness indicator dot + micro-label (`KIMI-UI-2`).
 * Shifts label color to `--noc-warn` when age exceeds `2 * pollIntervalSeconds`.
 */
export function StalenessDot({
  lastUpdatedUnixSeconds,
  pollIntervalSeconds = 3,
  nowUnixSeconds,
  labelPrefix = "live",
}: {
  lastUpdatedUnixSeconds?: number | null;
  pollIntervalSeconds?: number;
  nowUnixSeconds?: number;
  labelPrefix?: string;
}) {
  if (!lastUpdatedUnixSeconds || !Number.isFinite(lastUpdatedUnixSeconds) || lastUpdatedUnixSeconds <= 0) {
    return (
      <span
        className="tabular-nums"
        data-testid="staleness-dot"
        style={{
          display: "inline-flex",
          alignItems: "center",
          gap: "0.35rem",
          fontSize: NOC_TYPOGRAPHY.micro,
          color: NOC_COLORS.idle,
        }}
      >
        <i style={{ width: 7, height: 7, borderRadius: "50%", background: NOC_COLORS.idle, display: "inline-block" }} />
        <span>awaiting sample</span>
      </span>
    );
  }

  const now = nowUnixSeconds ?? Math.floor(Date.now() / 1000);
  const ageSec = Math.max(0, Math.round(now - lastUpdatedUnixSeconds));
  const isStale = ageSec > pollIntervalSeconds * 2;
  const dotColor = isStale ? NOC_COLORS.warn : NOC_COLORS.ok;

  return (
    <span
      className="tabular-nums"
      data-testid="staleness-dot"
      style={{
        display: "inline-flex",
        alignItems: "center",
        gap: "0.35rem",
        fontSize: NOC_TYPOGRAPHY.micro,
        color: isStale ? NOC_COLORS.warn : NOC_SURFACES.textMuted,
      }}
    >
      <i style={{ width: 7, height: 7, borderRadius: "50%", background: dotColor, display: "inline-block" }} />
      <span>
        {isStale ? "stale" : labelPrefix} • {ageSec}s
      </span>
    </span>
  );
}

/**
 * Standardized HUD status pill for cockpit headers and panel status bars (`GEMINI-UI-2` / `MISTRAL-UI-1`).
 */
export function HudStatusPill({
  label,
  value,
  title,
  tone = "cyan",
  onClick,
  maxWidth = 240,
  testId,
}: {
  label: string;
  value: string;
  title?: string;
  tone?: NocTone;
  onClick?: () => void;
  maxWidth?: number;
  testId?: string;
}) {
  const accent = resolveNocToneColor(tone);
  const bgMap: Record<NocTone, string> = {
    ok: "rgba(16, 185, 129, 0.1)",
    warn: "rgba(234, 179, 8, 0.12)",
    crit: "rgba(239, 68, 68, 0.12)",
    idle: "rgba(148, 163, 184, 0.1)",
    cyan: "rgba(56, 189, 248, 0.1)",
    indigo: "rgba(99, 102, 241, 0.12)",
    purple: "rgba(168, 85, 247, 0.12)",
    pink: "rgba(236, 72, 153, 0.12)",
    amber: "rgba(245, 158, 11, 0.12)",
  };
  const borderMap: Record<NocTone, string> = {
    ok: "rgba(16, 185, 129, 0.34)",
    warn: "rgba(234, 179, 8, 0.36)",
    crit: "rgba(239, 68, 68, 0.38)",
    idle: "rgba(148, 163, 184, 0.26)",
    cyan: "rgba(56, 189, 248, 0.34)",
    indigo: "rgba(99, 102, 241, 0.36)",
    purple: "rgba(168, 85, 247, 0.36)",
    pink: "rgba(236, 72, 153, 0.36)",
    amber: "rgba(245, 158, 11, 0.36)",
  };

  const commonStyle: React.CSSProperties = {
    maxWidth,
    minWidth: 0,
    padding: "0.38rem 0.68rem",
    borderRadius: NOC_RADIUS.s,
    border: `1px solid ${borderMap[tone]}`,
    background: bgMap[tone],
    color: NOC_SURFACES.textPrimary,
    fontSize: "0.74rem",
    lineHeight: 1.25,
    textAlign: "left",
    cursor: onClick ? "pointer" : "default",
  };

  const content = (
    <>
      <strong style={{ display: "block", color: accent, fontSize: "0.68rem", textTransform: "uppercase", letterSpacing: "0.04em" }}>
        {label}
      </strong>
      <span className="tabular-nums" style={{ display: "block", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", color: NOC_SURFACES.textSecondary }}>
        {value}
      </span>
    </>
  );

  if (onClick) {
    return (
      <button
        type="button"
        className="noc-focusable"
        title={title}
        onClick={onClick}
        data-testid={testId}
        style={commonStyle}
      >
        {content}
      </button>
    );
  }

  return (
    <div title={title} data-testid={testId} style={commonStyle}>
      {content}
    </div>
  );
}

/**
 * Styled empty-state or unavailable chip (`GROK-UI-3` / `CODEX-UI-4`).
 * Never renders a zeroed KPI strip.
 */
export function EmptyStateChip({
  title,
  detail,
  tone = "idle",
  actionLabel,
  onAction,
  testId = "empty-state-chip",
}: {
  title: string;
  detail?: string;
  tone?: "idle" | "warn" | "crit" | "ok";
  actionLabel?: string;
  onAction?: () => void;
  testId?: string;
}) {
  const dotColor = resolveNocToneColor(tone);
  return (
    <div
      data-testid={testId}
      style={{
        ...nocPanelStyle,
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        gap: "0.85rem",
        flexWrap: "wrap",
        padding: "0.85rem 1.05rem",
      }}
    >
      <div style={{ display: "flex", alignItems: "center", gap: "0.6rem", minWidth: 0 }}>
        <i style={{ width: 9, height: 9, borderRadius: "50%", background: dotColor, display: "inline-block", flexShrink: 0 }} />
        <div style={{ minWidth: 0 }}>
          <div style={{ fontSize: "0.84rem", fontWeight: 600, color: NOC_SURFACES.textSecondary }}>{title}</div>
          {detail && <div style={{ fontSize: "0.75rem", color: NOC_SURFACES.textMuted, marginTop: "0.12rem" }}>{detail}</div>}
        </div>
      </div>
      {actionLabel && onAction && (
        <button
          type="button"
          className="btn-secondary noc-focusable"
          style={{ marginTop: 0, padding: "0.35rem 0.75rem", fontSize: "0.78rem" }}
          onClick={onAction}
        >
          {actionLabel}
        </button>
      )}
    </div>
  );
}
