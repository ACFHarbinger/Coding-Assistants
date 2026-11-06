import { useState } from "react";
import { nocPanelStyle, nocCardDenseStyle } from "../../common/telemetry/panel";
import { EmptyStateChip, StalenessDot } from "../../common/telemetry/status";
import { NOC_COLORS, NOC_SURFACES } from "../../common/telemetry/tokens";
import { inputStyle } from "../hub/HubCharts";
import type { AgentRecord, WakeRecord } from "../hub/types";

function getStatusBadge(status: string) {
  const s = status.toLowerCase();
  if (s === "pending") {
    return {
      label: "Pending",
      bg: "rgba(234, 179, 8, 0.15)",
      border: "rgba(234, 179, 8, 0.35)",
      color: NOC_COLORS.warn,
    };
  }
  if (s === "delivered" || s === "completed" || s === "active") {
    return {
      label: "Delivered",
      bg: "rgba(16, 185, 129, 0.15)",
      border: "rgba(16, 185, 129, 0.35)",
      color: NOC_COLORS.ok,
    };
  }
  return {
    label: status ? status.charAt(0).toUpperCase() + status.slice(1) : "Unknown",
    bg: "rgba(148, 163, 184, 0.15)",
    border: "rgba(148, 163, 184, 0.35)",
    color: NOC_COLORS.idle,
  };
}

export default function WakesPanel({
  agents,
  wakeTarget,
  setWakeTarget,
  wakeReason,
  setWakeReason,
  requestWake,
  refreshWakes,
  wakes,
  lastUpdatedUnixSeconds,
}: {
  agents: AgentRecord[];
  wakeTarget: string;
  setWakeTarget: (next: string) => void;
  wakeReason: string;
  setWakeReason: (next: string) => void;
  requestWake: () => void;
  refreshWakes: () => void;
  wakes: WakeRecord[];
  lastUpdatedUnixSeconds?: number;
}) {
  const [localRefreshedAt, setLocalRefreshedAt] = useState<number>(() => Math.floor(Date.now() / 1000));
  const effectiveTimestamp = lastUpdatedUnixSeconds ?? localRefreshedAt;

  const handleRefresh = () => {
    setLocalRefreshedAt(Math.floor(Date.now() / 1000));
    refreshWakes();
  };

  return (
    <div className="fade-in" style={{ display: "flex", flexDirection: "column", gap: "1.25rem" }}>
      {/* Request Wake Form */}
      <div style={{ ...nocPanelStyle, display: "grid", gap: "1rem" }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", flexWrap: "wrap", gap: "0.5rem" }}>
          <div>
            <h3 style={{ margin: 0, fontSize: "1.05rem", fontWeight: 700, color: NOC_SURFACES.textPrimary }}>Request Wake</h3>
            <p style={{ margin: "0.2rem 0 0", fontSize: "0.8rem", color: NOC_SURFACES.textMuted }}>
              Enqueue an explicit agent wake request through the human oversight gate.
            </p>
          </div>
          <div style={{ display: "flex", alignItems: "center", gap: "0.75rem" }}>
            <StalenessDot lastUpdatedUnixSeconds={effectiveTimestamp} pollIntervalSeconds={60} labelPrefix="live" />
            <button className="btn-secondary noc-focusable" onClick={handleRefresh}>
              Refresh List
            </button>
          </div>
        </div>

        <div style={{ display: "flex", gap: "0.75rem", flexWrap: "wrap", alignItems: "center" }}>
          <select
            className="noc-focusable"
            value={wakeTarget}
            onChange={(e) => setWakeTarget(e.target.value)}
            style={{ ...inputStyle, minWidth: 160 }}
          >
            {agents.map((a) => (
              <option key={a.id} value={a.id}>
                {a.display_name}
              </option>
            ))}
          </select>
          <input
            className="noc-focusable"
            style={{ ...inputStyle, flex: 1, minWidth: 220 }}
            placeholder="Reason for waking…"
            value={wakeReason}
            onChange={(e) => setWakeReason(e.target.value)}
          />
          <button className="btn-primary noc-focusable" onClick={requestWake}>
            Wake (human gate)
          </button>
        </div>
      </div>

      {/* Wakes Stream */}
      <div style={{ display: "flex", flexDirection: "column", gap: "0.75rem", maxHeight: 440, overflowY: "auto", paddingRight: "0.25rem" }}>
        {wakes.length === 0 && (
          <div style={{ display: "flex", justifyContent: "center", padding: "2.5rem 1rem", width: "100%" }}>
            <EmptyStateChip
              title="No wake requests recorded"
              detail="Submit a wake request above to enqueue an agent wake."
              tone="idle"
            />
          </div>
        )}
        {wakes.map((w) => {
          const badge = getStatusBadge(w.status);
          return (
            <div
              key={w.id}
              style={{
                ...nocCardDenseStyle,
                display: "flex",
                justifyContent: "space-between",
                alignItems: "center",
                gap: "1rem",
                flexWrap: "wrap",
              }}
            >
              <div style={{ minWidth: 200, flex: 1 }}>
                <div style={{ display: "flex", alignItems: "center", gap: "0.6rem", marginBottom: "0.3rem" }}>
                  <span style={{ fontSize: "1rem", fontWeight: 700, color: NOC_COLORS.cyan }}>
                    {w.target_agent}
                  </span>
                  <span
                    style={{
                      fontSize: "0.72rem",
                      fontWeight: 700,
                      padding: "0.15rem 0.5rem",
                      borderRadius: "6px",
                      background: badge.bg,
                      color: badge.color,
                      border: `1px solid ${badge.border}`,
                    }}
                  >
                    {badge.label}
                  </span>
                </div>
                <div style={{ fontSize: "0.88rem", color: NOC_SURFACES.textSecondary, marginBottom: "0.35rem" }}>
                  {w.reason || <span style={{ color: NOC_SURFACES.textMuted, fontStyle: "italic" }}>No reason provided</span>}
                </div>
                <div
                  className="tabular-nums"
                  style={{ fontSize: "0.74rem", color: NOC_SURFACES.textMuted, fontFamily: "var(--font-mono)" }}
                >
                  {w.created_at}
                </div>
              </div>

              <div style={{ display: "flex", flexDirection: "column", alignItems: "flex-end", gap: "0.4rem" }}>
                {w.requires_human_gate && (
                  <span
                    style={{
                      fontSize: "0.75rem",
                      fontWeight: 600,
                      color: NOC_COLORS.warn,
                      background: "rgba(234, 179, 8, 0.12)",
                      border: "1px solid rgba(234, 179, 8, 0.3)",
                      padding: "0.2rem 0.55rem",
                      borderRadius: "6px",
                      display: "inline-flex",
                      alignItems: "center",
                      gap: "0.35rem",
                    }}
                  >
                    <i style={{ width: 6, height: 6, borderRadius: "50%", background: NOC_COLORS.warn, display: "inline-block" }} />
                    Human Gate Required
                  </span>
                )}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
