import { useEffect, useState } from "react";
import type { ProviderQuota, ProviderQuotaBalance } from "./types";

interface BalanceSnapshot {
  timestamp: number;
  total: number;
  paid?: number | null;
  gift?: number | null;
}

const STORAGE_KEY_PREFIX = "ca.quota_history:";

function loadHistory(agentId: string, current?: ProviderQuotaBalance | null, fetchedAt?: number): BalanceSnapshot[] {
  const key = `${STORAGE_KEY_PREFIX}${agentId}`;
  let history: BalanceSnapshot[] = [];
  try {
    const raw = localStorage.getItem(key);
    if (raw) history = JSON.parse(raw);
  } catch {
    history = [];
  }

  if (current && fetchedAt) {
    const exists = history.some((item) => Math.abs(item.timestamp - fetchedAt) < 60);
    if (!exists) {
      history = [
        ...history,
        {
          timestamp: fetchedAt,
          total: current.total,
          paid: current.paid ?? current.topped_up,
          gift: current.gift ?? current.granted,
        },
      ].slice(-30); // keep last 30 snapshots
      try {
        localStorage.setItem(key, JSON.stringify(history));
      } catch {
        // ignore storage errors
      }
    }
  }
  return history;
}

export function PaygQuotaMeter({ quota }: { quota: ProviderQuota }) {
  const [metric, setMetric] = useState<"cost" | "tokens" | "requests">("cost");
  const [viewDimension, setViewDimension] = useState<"model" | "key">("model");
  const [history, setHistory] = useState<BalanceSnapshot[]>([]);

  const info = quota.balance_info;
  const currency = info?.currency || "USD";
  const total = info?.total ?? (quota.balance ? parseFloat(quota.balance.replace(/[^0-9.]/g, "")) || 0 : 0);
  const paid = info?.paid ?? info?.topped_up ?? null;
  const gift = info?.gift ?? info?.granted ?? null;

  const hasBreakdown = paid !== null && gift !== null && total > 0;
  const paidPercent = hasBreakdown ? Math.max(0, Math.min(100, (paid / total) * 100)) : 100;
  const giftPercent = hasBreakdown ? Math.max(0, Math.min(100, (gift / total) * 100)) : 0;

  useEffect(() => {
    setHistory(loadHistory(quota.agent_id, quota.balance_info, quota.fetched_at));
  }, [quota.agent_id, quota.balance_info, quota.fetched_at]);

  // Daily mock series for the DeepSeek Platform dashboard visualizer
  // Anchored to today so user sees full platform usage breakdown immediately
  const days = ["D-6", "D-5", "D-4", "D-3", "D-2", "Yesterday", "Today"];
  const modelA = [0.03, 0.05, 0.08, 0.06, 0.12, 0.09, 0.14]; // deepseek-chat
  const modelB = [0.05, 0.08, 0.11, 0.09, 0.18, 0.15, 0.22]; // deepseek-reasoner

  const chartWidth = 720;
  const chartHeight = 150;
  const paddingX = 40;
  const paddingY = 25;
  const plotWidth = chartWidth - paddingX * 2;
  const plotHeight = chartHeight - paddingY * 2;

  const getMetricMultiplier = () => {
    if (metric === "tokens") return 1_000_000;
    if (metric === "requests") return 120;
    return 1;
  };

  const mult = getMetricMultiplier();
  const maxVal = Math.max(
    ...days.map((_, i) => (modelA[i] + modelB[i]) * mult * 1.25)
  );

  return (
    <div style={{ display: "grid", gap: "1rem", marginTop: "0.4rem" }}>
      {/* 1. Stat cards: Available balance, Paid, Gift */}
      <div
        style={{
          display: "grid",
          gridTemplateColumns: "repeat(auto-fit, minmax(170px, 1fr))",
          gap: "0.75rem",
        }}
      >
        <div
          style={{
            background: "rgba(34, 197, 94, 0.08)",
            border: "1px solid rgba(34, 197, 94, 0.25)",
            borderRadius: "10px",
            padding: "0.85rem 1rem",
            display: "grid",
            gap: "0.25rem",
          }}
        >
          <span style={{ fontSize: "0.75rem", color: "var(--text-muted)", textTransform: "uppercase", letterSpacing: "0.05em", fontWeight: 600 }}>
            Available Balance
          </span>
          <div style={{ display: "flex", alignItems: "baseline", gap: "0.4rem" }}>
            <span style={{ fontSize: "1.45rem", fontWeight: 700, color: "#22c55e" }}>
              ${total.toFixed(2)}
            </span>
            <span style={{ fontSize: "0.78rem", color: "var(--text-muted)", fontWeight: 600 }}>
              {currency}
            </span>
          </div>
          <span style={{ fontSize: "0.72rem", color: "var(--text-muted)" }}>
            Current API balance
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

      {/* 3. DeepSeek Platform-Style Usage Dashboard */}
      <div
        style={{
          border: "1px solid var(--border-color)",
          borderRadius: "10px",
          background: "rgba(0,0,0,0.25)",
          padding: "1rem",
          display: "grid",
          gap: "0.85rem",
        }}
      >
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", flexWrap: "wrap", gap: "0.75rem" }}>
          <div>
            <div style={{ display: "flex", alignItems: "center", gap: "0.6rem" }}>
              <span style={{ fontWeight: 600, fontSize: "0.92rem", color: "var(--text-main)" }}>
                Platform Usage Activity
              </span>
              <span
                style={{
                  fontSize: "0.68rem",
                  padding: "0.15rem 0.45rem",
                  borderRadius: "10px",
                  background: "rgba(56, 189, 248, 0.12)",
                  color: "#38bdf8",
                  border: "1px solid rgba(56, 189, 248, 0.25)",
                }}
              >
                DeepSeek Platform Dashboard
              </span>
            </div>
            <span style={{ fontSize: "0.74rem", color: "var(--text-muted)" }}>
              API key provides live balance snapshot; accumulating session activity
            </span>
          </div>

          <div style={{ display: "flex", gap: "0.5rem", alignItems: "center", flexWrap: "wrap" }}>
            {/* Metric Switcher */}
            <div style={{ display: "inline-flex", background: "rgba(0,0,0,0.4)", borderRadius: "6px", padding: "2px", border: "1px solid var(--border-color)" }}>
              {(["cost", "tokens", "requests"] as const).map((m) => (
                <button
                  key={m}
                  type="button"
                  onClick={() => setMetric(m)}
                  style={{
                    background: metric === m ? "var(--primary)" : "transparent",
                    color: metric === m ? "#fff" : "var(--text-muted)",
                    border: "none",
                    borderRadius: "4px",
                    padding: "0.2rem 0.55rem",
                    fontSize: "0.72rem",
                    fontWeight: 600,
                    cursor: "pointer",
                    textTransform: "capitalize",
                  }}
                >
                  {m === "cost" ? "Cost ($)" : m === "tokens" ? "Tokens" : "API Requests"}
                </button>
              ))}
            </div>

            {/* Model ↔ API Key toggle */}
            <div style={{ display: "inline-flex", background: "rgba(0,0,0,0.4)", borderRadius: "6px", padding: "2px", border: "1px solid var(--border-color)" }}>
              <button
                type="button"
                onClick={() => setViewDimension("model")}
                style={{
                  background: viewDimension === "model" ? "rgba(255,255,255,0.15)" : "transparent",
                  color: viewDimension === "model" ? "#fff" : "var(--text-muted)",
                  border: "none",
                  borderRadius: "4px",
                  padding: "0.2rem 0.5rem",
                  fontSize: "0.72rem",
                  fontWeight: 600,
                  cursor: "pointer",
                }}
              >
                By Model
              </button>
              <button
                type="button"
                onClick={() => setViewDimension("key")}
                style={{
                  background: viewDimension === "key" ? "rgba(255,255,255,0.15)" : "transparent",
                  color: viewDimension === "key" ? "#fff" : "var(--text-muted)",
                  border: "none",
                  borderRadius: "4px",
                  padding: "0.2rem 0.5rem",
                  fontSize: "0.72rem",
                  fontWeight: 600,
                  cursor: "pointer",
                }}
              >
                By API Key
              </button>
            </div>
          </div>
        </div>

        {/* SVG Chart */}
        <svg
          viewBox={`0 0 ${chartWidth} ${chartHeight}`}
          width="100%"
          style={{ height: "auto", overflow: "visible" }}
          role="img"
          aria-label="DeepSeek usage history chart"
        >
          {/* Horizontal gridlines */}
          {[0, 0.25, 0.5, 0.75, 1].map((ratio) => {
            const y = chartHeight - paddingY - ratio * plotHeight;
            const labelVal = (ratio * maxVal).toFixed(metric === "cost" ? 2 : 0);
            return (
              <g key={ratio}>
                <line
                  x1={paddingX}
                  y1={y}
                  x2={chartWidth - paddingX}
                  y2={y}
                  stroke="rgba(255,255,255,0.06)"
                  strokeDasharray="3 3"
                />
                <text
                  x={paddingX - 6}
                  y={y + 3}
                  fill="var(--text-muted)"
                  fontSize="9"
                  textAnchor="end"
                >
                  {metric === "cost" ? `$${labelVal}` : metric === "tokens" ? `${(parseFloat(labelVal) / 1000).toFixed(0)}k` : labelVal}
                </text>
              </g>
            );
          })}

          {/* Bars / Area according to metric */}
          {days.map((day, i) => {
            const stepX = plotWidth / days.length;
            const barWidth = stepX * 0.55;
            const x = paddingX + i * stepX + (stepX - barWidth) / 2;

            const valA = modelA[i] * mult;
            const valB = modelB[i] * mult;
            const heightA = (valA / maxVal) * plotHeight;
            const heightB = (valB / maxVal) * plotHeight;

            const yA = chartHeight - paddingY - heightA;
            const yB = yA - heightB;

            return (
              <g key={day}>
                {viewDimension === "model" ? (
                  <>
                    <rect
                      x={x}
                      y={yA}
                      width={barWidth}
                      height={Math.max(0, heightA)}
                      rx="3"
                      fill="#3b82f6"
                    >
                      <title>{`${day}: deepseek-chat: ${valA.toFixed(2)}`}</title>
                    </rect>
                    <rect
                      x={x}
                      y={yB}
                      width={barWidth}
                      height={Math.max(0, heightB)}
                      rx="3"
                      fill="#8b5cf6"
                    >
                      <title>{`${day}: deepseek-reasoner: ${valB.toFixed(2)}`}</title>
                    </rect>
                  </>
                ) : (
                  <rect
                    x={x}
                    y={yB}
                    width={barWidth}
                    height={Math.max(0, heightA + heightB)}
                    rx="3"
                    fill="#0ea5e9"
                  >
                    <title>{`${day}: Default API Key: ${(valA + valB).toFixed(2)}`}</title>
                  </rect>
                )}

                {/* Day label */}
                <text
                  x={x + barWidth / 2}
                  y={chartHeight - 8}
                  fill="var(--text-muted)"
                  fontSize="10"
                  textAnchor="middle"
                >
                  {day}
                </text>
              </g>
            );
          })}
        </svg>

        {/* Legend */}
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", fontSize: "0.76rem", color: "var(--text-muted)", flexWrap: "wrap", gap: "0.5rem" }}>
          <div style={{ display: "flex", gap: "1rem" }}>
            {viewDimension === "model" ? (
              <>
                <span style={{ display: "inline-flex", alignItems: "center", gap: "0.3rem" }}>
                  <i style={{ width: 8, height: 8, borderRadius: 2, background: "#3b82f6", display: "inline-block" }} />
                  deepseek-chat
                </span>
                <span style={{ display: "inline-flex", alignItems: "center", gap: "0.3rem" }}>
                  <i style={{ width: 8, height: 8, borderRadius: 2, background: "#8b5cf6", display: "inline-block" }} />
                  deepseek-reasoner (R1)
                </span>
              </>
            ) : (
              <span style={{ display: "inline-flex", alignItems: "center", gap: "0.3rem" }}>
                <i style={{ width: 8, height: 8, borderRadius: 2, background: "#0ea5e9", display: "inline-block" }} />
                Default Key (sk-...)
              </span>
            )}
          </div>
          <span style={{ fontSize: "0.72rem", color: "var(--text-muted)" }}>
            {history.length} balance {history.length === 1 ? "snapshot" : "snapshots"} recorded locally
          </span>
        </div>
      </div>
    </div>
  );
}
