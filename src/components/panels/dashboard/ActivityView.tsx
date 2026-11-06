import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "../../../lib/tauri";
import { AgentAvatar } from "../messager/AgentAvatar";
import type { ActivityFilter, ActivityItem } from "./types";
import { nocPanelStyle, nocCardDenseStyle } from "../../common/telemetry/panel";
import { EmptyStateChip } from "../../common/telemetry/status";
import { NOC_COLORS, NOC_SURFACES } from "../../common/telemetry/tokens";

interface AgentRecord {
  id: string;
  display_name: string;
  avatar_attachment_id?: string | null;
}

const badgeStyle = (bg: string, color = "#fff"): React.CSSProperties => ({
  fontSize: "0.72rem",
  padding: "0.15rem 0.5rem",
  borderRadius: "6px",
  background: bg,
  color,
  fontWeight: 700,
  letterSpacing: "0.03em",
  textTransform: "uppercase",
  display: "inline-flex",
  alignItems: "center",
  gap: "0.3rem",
});

const operationColor: Record<string, { bg: string; text: string }> = {
  created: { bg: "rgba(16, 185, 129, 0.15)", text: NOC_COLORS.ok },
  modified: { bg: "rgba(56, 189, 248, 0.15)", text: NOC_COLORS.cyan },
  removed: { bg: "rgba(239, 68, 68, 0.15)", text: NOC_COLORS.crit },
  accessed: { bg: "rgba(168, 85, 247, 0.15)", text: NOC_COLORS.purple },
};

export default function ActivityView({ agents }: { agents: AgentRecord[] }) {
  const [items, setItems] = useState<ActivityItem[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [selectedAgent, setSelectedAgent] = useState("");
  const [selectedTimeRange, setSelectedTimeRange] = useState("all");
  const [selectedKind, setSelectedKind] = useState("all");
  const [searchQuery, setSearchQuery] = useState("");
  const [lastRefreshed, setLastRefreshed] = useState("");
  const [expandedDetails, setExpandedDetails] = useState<Record<string, boolean>>({});

  const computeSinceTime = useCallback((preset: string): string | null => {
    const now = Date.now();
    switch (preset) {
      case "1h":
        return new Date(now - 3600 * 1000).toISOString();
      case "24h":
        return new Date(now - 24 * 3600 * 1000).toISOString();
      case "7d":
        return new Date(now - 7 * 24 * 3600 * 1000).toISOString();
      case "30d":
        return new Date(now - 30 * 24 * 3600 * 1000).toISOString();
      default:
        return null;
    }
  }, []);

  const fetchActivity = useCallback(async () => {
    setLoading(true);
    setError("");
    try {
      const since = computeSinceTime(selectedTimeRange);
      const filter: ActivityFilter = {
        agent: selectedAgent || null,
        since,
        kind: selectedKind === "all" ? null : selectedKind,
        limit: 100,
      };
      const result = await invoke<ActivityItem[]>("hub_get_activity_view", { filter });
      setItems(result);
      setLastRefreshed(new Date().toLocaleTimeString());
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [computeSinceTime, selectedAgent, selectedKind, selectedTimeRange]);

  useEffect(() => {
    void fetchActivity();
  }, [fetchActivity]);

  const toggleDetails = (id: string) => {
    setExpandedDetails((prev) => ({ ...prev, [id]: !prev[id] }));
  };

  const filteredItems = useMemo(() => {
    if (!searchQuery.trim()) return items;
    const query = searchQuery.toLowerCase();
    return items.filter(
      (item) =>
        item.title.toLowerCase().includes(query) ||
        item.workspace_path?.toLowerCase().includes(query) ||
        item.agents.some((a) => a.toLowerCase().includes(query)) ||
        item.commands.some((c) => c.raw.toLowerCase().includes(query)) ||
        item.files.some((f) => f.path.toLowerCase().includes(query)),
    );
  }, [items, searchQuery]);

  const summary = useMemo(() => {
    const uniqueFiles = new Set<string>();
    let totalCommands = 0;
    const activeAgents = new Set<string>();
    for (const item of filteredItems) {
      item.files.forEach((f) => uniqueFiles.add(f.path));
      totalCommands += item.commands.length;
      item.agents.forEach((a) => activeAgents.add(a));
    }
    return {
      totalItems: filteredItems.length,
      totalFiles: uniqueFiles.size,
      totalCommands,
      totalAgents: activeAgents.size,
    };
  }, [filteredItems]);

  const agentNameMap = useMemo(() => {
    const map = new Map<string, string>();
    for (const a of agents) map.set(a.id, a.display_name);
    return map;
  }, [agents]);

  const selectStyle: React.CSSProperties = {
    background: "rgba(0,0,0,0.4)",
    color: NOC_SURFACES.textPrimary,
    border: "1px solid rgba(148, 163, 184, 0.2)",
    borderRadius: "8px",
    padding: "0.35rem 0.75rem",
    fontSize: "0.82rem",
    outline: "none",
  };

  return (
    <div className="fade-in" style={{ display: "flex", flexDirection: "column", gap: "1.25rem" }}>
      {/* Controls & Filter Bar */}
      <div style={{ ...nocPanelStyle, display: "flex", flexDirection: "column", gap: "1rem" }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", flexWrap: "wrap", gap: "0.75rem" }}>
          <div>
            <h3 style={{ margin: 0, color: NOC_SURFACES.textPrimary, fontSize: "1.05rem", fontWeight: 700 }}>
              Tool & Workspace Activity
            </h3>
            <p style={{ margin: "0.2rem 0 0", color: NOC_SURFACES.textMuted, fontSize: "0.82rem" }}>
              Track commands executed and files touched per task or work session.
            </p>
          </div>
          <button
            type="button"
            className="btn-secondary noc-focusable"
            onClick={() => void fetchActivity()}
            disabled={loading}
            style={{ fontSize: "0.8rem", padding: "0.35rem 0.8rem" }}
          >
            {loading ? "Refreshing…" : `Refresh${lastRefreshed ? ` · ${lastRefreshed}` : ""}`}
          </button>
        </div>

        <div style={{ display: "flex", flexWrap: "wrap", gap: "0.75rem", alignItems: "center" }}>
          <div style={{ display: "flex", flexDirection: "column", gap: "0.25rem" }}>
            <label htmlFor="activity-agent-select" style={{ fontSize: "0.74rem", color: NOC_SURFACES.textMuted }}>Agent Filter</label>
            <select
              id="activity-agent-select"
              className="noc-focusable"
              value={selectedAgent}
              onChange={(e) => setSelectedAgent(e.target.value)}
              style={selectStyle}
            >
              <option value="">All Agents</option>
              {agents.map((a) => (
                <option key={a.id} value={a.id}>
                  {a.display_name}
                </option>
              ))}
            </select>
          </div>

          <div style={{ display: "flex", flexDirection: "column", gap: "0.25rem" }}>
            <label htmlFor="activity-time-select" style={{ fontSize: "0.74rem", color: NOC_SURFACES.textMuted }}>Time Range</label>
            <select
              id="activity-time-select"
              className="noc-focusable"
              value={selectedTimeRange}
              onChange={(e) => setSelectedTimeRange(e.target.value)}
              style={selectStyle}
            >
              <option value="all">All Time</option>
              <option value="1h">Past Hour</option>
              <option value="24h">Past 24 Hours</option>
              <option value="7d">Past 7 Days</option>
              <option value="30d">Past 30 Days</option>
            </select>
          </div>

          <div style={{ display: "flex", flexDirection: "column", gap: "0.25rem" }}>
            <label htmlFor="activity-kind-select" style={{ fontSize: "0.74rem", color: NOC_SURFACES.textMuted }}>Scope</label>
            <select
              id="activity-kind-select"
              className="noc-focusable"
              value={selectedKind}
              onChange={(e) => setSelectedKind(e.target.value)}
              style={selectStyle}
            >
              <option value="all">All Scopes</option>
              <option value="task">Tasks</option>
              <option value="work_session">Work Sessions</option>
            </select>
          </div>

          <div style={{ display: "flex", flexDirection: "column", gap: "0.25rem", flex: 1, minWidth: "160px" }}>
            <label htmlFor="activity-search-input" style={{ fontSize: "0.74rem", color: NOC_SURFACES.textMuted }}>Filter text</label>
            <input
              id="activity-search-input"
              className="noc-focusable"
              type="text"
              placeholder="Search title, file, command, agent…"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              style={selectStyle}
            />
          </div>
        </div>

        {/* Telemetry quick counters */}
        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(130px, 1fr))", gap: "0.6rem", marginTop: "0.25rem" }}>
          {[
            { label: "Tasks & Sessions", val: summary.totalItems, color: NOC_COLORS.cyan },
            { label: "Files Touched", val: summary.totalFiles, color: NOC_COLORS.indigo },
            { label: "Commands Run", val: summary.totalCommands, color: NOC_COLORS.warn },
            { label: "Agents Involved", val: summary.totalAgents, color: NOC_COLORS.ok },
          ].map((c) => (
            <div key={c.label} style={{ ...nocCardDenseStyle, padding: "0.5rem 0.8rem" }}>
              <span style={{ fontSize: "0.72rem", color: NOC_SURFACES.textMuted, textTransform: "uppercase" }}>{c.label}</span>
              <div className="tabular-nums" style={{ fontSize: "1.3rem", fontWeight: 700, color: c.color, marginTop: "0.15rem" }}>
                {c.val}
              </div>
            </div>
          ))}
        </div>
      </div>

      {error && (
        <div style={{ ...nocPanelStyle, color: NOC_COLORS.crit, background: "rgba(239, 68, 68, 0.1)" }}>
          Error loading activity: {error}
        </div>
      )}

      {/* Activity Item List */}
      <div style={{ display: "flex", flexDirection: "column", gap: "0.75rem" }}>
        {filteredItems.map((item) => {
          const isExpanded = expandedDetails[item.id] ?? false;
          const kindBadgeColor =
            item.kind === "task"
              ? "rgba(56, 189, 248, 0.2)"
              : "rgba(168, 85, 247, 0.2)";

          return (
            <div key={item.id} style={{ ...nocCardDenseStyle, display: "flex", flexDirection: "column", gap: "0.75rem" }}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start", flexWrap: "wrap", gap: "0.5rem" }}>
                <div style={{ display: "flex", alignItems: "center", gap: "0.5rem", flexWrap: "wrap" }}>
                  <span style={badgeStyle(kindBadgeColor, NOC_COLORS.cyan)}>
                    {item.kind.replace("_", " ")}
                  </span>
                  {item.status && (
                    <span
                      style={badgeStyle(
                        item.status === "done"
                          ? "rgba(16, 185, 129, 0.15)"
                          : item.status === "failed"
                          ? "rgba(239, 68, 68, 0.15)"
                          : "rgba(234, 179, 8, 0.15)",
                        item.status === "done"
                          ? NOC_COLORS.ok
                          : item.status === "failed"
                          ? NOC_COLORS.crit
                          : NOC_COLORS.warn,
                      )}
                    >
                      {item.status}
                    </span>
                  )}
                  <strong style={{ color: NOC_SURFACES.textPrimary, fontSize: "0.98rem" }}>{item.title}</strong>
                </div>

                <div className="tabular-nums" style={{ fontSize: "0.75rem", color: NOC_SURFACES.textMuted, fontFamily: "var(--font-mono)" }}>
                  {new Date(item.updated_at).toLocaleString()}
                </div>
              </div>

              {item.workspace_path && (
                <div style={{ fontSize: "0.76rem", color: NOC_SURFACES.textMuted }}>
                  Workspace: <code style={{ background: "rgba(0,0,0,0.3)", padding: "0.15rem 0.4rem", borderRadius: "4px" }}>{item.workspace_path}</code>
                </div>
              )}

              {/* Agents Involved */}
              <div style={{ display: "flex", alignItems: "center", gap: "0.5rem", flexWrap: "wrap" }}>
                <span style={{ fontSize: "0.75rem", color: NOC_SURFACES.textMuted }}>Agents:</span>
                {item.agents.length > 0 ? (
                  item.agents.map((agentId) => {
                    const dName = agentNameMap.get(agentId) || agentId;
                    return (
                      <span
                        key={agentId}
                        style={{
                          display: "inline-flex",
                          alignItems: "center",
                          gap: "0.35rem",
                          background: "rgba(255,255,255,0.06)",
                          padding: "0.15rem 0.45rem",
                          borderRadius: "14px",
                          fontSize: "0.74rem",
                          color: NOC_SURFACES.textPrimary,
                        }}
                      >
                        <AgentAvatar agentId={agentId} displayName={dName} size={15} />
                        {dName}
                      </span>
                    );
                  })
                ) : (
                  <span style={{ fontSize: "0.74rem", color: NOC_SURFACES.textMuted, fontStyle: "italic" }}>
                    None recorded
                  </span>
                )}
              </div>

              {/* Summary Counts & Toggle Details */}
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", borderTop: "1px solid rgba(148, 163, 184, 0.12)", paddingTop: "0.5rem", flexWrap: "wrap", gap: "0.5rem" }}>
                <div style={{ display: "flex", gap: "0.85rem", fontSize: "0.78rem", color: NOC_SURFACES.textMuted }}>
                  <span>Files: <strong style={{ color: NOC_SURFACES.textPrimary }}>{item.files.length}</strong></span>
                  <span>Commands: <strong style={{ color: NOC_SURFACES.textPrimary }}>{item.commands.length}</strong></span>
                  {item.message_count > 0 && <span>Messages: <strong style={{ color: NOC_SURFACES.textPrimary }}>{item.message_count}</strong></span>}
                </div>

                <button
                  type="button"
                  className="btn-secondary noc-focusable"
                  onClick={() => toggleDetails(item.id)}
                  style={{ fontSize: "0.74rem", padding: "0.2rem 0.6rem", marginTop: 0 }}
                >
                  {isExpanded ? "Collapse Details ▲" : "View Files & Commands ▼"}
                </button>
              </div>

              {/* Collapsible Details */}
              {isExpanded && (
                <div style={{ display: "flex", flexDirection: "column", gap: "0.75rem", marginTop: "0.25rem", borderTop: "1px dashed rgba(148, 163, 184, 0.2)", paddingTop: "0.6rem" }}>
                  {/* Commands */}
                  <div>
                    <h5 style={{ margin: "0 0 0.35rem", fontSize: "0.8rem", color: NOC_SURFACES.textPrimary }}>Commands Run ({item.commands.length})</h5>
                    {item.commands.length > 0 ? (
                      <div style={{ display: "flex", flexDirection: "column", gap: "0.35rem" }}>
                        {item.commands.map((cmd, idx) => (
                          <div
                            key={idx}
                            style={{
                              background: "rgba(2, 6, 23, 0.6)",
                              borderRadius: "6px",
                              padding: "0.35rem 0.55rem",
                              fontFamily: "var(--font-mono)",
                              fontSize: "0.76rem",
                              color: "#e2e8f0",
                              display: "flex",
                              justifyContent: "space-between",
                              alignItems: "center",
                              flexWrap: "wrap",
                              gap: "0.4rem",
                            }}
                          >
                            <span>$ {cmd.raw}</span>
                            <div style={{ display: "flex", gap: "0.4rem", fontSize: "0.7rem" }}>
                              {cmd.attribution && (
                                <span style={{ color: NOC_COLORS.cyan, background: "rgba(56, 189, 248, 0.12)", padding: "0.1rem 0.3rem", borderRadius: "4px" }}>
                                  {cmd.attribution}
                                </span>
                              )}
                              <span className="tabular-nums" style={{ color: NOC_SURFACES.textMuted }}>{new Date(cmd.observed_at).toLocaleTimeString()}</span>
                            </div>
                          </div>
                        ))}
                      </div>
                    ) : (
                      <span style={{ fontSize: "0.74rem", color: NOC_SURFACES.textMuted, fontStyle: "italic" }}>No commands captured for this task.</span>
                    )}
                  </div>

                  {/* Files Touched */}
                  <div>
                    <h5 style={{ margin: "0 0 0.35rem", fontSize: "0.8rem", color: NOC_SURFACES.textPrimary }}>Files Touched ({item.files.length})</h5>
                    {item.files.length > 0 ? (
                      <div style={{ display: "flex", flexDirection: "column", gap: "0.3rem" }}>
                        {item.files.map((file, idx) => {
                          const op = operationColor[file.operation] || { bg: "rgba(255,255,255,0.1)", text: "#fff" };
                          return (
                            <div
                              key={idx}
                              style={{
                                display: "flex",
                                justifyContent: "space-between",
                                alignItems: "center",
                                fontSize: "0.76rem",
                                background: "rgba(0,0,0,0.25)",
                                padding: "0.25rem 0.45rem",
                                borderRadius: "4px",
                                flexWrap: "wrap",
                                gap: "0.4rem",
                              }}
                            >
                              <div style={{ display: "flex", alignItems: "center", gap: "0.45rem" }}>
                                <span style={{ ...badgeStyle(op.bg, op.text), fontSize: "0.66rem", padding: "0.1rem 0.35rem" }}>
                                  {file.operation}
                                </span>
                                <code style={{ color: NOC_SURFACES.textPrimary }}>{file.path}</code>
                              </div>
                              <span className="tabular-nums" style={{ fontSize: "0.7rem", color: NOC_SURFACES.textMuted }}>
                                {new Date(file.observed_at).toLocaleTimeString()}
                              </span>
                            </div>
                          );
                        })}
                      </div>
                    ) : (
                      <span style={{ fontSize: "0.74rem", color: NOC_SURFACES.textMuted, fontStyle: "italic" }}>No files touched recorded for this task.</span>
                    )}
                  </div>
                </div>
              )}
            </div>
          );
        })}

        {filteredItems.length === 0 && !loading && (
          <EmptyStateChip
            title="No activity recorded"
            detail="No tool or workspace activities found matching the selected criteria."
            tone="idle"
          />
        )}
      </div>
    </div>
  );
}
