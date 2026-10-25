import React, { useEffect, useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ScorecardView, AgentScorecardRow, HarnessScorecardRow, ProviderScorecardRow, FigureProvenance } from "./types";
import { ScorecardDigest } from "./ScorecardDigest";
import { ScorecardOversight } from "./ScorecardOversight";
import { Meter } from "../../common/telemetry/gauges";
import { nocCardDenseStyle, nocPanelStyle } from "../../common/telemetry/panel";
import { EmptyStateChip, StalenessDot } from "../../common/telemetry/status";
import { NOC_COLORS, NOC_SURFACES } from "../../common/telemetry/tokens";

export function ScorecardTab() {
  const renderCoverage = (coverage?: { measured_requests: number; total_requests: number } | null) =>
    coverage && coverage.measured_requests < coverage.total_requests ? (
      <small title="Subtotal of available measurements; other requests are missing complete measurements">
        Partial: {coverage.measured_requests}/{coverage.total_requests} requests fully measured
      </small>
    ) : null;
  const [data, setData] = useState<ScorecardView | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadScorecard = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const res = await invoke<ScorecardView>("hub_get_scorecard", { callerAgentId: null });
      setData(res);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadScorecard();
  }, [loadScorecard]);

  const renderProvenanceChip = (prov: FigureProvenance | string) => {
    const style: React.CSSProperties = {
      display: "inline-flex",
      alignItems: "center",
      gap: "0.25rem",
      padding: "0.15rem 0.45rem",
      borderRadius: "4px",
      fontSize: "0.7rem",
      fontWeight: 600,
      textTransform: "uppercase",
      letterSpacing: "0.03em",
    };

    if (prov === "measured") {
      return (
        <span style={{ ...style, background: "rgba(16, 185, 129, 0.15)", color: "#10b981", border: "1px solid rgba(16, 185, 129, 0.3)" }}>
          measured
        </span>
      );
    }
    if (prov === "estimated") {
      return (
        <span style={{ ...style, background: "rgba(245, 158, 11, 0.15)", color: "#f59e0b", border: "1px solid rgba(245, 158, 11, 0.3)" }}>
          estimated
        </span>
      );
    }
    return (
      <span style={{ ...style, background: "rgba(148, 163, 184, 0.15)", color: "#94a3b8", border: "1px solid rgba(148, 163, 184, 0.3)" }}>
        unknown
      </span>
    );
  };

  const renderCoverageChip = (coverage: string) => {
    const base: React.CSSProperties = {
      display: "inline-block",
      padding: "0.2rem 0.5rem",
      borderRadius: "9999px",
      fontSize: "0.75rem",
      fontWeight: 600,
      textTransform: "capitalize",
    };

    if (coverage === "managed") {
      return <span style={{ ...base, background: "rgba(34, 197, 94, 0.2)", color: "#22c55e", border: "1px solid rgba(34, 197, 94, 0.4)" }}>managed</span>;
    }
    if (coverage === "observed") {
      return <span style={{ ...base, background: "rgba(59, 130, 246, 0.2)", color: "#60a5fa", border: "1px solid rgba(59, 130, 246, 0.4)" }}>observed</span>;
    }
    return <span style={{ ...base, background: "rgba(100, 116, 139, 0.2)", color: "#94a3b8", border: "1px solid rgba(100, 116, 139, 0.4)" }}>unregistered</span>;
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "1.5rem" }}>
      {/* Top Header */}
      <div style={{ ...nocPanelStyle, display: "flex", justifyContent: "space-between", alignItems: "center", flexWrap: "wrap", gap: "1rem" }}>
        <div>
          <div style={{ display: "flex", alignItems: "center", gap: "0.65rem", flexWrap: "wrap" }}>
            <h3 style={{ margin: 0, fontSize: "1.1rem", fontWeight: 700, color: NOC_SURFACES.textPrimary }}>Team Scorecard</h3>
            <StalenessDot lastUpdatedUnixSeconds={data?.generated_at ? Math.floor(Date.parse(data.generated_at) / 1000) : null} pollIntervalSeconds={60} />
          </div>
          <p style={{ margin: "0.25rem 0 0 0", fontSize: "0.82rem", color: NOC_SURFACES.textMuted }}>
            Projected security posture, operational backlog, harness coverage, and measured provider usage.
          </p>
        </div>
        <div style={{ display: "flex", alignItems: "center", gap: "0.85rem" }}>
          {data?.generated_at && (
            <span className="tabular-nums" style={{ fontSize: "0.75rem", color: NOC_SURFACES.textMuted }}>
              Updated: {new Date(data.generated_at).toLocaleTimeString()}
            </span>
          )}
          <button
            type="button"
            className="btn-secondary noc-focusable"
            onClick={() => void loadScorecard()}
            disabled={loading}
            style={{
              marginTop: 0,
              padding: "0.35rem 0.8rem",
              fontSize: "0.8rem",
              cursor: loading ? "wait" : "pointer",
            }}
          >
            {loading ? "Refreshing…" : "Refresh"}
          </button>
        </div>
      </div>

      {error && (
        <div style={{ padding: "0.75rem", borderRadius: "6px", background: "rgba(239, 68, 68, 0.15)", border: "1px solid rgba(239, 68, 68, 0.3)", color: "#ef4444", fontSize: "0.85rem" }}>
          Failed to load scorecard: {error}
        </div>
      )}

      {/* Section 0: Owner Review Digest (T4 / #344) — owner-only, advisory */}
      <ScorecardDigest digest={data?.digest ?? null} />

      {/* Section 1: Agents */}
      <div>
        <div style={{ display: "flex", alignItems: "baseline", gap: "0.5rem", marginBottom: "0.75rem" }}>
          <h4 style={{ margin: 0, fontSize: "1rem", color: "var(--text-color, #fff)" }}>Agent Cards</h4>
          <span style={{ fontSize: "0.8rem", color: "var(--text-muted, #94a3b8)" }}>
            Sorted by open security events, oldest pending decisions, then measured cost (never composite)
          </span>
        </div>

        {data?.agents && data.agents.length > 0 ? (
          <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(320px, 1fr))", gap: "1rem" }}>
            {data.agents.map((agent: AgentScorecardRow) => {
              const hasOpenSec = agent.open_security_events.length > 0;
              return (
                <div
                  key={agent.agent_id}
                  style={{
                    ...nocCardDenseStyle,
                    borderLeft: hasOpenSec ? `3px solid ${NOC_COLORS.crit}` : `3px solid ${NOC_COLORS.cyan}`,
                    display: "flex",
                    flexDirection: "column",
                    gap: "0.75rem",
                  }}
                >
                  {/* Card Title & Role */}
                  <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start" }}>
                    <div>
                      <div style={{ fontWeight: 600, fontSize: "1rem", color: "var(--text-color, #fff)" }}>{agent.display_name}</div>
                      <code style={{ fontSize: "0.75rem", color: "var(--text-muted, #94a3b8)" }}>{agent.agent_id}</code>
                    </div>
                    {agent.oversight_role && (
                      <span style={{ padding: "0.15rem 0.5rem", borderRadius: "4px", fontSize: "0.75rem", background: "rgba(99, 102, 241, 0.2)", color: "#818cf8", border: "1px solid rgba(99, 102, 241, 0.4)" }}>
                        {agent.oversight_role}
                      </span>
                    )}
                    {(agent.unresolved_disagreements ?? 0) > 0 && (
                      <span
                        title="Unresolved scorecard.disagreement rows naming this agent — inspect, not a score"
                        style={{ padding: "0.15rem 0.5rem", borderRadius: "4px", fontSize: "0.75rem", background: "rgba(245, 158, 11, 0.2)", color: "#fbbf24", border: "1px solid rgba(245, 158, 11, 0.4)" }}
                      >
                        ⚡ {agent.unresolved_disagreements} disagreement{agent.unresolved_disagreements! > 1 ? "s" : ""}
                      </span>
                    )}
                  </div>

                  {/* Security events banner */}
                  {hasOpenSec ? (
                    <div style={{ padding: "0.5rem", borderRadius: "6px", background: "rgba(239, 68, 68, 0.12)", border: "1px solid rgba(239, 68, 68, 0.25)" }}>
                      <div style={{ fontSize: "0.8rem", fontWeight: 600, color: "#f87171" }}>
                        ⚠️ {agent.open_security_events.length} Open Security Event{agent.open_security_events.length > 1 ? "s" : ""}
                      </div>
                      <div style={{ marginTop: "0.25rem", display: "flex", flexDirection: "column", gap: "0.2rem" }}>
                        {agent.open_security_events.slice(0, 3).map((e) => (
                          <div key={e.id} style={{ fontSize: "0.75rem", color: "#fca5a5" }}>
                            • <code>{e.operation}</code> ({e.status})
                          </div>
                        ))}
                      </div>
                    </div>
                  ) : (
                    <div style={{ fontSize: "0.8rem", color: "#34d399", display: "flex", alignItems: "center", gap: "0.25rem" }}>
                      ✓ 0 open security alerts
                    </div>
                  )}

                  {/* Pending human decision */}
                  <div style={{ fontSize: "0.8rem" }}>
                    <span style={{ color: "var(--text-muted, #94a3b8)" }}>Pending Human Decision: </span>
                    {agent.oldest_pending_human_decision ? (
                      <span style={{ color: "#fbbf24", fontWeight: 500 }}>
                        ⏳ Since {new Date(agent.oldest_pending_human_decision).toLocaleDateString()} {new Date(agent.oldest_pending_human_decision).toLocaleTimeString()}
                      </span>
                    ) : (
                      <span style={{ color: "var(--text-muted, #94a3b8)" }}>None</span>
                    )}
                  </div>

                  {/* Tokens, Cost & Budget info */}
                  <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start", paddingTop: "0.5rem", borderTop: "1px solid rgba(255, 255, 255, 0.06)", fontSize: "0.85rem", gap: "0.5rem", flexWrap: "wrap" }}>
                    <div>
                      <div style={{ fontSize: "0.75rem", color: "var(--text-muted, #94a3b8)", marginBottom: "0.15rem" }}>Tokens</div>
                      <div style={{ display: "flex", alignItems: "center", gap: "0.4rem" }}>
                        <span className="tabular-nums" style={{ fontWeight: 600 }}>
                          {agent.tokens?.value != null ? agent.tokens.value.toLocaleString() : "—"}
                        </span>
                        {agent.tokens && renderProvenanceChip(agent.tokens.provenance)}
                        {renderCoverage(agent.tokens?.coverage)}
                      </div>
                    </div>

                    <div>
                      <div style={{ fontSize: "0.75rem", color: "var(--text-muted, #94a3b8)", marginBottom: "0.15rem" }}>Attributed Cost</div>
                      <div style={{ display: "flex", alignItems: "center", gap: "0.4rem" }}>
                        <span className="tabular-nums" style={{ fontWeight: 600 }}>
                          {agent.cost.value != null ? (agent.cost.unit === "currency" ? `$${agent.cost.value.toFixed(4)}` : agent.cost.value.toLocaleString(undefined, { maximumFractionDigits: 4 })) : "—"}
                        </span>
                        {renderProvenanceChip(agent.cost.provenance)}
                        {renderCoverage(agent.cost.coverage)}
                      </div>
                    </div>

                    {agent.budget_limit != null && (
                      <div style={{ textAlign: "right" }}>
                        <div style={{ fontSize: "0.75rem", color: "var(--text-muted, #94a3b8)", marginBottom: "0.15rem" }}>Budget</div>
                        <div className="tabular-nums" style={{ fontSize: "0.8rem" }}>
                          {agent.budget_spent?.toFixed(1) ?? "0"} / {agent.budget_limit.toFixed(1)}
                          {agent.budget_paused && <span style={{ color: "#ef4444", marginLeft: "0.25rem" }}>(Paused)</span>}
                        </div>
                      </div>
                    )}
                  </div>
                  {agent.budget_limit != null && agent.budget_limit > 0 && (
                    <Meter
                      percent={((agent.budget_spent ?? 0) / agent.budget_limit) * 100}
                      detail={`${(((agent.budget_spent ?? 0) / agent.budget_limit) * 100).toFixed(0)}%`}
                      tone={agent.budget_paused ? "crit" : ((agent.budget_spent ?? 0) / agent.budget_limit) > 0.85 ? "warn" : "ok"}
                      label="Budget Utilization"
                    />
                  )}
                </div>
              );
            })}
          </div>
        ) : (
          <EmptyStateChip title="No agent rows available" detail="Agent scorecard rows appear once agents are registered in the Hub." tone="idle" />
        )}
      </div>

      {/* Section: Oversight (T7) — probes + reviewer signature */}
      <ScorecardOversight probes={data?.probes} reviewers={data?.reviewers} />

      {/* Section 2: Harness Coverage */}
      <div>
        <div style={{ display: "flex", alignItems: "baseline", gap: "0.5rem", marginBottom: "0.75rem" }}>
          <h4 style={{ margin: 0, fontSize: "1rem", color: NOC_SURFACES.textPrimary }}>Harness Coverage</h4>
          <span style={{ fontSize: "0.8rem", color: NOC_SURFACES.textMuted }}>
            Tracking managed vs observed registrations across workspaces
          </span>
        </div>

        {data?.harnesses && data.harnesses.length > 0 ? (
          <div style={{ ...nocPanelStyle, padding: 0, overflowX: "auto" }}>
            <table style={{ width: "100%", borderCollapse: "collapse", fontSize: "0.83rem", textAlign: "left" }}>
              <thead>
                <tr style={{ borderBottom: "1px solid rgba(148, 163, 184, 0.16)", background: "rgba(9, 14, 26, 0.7)" }}>
                  <th style={{ padding: "0.65rem 1rem", color: NOC_SURFACES.textMuted, fontSize: "0.72rem", textTransform: "uppercase", letterSpacing: "0.05em" }}>Harness</th>
                  <th style={{ padding: "0.65rem 1rem", color: NOC_SURFACES.textMuted, fontSize: "0.72rem", textTransform: "uppercase", letterSpacing: "0.05em" }}>Coverage</th>
                  <th style={{ padding: "0.65rem 1rem", color: NOC_SURFACES.textMuted, fontSize: "0.72rem", textTransform: "uppercase", letterSpacing: "0.05em" }}>Workspace</th>
                  <th style={{ padding: "0.65rem 1rem", color: NOC_SURFACES.textMuted, fontSize: "0.72rem", textTransform: "uppercase", letterSpacing: "0.05em" }}>State</th>
                  <th style={{ padding: "0.65rem 1rem", color: NOC_SURFACES.textMuted, fontSize: "0.72rem", textTransform: "uppercase", letterSpacing: "0.05em" }}>Session</th>
                </tr>
              </thead>
              <tbody>
                {data.harnesses.map((h: HarnessScorecardRow, idx: number) => (
                  <tr key={`${h.harness}-${h.workspace}-${idx}`} style={{ borderBottom: "1px solid rgba(148, 163, 184, 0.08)" }}>
                    <td style={{ padding: "0.6rem 1rem", fontWeight: 700, color: NOC_COLORS.cyan }}>{h.harness}</td>
                    <td style={{ padding: "0.6rem 1rem" }}>{renderCoverageChip(h.coverage)}</td>
                    <td style={{ padding: "0.6rem 1rem", color: NOC_SURFACES.textSecondary }}>
                      <code style={{ fontFamily: "var(--font-mono)", fontSize: "0.78rem" }}>{h.workspace}</code>
                    </td>
                    <td style={{ padding: "0.6rem 1rem", color: NOC_SURFACES.textMuted }}>{h.state || "—"}</td>
                    <td style={{ padding: "0.6rem 1rem", color: NOC_SURFACES.textMuted }}>
                      {h.disk_session_id ? <code style={{ fontFamily: "var(--font-mono)", fontSize: "0.78rem" }}>{h.disk_session_id}</code> : "—"}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ) : (
          <EmptyStateChip title="No harness records available" detail="Register an observed or managed harness session in Orchestrate." tone="idle" />
        )}
      </div>

      {/* Section 3: Model Providers */}
      <div>
        <div style={{ display: "flex", alignItems: "baseline", gap: "0.5rem", marginBottom: "0.75rem" }}>
          <h4 style={{ margin: 0, fontSize: "1rem", color: "var(--text-color, #fff)" }}>Provider Routing & Measured Usage</h4>
          <span style={{ fontSize: "0.8rem", color: "var(--text-muted, #94a3b8)" }}>
            Token and cost metrics from verified provider.usage audit entries
          </span>
        </div>

        {data?.providers && data.providers.length > 0 ? (
          <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(280px, 1fr))", gap: "1rem" }}>
            {data.providers.map((p: ProviderScorecardRow, idx: number) => (
              <div
                key={`${p.provider}-${p.via_gateway}-${idx}`}
                style={{
                  ...nocCardDenseStyle,
                  display: "flex",
                  flexDirection: "column",
                  gap: "0.5rem",
                }}
              >
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                  <div style={{ fontWeight: 600, fontSize: "0.95rem", color: NOC_COLORS.cyan }}>{p.provider}</div>
                  {p.via_gateway && (
                    <span style={{ fontSize: "0.75rem", padding: "0.15rem 0.4rem", borderRadius: "4px", background: "rgba(168, 85, 247, 0.15)", color: "#c084fc", border: "1px solid rgba(168, 85, 247, 0.3)" }}>
                      via {p.via_gateway}
                    </span>
                  )}
                </div>

                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", fontSize: "0.85rem", marginTop: "0.25rem" }}>
                  <span style={{ color: "var(--text-muted, #94a3b8)" }}>Measured Tokens:</span>
                  <div style={{ display: "flex", alignItems: "center", gap: "0.3rem" }}>
                    <span className="tabular-nums">{p.tokens.value != null ? p.tokens.value.toLocaleString() : "—"}</span>
                    {renderProvenanceChip(p.tokens.provenance)}
                    {renderCoverage(p.tokens.coverage)}
                  </div>
                </div>

                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", fontSize: "0.85rem" }}>
                  <span style={{ color: "var(--text-muted, #94a3b8)" }}>Measured Cost:</span>
                  <div style={{ display: "flex", alignItems: "center", gap: "0.3rem" }}>
                    <span className="tabular-nums">{p.cost.value != null ? `$${p.cost.value.toFixed(4)}` : "—"}</span>
                    {renderProvenanceChip(p.cost.provenance)}
                    {renderCoverage(p.cost.coverage)}
                  </div>
                </div>

                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", fontSize: "0.85rem" }}>
                  <span style={{ color: "var(--text-muted, #94a3b8)" }}>Requests:</span>
                  <span className="tabular-nums" style={{ color: "var(--text-muted, #94a3b8)" }}>{p.request_count.value ?? 0}</span>
                </div>
              </div>
            ))}
          </div>
        ) : (
          <div style={{ color: "var(--text-muted, #94a3b8)", fontSize: "0.85rem" }}>
            No measured provider usage recorded yet. Calls routed via gateway or direct provider will surface here once audited.
          </div>
        )}
      </div>
    </div>
  );
}
