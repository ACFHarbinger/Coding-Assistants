import React, { useEffect, useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ScorecardView, AgentScorecardRow, HarnessScorecardRow, ProviderScorecardRow, FigureProvenance } from "./types";

export function ScorecardTab() {
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
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", flexWrap: "wrap", gap: "1rem" }}>
        <div>
          <h3 style={{ margin: 0, fontSize: "1.2rem", color: "var(--text-color, #fff)" }}>Team Scorecard</h3>
          <p style={{ margin: "0.25rem 0 0 0", fontSize: "0.85rem", color: "var(--text-muted, #94a3b8)" }}>
            Projected security posture, operational backlog, harness coverage, and measured provider usage.
          </p>
        </div>
        <div style={{ display: "flex", alignItems: "center", gap: "1rem" }}>
          {data?.generated_at && (
            <span style={{ fontSize: "0.75rem", color: "var(--text-muted, #94a3b8)" }}>
              Updated: {new Date(data.generated_at).toLocaleTimeString()}
            </span>
          )}
          <button
            onClick={() => void loadScorecard()}
            disabled={loading}
            style={{
              padding: "0.35rem 0.8rem",
              borderRadius: "6px",
              background: "rgba(255, 255, 255, 0.08)",
              border: "1px solid rgba(255, 255, 255, 0.15)",
              color: "var(--text-color, #fff)",
              cursor: loading ? "wait" : "pointer",
              fontSize: "0.85rem",
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
                    background: "rgba(255, 255, 255, 0.03)",
                    border: hasOpenSec ? "1px solid rgba(239, 68, 68, 0.5)" : "1px solid rgba(255, 255, 255, 0.1)",
                    borderRadius: "8px",
                    padding: "1rem",
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

                  {/* Cost & Budget info */}
                  <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", paddingTop: "0.5rem", borderTop: "1px solid rgba(255, 255, 255, 0.06)", fontSize: "0.85rem" }}>
                    <div>
                      <div style={{ fontSize: "0.75rem", color: "var(--text-muted, #94a3b8)", marginBottom: "0.15rem" }}>Attributed Cost</div>
                      <div style={{ display: "flex", alignItems: "center", gap: "0.4rem" }}>
                        <span style={{ fontWeight: 600 }}>
                          {agent.cost.value != null ? agent.cost.value.toLocaleString(undefined, { maximumFractionDigits: 4 }) : "—"}
                        </span>
                        {renderProvenanceChip(agent.cost.provenance)}
                      </div>
                    </div>

                    {agent.budget_limit != null && (
                      <div style={{ textAlign: "right" }}>
                        <div style={{ fontSize: "0.75rem", color: "var(--text-muted, #94a3b8)", marginBottom: "0.15rem" }}>Budget</div>
                        <div style={{ fontSize: "0.8rem" }}>
                          {agent.budget_spent?.toFixed(1) ?? "0"} / {agent.budget_limit.toFixed(1)}
                          {agent.budget_paused && <span style={{ color: "#ef4444", marginLeft: "0.25rem" }}>(Paused)</span>}
                        </div>
                      </div>
                    )}
                  </div>
                </div>
              );
            })}
          </div>
        ) : (
          <div style={{ color: "var(--text-muted, #94a3b8)", fontSize: "0.85rem" }}>No agent rows available.</div>
        )}
      </div>

      {/* Section 2: Harness Coverage */}
      <div>
        <div style={{ display: "flex", alignItems: "baseline", gap: "0.5rem", marginBottom: "0.75rem" }}>
          <h4 style={{ margin: 0, fontSize: "1rem", color: "var(--text-color, #fff)" }}>Harness Coverage</h4>
          <span style={{ fontSize: "0.8rem", color: "var(--text-muted, #94a3b8)" }}>
            Tracking managed vs observed registrations across workspaces
          </span>
        </div>

        {data?.harnesses && data.harnesses.length > 0 ? (
          <div style={{ overflowX: "auto", background: "rgba(255, 255, 255, 0.02)", borderRadius: "8px", border: "1px solid rgba(255, 255, 255, 0.08)" }}>
            <table style={{ width: "100%", borderCollapse: "collapse", fontSize: "0.85rem", textAlign: "left" }}>
              <thead>
                <tr style={{ borderBottom: "1px solid rgba(255, 255, 255, 0.1)", background: "rgba(255, 255, 255, 0.02)" }}>
                  <th style={{ padding: "0.6rem 1rem", color: "var(--text-muted, #94a3b8)" }}>Harness</th>
                  <th style={{ padding: "0.6rem 1rem", color: "var(--text-muted, #94a3b8)" }}>Coverage</th>
                  <th style={{ padding: "0.6rem 1rem", color: "var(--text-muted, #94a3b8)" }}>Workspace</th>
                  <th style={{ padding: "0.6rem 1rem", color: "var(--text-muted, #94a3b8)" }}>State</th>
                  <th style={{ padding: "0.6rem 1rem", color: "var(--text-muted, #94a3b8)" }}>Session</th>
                </tr>
              </thead>
              <tbody>
                {data.harnesses.map((h: HarnessScorecardRow, idx: number) => (
                  <tr key={`${h.harness}-${h.workspace}-${idx}`} style={{ borderBottom: "1px solid rgba(255, 255, 255, 0.04)" }}>
                    <td style={{ padding: "0.6rem 1rem", fontWeight: 600 }}>{h.harness}</td>
                    <td style={{ padding: "0.6rem 1rem" }}>{renderCoverageChip(h.coverage)}</td>
                    <td style={{ padding: "0.6rem 1rem", color: "var(--text-muted, #94a3b8)" }}>
                      <code>{h.workspace}</code>
                    </td>
                    <td style={{ padding: "0.6rem 1rem", color: "var(--text-muted, #94a3b8)" }}>{h.state || "—"}</td>
                    <td style={{ padding: "0.6rem 1rem", color: "var(--text-muted, #94a3b8)" }}>
                      {h.disk_session_id ? <code>{h.disk_session_id}</code> : "—"}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ) : (
          <div style={{ color: "var(--text-muted, #94a3b8)", fontSize: "0.85rem" }}>No harness records available.</div>
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
                  background: "rgba(255, 255, 255, 0.03)",
                  border: "1px solid rgba(255, 255, 255, 0.08)",
                  borderRadius: "8px",
                  padding: "1rem",
                  display: "flex",
                  flexDirection: "column",
                  gap: "0.5rem",
                }}
              >
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                  <div style={{ fontWeight: 600, fontSize: "0.95rem" }}>{p.provider}</div>
                  {p.via_gateway && (
                    <span style={{ fontSize: "0.75rem", padding: "0.15rem 0.4rem", borderRadius: "4px", background: "rgba(168, 85, 247, 0.15)", color: "#c084fc", border: "1px solid rgba(168, 85, 247, 0.3)" }}>
                      via {p.via_gateway}
                    </span>
                  )}
                </div>

                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", fontSize: "0.85rem", marginTop: "0.25rem" }}>
                  <span style={{ color: "var(--text-muted, #94a3b8)" }}>Measured Tokens:</span>
                  <div style={{ display: "flex", alignItems: "center", gap: "0.3rem" }}>
                    <span>{p.tokens.value != null ? p.tokens.value.toLocaleString() : "—"}</span>
                    {renderProvenanceChip(p.tokens.provenance)}
                  </div>
                </div>

                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", fontSize: "0.85rem" }}>
                  <span style={{ color: "var(--text-muted, #94a3b8)" }}>Measured Cost:</span>
                  <div style={{ display: "flex", alignItems: "center", gap: "0.3rem" }}>
                    <span>{p.cost.value != null ? `$${p.cost.value.toFixed(4)}` : "—"}</span>
                    {renderProvenanceChip(p.cost.provenance)}
                  </div>
                </div>

                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", fontSize: "0.85rem" }}>
                  <span style={{ color: "var(--text-muted, #94a3b8)" }}>Requests:</span>
                  <span style={{ color: "var(--text-muted, #94a3b8)" }}>{p.request_count.value ?? 0}</span>
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
