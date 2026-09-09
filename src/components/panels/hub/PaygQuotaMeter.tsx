import { useEffect, useState } from "react";
import type { ProviderQuota, ProviderQuotaBalance } from "./types";

export interface BalanceSnapshot {
  timestamp: number;
  total: number;
  paid?: number | null;
  gift?: number | null;
}

export const STORAGE_KEY_PREFIX = "ca.quota_history:";

export function loadHistory(
  agentId: string,
  current?: ProviderQuotaBalance | null,
  fetchedAt?: number
): BalanceSnapshot[] {
  const key = `${STORAGE_KEY_PREFIX}${agentId}`;
  let history: BalanceSnapshot[] = [];
  try {
    const raw = localStorage.getItem(key);
    if (raw) {
      const parsed = JSON.parse(raw);
      if (Array.isArray(parsed)) history = parsed;
    }
  } catch {
    history = [];
  }

  if (current && current.kind !== "spend" && fetchedAt) {
    const exists = history.some((item) => Math.abs(item.timestamp - fetchedAt) < 60);
    if (!exists) {
      history = [
        ...history,
        {
          timestamp: fetchedAt,
          total: current.total,
          paid: current.paid ?? current.topped_up ?? null,
          gift: current.gift ?? current.granted ?? null,
        },
      ].slice(-30); // retain last 30 snapshots
      try {
        localStorage.setItem(key, JSON.stringify(history));
      } catch {
        // ignore local storage errors
      }
    }
  }
  return history;
}

function formatTime(timestamp: number): string {
  try {
    const date = new Date(timestamp * 1000);
    return date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  } catch {
    return "";
  }
}

function formatFullDate(timestamp: number): string {
  try {
    const date = new Date(timestamp * 1000);
    return date.toLocaleString();
  } catch {
    return "";
  }
}

export function PaygQuotaMeter({ quota }: { quota: ProviderQuota }) {
  const [history, setHistory] = useState<BalanceSnapshot[]>([]);

  const info = quota.balance_info;
  const isSpend = info?.kind === "spend" || Boolean(quota.balance?.toLowerCase().startsWith("spent"));
  const currency = info?.currency || "USD";
  const total = info?.total ?? (quota.balance ? parseFloat(quota.balance.replace(/[^0-9.]/g, "")) || 0 : 0);
  const paid = isSpend ? null : (info?.paid ?? info?.topped_up ?? null);
  const gift = isSpend ? null : (info?.gift ?? info?.granted ?? null);

  const hasBreakdown = paid !== null && gift !== null && total > 0;
  const paidPercent = hasBreakdown ? Math.max(0, Math.min(100, (paid / total) * 100)) : 100;
  const giftPercent = hasBreakdown ? Math.max(0, Math.min(100, (gift / total) * 100)) : 0;

  useEffect(() => {
    setHistory(loadHistory(quota.agent_id, quota.balance_info, quota.fetched_at));
  }, [quota.agent_id, quota.balance_info, quota.fetched_at]);

  // Chart dimensions & layout
  const chartWidth = 720;
  const chartHeight = 150;
  const paddingLeft = 55;
  const paddingRight = 25;
  const paddingTop = 20;
  const paddingBottom = 25;
  const plotWidth = chartWidth - paddingLeft - paddingRight;
  const plotHeight = chartHeight - paddingTop - paddingBottom;

  const values = history.map((h) => h.total);
  const rawMin = values.length > 0 ? Math.min(...values) : 0;
  const rawMax = values.length > 0 ? Math.max(...values) : 0;
  const minVal = rawMin === rawMax ? Math.max(0, rawMin - 1) : rawMin;
  const maxVal = rawMin === rawMax ? rawMax + 1 : rawMax;
  const range = maxVal - minVal || 1;

  const points = history.map((snap, i) => {
    const x = paddingLeft + (history.length > 1 ? (i / (history.length - 1)) * plotWidth : plotWidth / 2);
    const y = chartHeight - paddingBottom - ((snap.total - minVal) / range) * plotHeight;
    return { x, y, snap };
  });

  const polylinePoints = points.map((p) => `${p.x.toFixed(1)},${p.y.toFixed(1)}`).join(" ");
  const areaPath =
    points.length >= 2
      ? `M ${points[0].x.toFixed(1)},${chartHeight - paddingBottom} L ${polylinePoints} L ${points[points.length - 1].x.toFixed(1)},${chartHeight - paddingBottom} Z`
      : "";

  const oldest = history.length > 0 ? history[0] : null;
  const latest = history.length > 0 ? history[history.length - 1] : null;
  const delta = oldest && latest ? latest.total - oldest.total : 0;

  return (
    <div style={{ display: "grid", gap: "1rem", marginTop: "0.4rem" }}>
      {/* 1. Stat cards: Available balance / Period Spend, Paid, Gift */}
      <div
        style={{
          display: "grid",
          gridTemplateColumns: "repeat(auto-fit, minmax(170px, 1fr))",
          gap: "0.75rem",
        }}
      >
        <div
          style={{
            background: isSpend ? "rgba(56, 189, 248, 0.08)" : "rgba(34, 197, 94, 0.08)",
            border: `1px solid ${isSpend ? "rgba(56, 189, 248, 0.25)" : "rgba(34, 197, 94, 0.25)"}`,
            borderRadius: "10px",
            padding: "0.85rem 1rem",
            display: "grid",
            gap: "0.25rem",
          }}
        >
          <span style={{ fontSize: "0.75rem", color: "var(--text-muted)", textTransform: "uppercase", letterSpacing: "0.05em", fontWeight: 600 }}>
            {isSpend ? "Period Spend" : "Available Balance"}
          </span>
          <div style={{ display: "flex", alignItems: "baseline", gap: "0.4rem" }}>
            <span style={{ fontSize: "1.45rem", fontWeight: 700, color: isSpend ? "#38bdf8" : "#22c55e" }}>
              ${total.toFixed(2)}
            </span>
            <span style={{ fontSize: "0.78rem", color: "var(--text-muted)", fontWeight: 600 }}>
              {currency}
            </span>
          </div>
          <span style={{ fontSize: "0.72rem", color: "var(--text-muted)" }}>
            {isSpend ? "Total period spend" : "Current API balance"}
          </span>
        </div>

        {paid !== null && (
          <div
            style={{
              background: "rgba(56, 189, 248, 0.08)",
              border: "1px solid rgba(56, 189, 248, 0.25)",
              borderRadius: "10px",
              padding: "0.85rem 1rem",
              display: "grid",
              gap: "0.25rem",
            }}
          >
            <span style={{ fontSize: "0.75rem", color: "var(--text-muted)", textTransform: "uppercase", letterSpacing: "0.05em", fontWeight: 600 }}>
              Paid / Topped-up
            </span>
            <div style={{ display: "flex", alignItems: "baseline", gap: "0.4rem" }}>
              <span style={{ fontSize: "1.45rem", fontWeight: 700, color: "#38bdf8" }}>
                ${paid.toFixed(2)}
              </span>
              <span style={{ fontSize: "0.78rem", color: "var(--text-muted)", fontWeight: 600 }}>
                {currency}
              </span>
            </div>
            <span style={{ fontSize: "0.72rem", color: "var(--text-muted)" }}>
              {hasBreakdown ? `${paidPercent.toFixed(1)}% of total funds` : "Purchased credits"}
            </span>
          </div>
        )}

        {gift !== null && (
          <div
            style={{
              background: "rgba(52, 211, 153, 0.08)",
              border: "1px solid rgba(52, 211, 153, 0.25)",
              borderRadius: "10px",
              padding: "0.85rem 1rem",
              display: "grid",
              gap: "0.25rem",
            }}
          >
            <span style={{ fontSize: "0.75rem", color: "var(--text-muted)", textTransform: "uppercase", letterSpacing: "0.05em", fontWeight: 600 }}>
              Free / Gift Grant
            </span>
            <div style={{ display: "flex", alignItems: "baseline", gap: "0.4rem" }}>
              <span style={{ fontSize: "1.45rem", fontWeight: 700, color: "#34d399" }}>
                ${gift.toFixed(2)}
              </span>
              <span style={{ fontSize: "0.78rem", color: "var(--text-muted)", fontWeight: 600 }}>
                {currency}
              </span>
            </div>
            <span style={{ fontSize: "0.72rem", color: "var(--text-muted)" }}>
              {hasBreakdown ? `${giftPercent.toFixed(1)}% of total funds` : "Promotional credits"}
            </span>
          </div>
        )}
      </div>

      {/* 2. Segmented balance bar */}
      {!isSpend && (
        <div style={{ display: "grid", gap: "0.4rem", padding: "0.4rem 0" }}>
        <div style={{ display: "flex", justifyContent: "space-between", fontSize: "0.8rem", color: "var(--text-muted)" }}>
          <span style={{ fontWeight: 600 }}>Credit Composition</span>
          <span>{hasBreakdown ? `Paid: $${paid.toFixed(2)} | Gift: $${gift.toFixed(2)}` : `$${total.toFixed(2)} ${currency}`}</span>
        </div>
        <div
          style={{
            height: "14px",
            background: "#1e293b",
            borderRadius: "7px",
            overflow: "hidden",
            display: "flex",
            boxShadow: "inset 0 1px 3px rgba(0,0,0,0.4)",
          }}
          title={hasBreakdown ? `Paid: $${paid.toFixed(2)} (${paidPercent.toFixed(1)}%) | Gift: $${gift.toFixed(2)} (${giftPercent.toFixed(1)}%)` : `$${total.toFixed(2)} ${currency}`}
        >
          {hasBreakdown ? (
            <>
              <div
                style={{
                  width: `${paidPercent}%`,
                  height: "100%",
                  background: "linear-gradient(90deg, #0284c7, #38bdf8)",
                  transition: "width 0.4s ease",
                }}
              />
              <div
                style={{
                  width: `${giftPercent}%`,
                  height: "100%",
                  background: "linear-gradient(90deg, #059669, #34d399)",
                  transition: "width 0.4s ease",
                }}
              />
            </>
          ) : (
            <div
              style={{
                width: "100%",
                height: "100%",
                background: "var(--primary)",
              }}
            />
          )}
        </div>
        <div style={{ display: "flex", gap: "1.25rem", fontSize: "0.75rem", color: "var(--text-muted)", marginTop: "0.1rem" }}>
          <span style={{ display: "inline-flex", alignItems: "center", gap: "0.35rem" }}>
            <i style={{ width: 8, height: 8, borderRadius: "50%", background: "#38bdf8", display: "inline-block" }} />
            Paid Refill ({hasBreakdown ? `${paidPercent.toFixed(1)}%` : "100%"})
          </span>
          <span style={{ display: "inline-flex", alignItems: "center", gap: "0.35rem" }}>
            <i style={{ width: 8, height: 8, borderRadius: "50%", background: "#34d399", display: "inline-block" }} />
            Promotional Grant ({hasBreakdown ? `${giftPercent.toFixed(1)}%` : "0%"})
          </span>
        </div>
      </div>
      )}

      {/* 3. Truthful Local Balance History */}
      {!isSpend && (
      <div
        style={{
          border: "1px solid var(--border-color)",
          borderRadius: "10px",
          background: "rgba(0,0,0,0.25)",
          padding: "1rem",
          display: "grid",
          gap: "0.85rem",
        }}
        data-testid="payg-balance-history"
      >
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", flexWrap: "wrap", gap: "0.5rem" }}>
          <div>
            <div style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
              <span style={{ fontWeight: 600, fontSize: "0.92rem", color: "var(--text-main)" }}>
                Balance History
              </span>
              <span
                style={{
                  fontSize: "0.68rem",
                  padding: "0.15rem 0.45rem",
                  borderRadius: "10px",
                  background: "rgba(34, 197, 94, 0.12)",
                  color: "#22c55e",
                  border: "1px solid rgba(34, 197, 94, 0.25)",
                }}
              >
                Local Snapshots
              </span>
            </div>
            <span style={{ fontSize: "0.74rem", color: "var(--text-muted)" }}>
              Derived solely from captured balance snapshots over time
            </span>
          </div>

          {history.length >= 2 && latest && oldest && (
            <div style={{ display: "flex", alignItems: "center", gap: "0.75rem", fontSize: "0.76rem" }}>
              <span style={{ color: "var(--text-muted)" }}>
                Change:{" "}
                <strong style={{ color: delta >= 0 ? "#22c55e" : "#ef4444" }}>
                  {delta >= 0 ? "+" : ""}${delta.toFixed(2)} {currency}
                </strong>
              </span>
              <span style={{ color: "var(--text-muted)" }}>
                ({history.length} snapshots)
              </span>
            </div>
          )}
        </div>

        {/* Truthful empty or insufficient history state (< 2 snapshots) */}
        {history.length < 2 ? (
          <div
            style={{
              border: "1px dashed var(--border-color)",
              borderRadius: "8px",
              background: "rgba(0,0,0,0.15)",
              padding: "1.25rem 1rem",
              textAlign: "center",
              display: "grid",
              gap: "0.35rem",
            }}
            data-testid="insufficient-history"
          >
            <span style={{ fontWeight: 600, fontSize: "0.88rem", color: "var(--text-main)" }}>
              Insufficient Snapshot History
            </span>
            <span style={{ fontSize: "0.8rem", color: "var(--text-muted)" }}>
              {history.length === 1
                ? "1 balance snapshot recorded. At least two snapshots are needed to plot balance changes over time."
                : "No balance snapshots recorded yet. Snapshots accumulate locally across quota refreshes."}
            </span>
            <span style={{ fontSize: "0.74rem", color: "var(--text-muted)", opacity: 0.8 }}>
              Click Refresh or wait for the auto-refresh cadence to establish a trend.
            </span>
          </div>
        ) : (
          /* SVG Balance Trend Chart */
          <div>
            <svg
              viewBox={`0 0 ${chartWidth} ${chartHeight}`}
              width="100%"
              style={{ height: "auto", overflow: "visible" }}
              role="img"
              aria-label="Recorded balance over time"
            >
              <defs>
                <linearGradient id="balanceAreaGrad" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="0%" stopColor="#22c55e" stopOpacity="0.25" />
                  <stop offset="100%" stopColor="#22c55e" stopOpacity="0.0" />
                </linearGradient>
              </defs>

              {/* Horizontal gridlines */}
              {[0, 0.5, 1].map((ratio) => {
                const y = chartHeight - paddingBottom - ratio * plotHeight;
                const labelVal = minVal + ratio * range;
                return (
                  <g key={ratio}>
                    <line
                      x1={paddingLeft}
                      y1={y}
                      x2={chartWidth - paddingRight}
                      y2={y}
                      stroke="rgba(255,255,255,0.06)"
                      strokeDasharray="3 3"
                    />
                    <text
                      x={paddingLeft - 6}
                      y={y + 3}
                      fill="var(--text-muted)"
                      fontSize="9"
                      textAnchor="end"
                    >
                      ${labelVal.toFixed(2)}
                    </text>
                  </g>
                );
              })}

              {/* Area gradient under line */}
              {areaPath && <path d={areaPath} fill="url(#balanceAreaGrad)" />}

              {/* Polyline connecting snapshots */}
              <polyline
                points={polylinePoints}
                fill="none"
                stroke="#22c55e"
                strokeWidth="2.5"
                strokeLinejoin="round"
                strokeLinecap="round"
              />

              {/* Snapshot points with hover tooltips */}
              {points.map((p, idx) => (
                <g key={idx}>
                  <circle
                    cx={p.x}
                    cy={p.y}
                    r="4"
                    fill="#22c55e"
                    stroke="#0f172a"
                    strokeWidth="1.5"
                  >
                    <title>{`${formatFullDate(p.snap.timestamp)}: $${p.snap.total.toFixed(2)} ${currency}${p.snap.paid !== null && p.snap.paid !== undefined ? ` (Paid: $${p.snap.paid.toFixed(2)}, Gift: $${(p.snap.gift ?? 0).toFixed(2)})` : ""}`}</title>
                  </circle>
                  {/* Label for first and last points */}
                  {(idx === 0 || idx === points.length - 1 || (points.length <= 6)) && (
                    <text
                      x={p.x}
                      y={chartHeight - 6}
                      fill="var(--text-muted)"
                      fontSize="9"
                      textAnchor="middle"
                    >
                      {formatTime(p.snap.timestamp)}
                    </text>
                  )}
                </g>
              ))}
            </svg>

            {/* Chart footer */}
            <div
              style={{
                display: "flex",
                justifyContent: "space-between",
                alignItems: "center",
                fontSize: "0.74rem",
                color: "var(--text-muted)",
                marginTop: "0.4rem",
                paddingTop: "0.3rem",
                borderTop: "1px solid rgba(255,255,255,0.05)",
              }}
            >
              <span>First: {oldest ? formatFullDate(oldest.timestamp) : "—"}</span>
              <span>Latest: {latest ? formatFullDate(latest.timestamp) : "—"}</span>
            </div>
          </div>
        )}
      </div>
      )}
    </div>
  );
}
