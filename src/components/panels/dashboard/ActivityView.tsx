import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "../../../lib/tauri";
import { AgentAvatar } from "../messager/AgentAvatar";
import type { ActivityFilter, ActivityItem } from "./types";

interface AgentRecord {
  id: string;
  display_name: string;
  avatar_attachment_id?: string | null;
}

const cardStyle: React.CSSProperties = {
  border: "1px solid var(--border-color)",
  borderRadius: "12px",
  padding: "1.25rem",
  background: "rgba(0, 0, 0, 0.3)",
};

const badgeStyle = (bg: string, color = "#fff"): React.CSSProperties => ({
  fontSize: "0.72rem",
  padding: "0.2rem 0.55rem",
  borderRadius: "6px",
  background: bg,
  color,
  fontWeight: 600,
  letterSpacing: "0.03em",
  textTransform: "uppercase",
  display: "inline-flex",
  alignItems: "center",
  gap: "0.3rem",
});

const operationColor: Record<string, string> = {
  created: "rgba(34, 197, 94, 0.2)",
  modified: "rgba(59, 130, 246, 0.2)",
  removed: "rgba(239, 68, 68, 0.2)",
  accessed: "rgba(168, 85, 247, 0.2)",
};

const operationTextColor: Record<string, string> = {
  created: "#4ade80",
  modified: "#60a5fa",
  removed: "#f87171",
  accessed: "#c084fc",
};

export default function ActivityView({
  agents,
}: {
  agents: AgentRecord[];
}) {
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
    for (const a of agents) {
      map.set(a.id, a.display_name);
    }
    return map;
  }, [agents]);

  return (
    <div className="fade-in" style={{ display: "flex", flexDirection: "column", gap: "1.25rem" }}>
      {/* Controls & Filter Bar */}
      <div style={{ ...cardStyle, display: "flex", flexDirection: "column", gap: "1rem" }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", flexWrap: "wrap", gap: "0.75rem" }}>
          <div>
            <h3 style={{ margin: 0, color: "var(--text-main)", fontSize: "1.1rem" }}>
              Tool & Workspace Activity
            </h3>
            <p style={{ margin: "0.25rem 0 0", color: "var(--text-muted)", fontSize: "0.82rem" }}>
              Track agents, commands run, and files touched per task or work session (Roadmap D4).
            </p>
          </div>
          <button
            type="button"
            className="btn-secondary"
            onClick={() => void fetchActivity()}
            disabled={loading}
            style={{ fontSize: "0.8rem", padding: "0.35rem 0.8rem" }}
          >
            {loading ? "Refreshing…" : `Refresh${lastRefreshed ? ` · ${lastRefreshed}` : ""}`}
          </button>
        </div>

        <div style={{ display: "flex", flexWrap: "wrap", gap: "0.75rem", alignItems: "center" }}>
          <div style={{ display: "flex", flexDirection: "column", gap: "0.25rem" }}>
            <label htmlFor="activity-agent-select" style={{ fontSize: "0.75rem", color: "var(--text-muted)" }}>Agent Filter</label>
            <select
              id="activity-agent-select"
              value={selectedAgent}
              onChange={(e) => setSelectedAgent(e.target.value)}
              style={{
                background: "rgba(0,0,0,0.4)",
                color: "var(--text-main)",
                border: "1px solid var(--border-color)",
                borderRadius: "8px",
                padding: "0.35rem 0.75rem",
                fontSize: "0.82rem",
              }}
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
            <label htmlFor="activity-time-select" style={{ fontSize: "0.75rem", color: "var(--text-muted)" }}>Time Range</label>
            <select
              id="activity-time-select"
              value={selectedTimeRange}
              onChange={(e) => setSelectedTimeRange(e.target.value)}
              style={{
                background: "rgba(0,0,0,0.4)",
                color: "var(--text-main)",
                border: "1px solid var(--border-color)",
                borderRadius: "8px",
                padding: "0.35rem 0.75rem",
                fontSize: "0.82rem",
              }}
            >
              <option value="all">All Time</option>
              <option value="1h">Last 1 Hour</option>
              <option value="24h">Last 24 Hours</option>
              <option value="7d">Last 7 Days</option>
              <option value="30d">Last 30 Days</option>
            </select>
          </div>

          <div style={{ display: "flex", flexDirection: "column", gap: "0.25rem" }}>
            <label htmlFor="activity-scope-select" style={{ fontSize: "0.75rem", color: "var(--text-muted)" }}>Scope</label>
            <select
              id="activity-scope-select"
              value={selectedKind}
              onChange={(e) => setSelectedKind(e.target.value)}
              style={{
                background: "rgba(0,0,0,0.4)",
                color: "var(--text-main)",
                border: "1px solid var(--border-color)",
                borderRadius: "8px",
                padding: "0.35rem 0.75rem",
                fontSize: "0.82rem",
              }}
            >
              <option value="all">All Scopes</option>
              <option value="task">Tasks</option>
              <option value="work_session">Work Sessions</option>
            </select>
          </div>

          <div style={{ display: "flex", flexDirection: "column", gap: "0.25rem", flex: 1, minWidth: "160px" }}>
            <label htmlFor="activity-search-input" style={{ fontSize: "0.75rem", color: "var(--text-muted)" }}>Filter text</label>
            <input
              id="activity-search-input"
              type="text"
              placeholder="Search title, file, command, agent…"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              style={{
                background: "rgba(0,0,0,0.4)",
                color: "var(--text-main)",
                border: "1px solid var(--border-color)",
                borderRadius: "8px",
                padding: "0.35rem 0.75rem",
                fontSize: "0.82rem",
              }}
            />
          </div>
        </div>

        {/* Telemetry quick counters */}
        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(130px, 1fr))", gap: "0.6rem", marginTop: "0.25rem" }}>
          <div style={{ background: "rgba(255,255,255,0.03)", padding: "0.5rem 0.8rem", borderRadius: "8px", border: "1px solid rgba(255,255,255,0.05)" }}>
            <span style={{ fontSize: "0.72rem", color: "var(--text-muted)", textTransform: "uppercase" }}>Tasks & Sessions</span>
            <div style={{ fontSize: "1.3rem", fontWeight: 700, color: "var(--primary)" }}>{summary.totalItems}</div>
          </div>
          <div style={{ background: "rgba(255,255,255,0.03)", padding: "0.5rem 0.8rem", borderRadius: "8px", border: "1px solid rgba(255,255,255,0.05)" }}>
            <span style={{ fontSize: "0.72rem", color: "var(--text-muted)", textTransform: "uppercase" }}>Files Touched</span>
            <div style={{ fontSize: "1.3rem", fontWeight: 700, color: "#38bdf8" }}>{summary.totalFiles}</div>
          </div>
          <div style={{ background: "rgba(255,255,255,0.03)", padding: "0.5rem 0.8rem", borderRadius: "8px", border: "1px solid rgba(255,255,255,0.05)" }}>
            <span style={{ fontSize: "0.72rem", color: "var(--text-muted)", textTransform: "uppercase" }}>Commands Run</span>
            <div style={{ fontSize: "1.3rem", fontWeight: 700, color: "#facc15" }}>{summary.totalCommands}</div>
          </div>
          <div style={{ background: "rgba(255,255,255,0.03)", padding: "0.5rem 0.8rem", borderRadius: "8px", border: "1px solid rgba(255,255,255,0.05)" }}>
            <span style={{ fontSize: "0.72rem", color: "var(--text-muted)", textTransform: "uppercase" }}>Agents Involved</span>
            <div style={{ fontSize: "1.3rem", fontWeight: 700, color: "#4ade80" }}>{summary.totalAgents}</div>
          </div>
        </div>
      </div>

      {error && <div style={{ ...cardStyle, color: "#ef4444" }}>Error loading activity: {error}</div>}

      {/* Activity Item List */}
      <div style={{ display: "flex", flexDirection: "column", gap: "1rem" }}>
        {filteredItems.map((item) => {
          const isExpanded = expandedDetails[item.id] ?? false;
          const kindBadgeColor =
            item.kind === "task"
              ? "rgba(59, 130, 246, 0.25)"
              : item.kind === "work_session"
              ? "rgba(168, 85, 247, 0.25)"
              : "rgba(107, 114, 128, 0.25)";

          return (
            <div key={item.id} style={{ ...cardStyle, display: "flex", flexDirection: "column", gap: "0.85rem" }}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start", flexWrap: "wrap", gap: "0.5rem" }}>
                <div style={{ display: "flex", alignItems: "center", gap: "0.5rem", flexWrap: "wrap" }}>
                  <span style={badgeStyle(kindBadgeColor, "#93c5fd")}>
                    {item.kind.replace("_", " ")}
                  </span>
                  {item.status && (
                    <span
                      style={badgeStyle(
                        item.status === "done"
                          ? "rgba(34, 197, 94, 0.2)"
                          : item.status === "failed"
                          ? "rgba(239, 68, 68, 0.2)"
                          : "rgba(234, 179, 8, 0.2)",
                        item.status === "done"
                          ? "#4ade80"
                          : item.status === "failed"
                          ? "#f87171"
                          : "#fde047",
                      )}
                    >
                      {item.status}
                    </span>
                  )}
                  <strong style={{ color: "var(--text-main)", fontSize: "1.05rem" }}>{item.title}</strong>
                </div>

                <div style={{ fontSize: "0.75rem", color: "var(--text-muted)" }}>
                  Updated: {new Date(item.updated_at).toLocaleString()}
                </div>
              </div>

              {item.workspace_path && (
                <div style={{ fontSize: "0.78rem", color: "var(--text-muted)" }}>
                  Workspace: <code style={{ background: "rgba(0,0,0,0.3)", padding: "0.15rem 0.4rem", borderRadius: "4px" }}>{item.workspace_path}</code>
                </div>
              )}

              {/* Agents Involved */}
              <div style={{ display: "flex", alignItems: "center", gap: "0.5rem", flexWrap: "wrap" }}>
                <span style={{ fontSize: "0.75rem", color: "var(--text-muted)" }}>Agents:</span>
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
                          padding: "0.2rem 0.5rem",
                          borderRadius: "16px",
                          fontSize: "0.75rem",
                          color: "var(--text-main)",
                        }}
                      >
                        <AgentAvatar agentId={agentId} displayName={dName} size={16} />
                        {dName}
                      </span>
                    );
                  })
                ) : (
                  <span style={{ fontSize: "0.75rem", color: "var(--text-muted)", fontStyle: "italic" }}>
                    None recorded
                  </span>
                )}
              </div>

              {/* Summary Counts & Toggle Details */}
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", borderTop: "1px solid rgba(255,255,255,0.05)", paddingTop: "0.6rem", flexWrap: "wrap", gap: "0.5rem" }}>
                <div style={{ display: "flex", gap: "1rem", fontSize: "0.78rem", color: "var(--text-muted)" }}>
                  <span>Files: <strong style={{ color: "var(--text-main)" }}>{item.files.length}</strong></span>
                  <span>Commands: <strong style={{ color: "var(--text-main)" }}>{item.commands.length}</strong></span>
                  {item.message_count > 0 && <span>Messages: <strong style={{ color: "var(--text-main)" }}>{item.message_count}</strong></span>}
                  {item.capture_count > 0 && <span>Captures: <strong style={{ color: "var(--text-main)" }}>{item.capture_count}</strong></span>}
                </div>

                <button
                  type="button"
                  className="btn-secondary"
                  onClick={() => toggleDetails(item.id)}
                  style={{ fontSize: "0.74rem", padding: "0.2rem 0.6rem", marginTop: 0 }}
                >
                  {isExpanded ? "Collapse Details ▲" : "View Files & Commands ▼"}
                </button>
              </div>

              {/* Collapsible Details */}
              {isExpanded && (
                <div style={{ display: "flex", flexDirection: "column", gap: "0.75rem", marginTop: "0.25rem", borderTop: "1px dashed var(--border-color)", paddingTop: "0.75rem" }}>
                  {/* Commands */}
                  <div>
                    <h5 style={{ margin: "0 0 0.4rem", fontSize: "0.82rem", color: "var(--text-main)" }}>Commands Run ({item.commands.length})</h5>
                    {item.commands.length > 0 ? (
                      <div style={{ display: "flex", flexDirection: "column", gap: "0.4rem" }}>
                        {item.commands.map((cmd, idx) => (
                          <div
                            key={idx}
                            style={{
                              background: "rgba(0,0,0,0.5)",
                              borderRadius: "6px",
                              padding: "0.4rem 0.6rem",
                              fontFamily: "monospace",
                              fontSize: "0.78rem",
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
                                <span style={{ color: "var(--primary)", background: "rgba(255,255,255,0.05)", padding: "0.1rem 0.3rem", borderRadius: "4px" }}>
                                  {cmd.attribution}
                                </span>
                              )}
                              <span style={{ color: "var(--text-muted)" }}>{new Date(cmd.observed_at).toLocaleTimeString()}</span>
                            </div>
                          </div>
                        ))}
                      </div>
                    ) : (
                      <span style={{ fontSize: "0.75rem", color: "var(--text-muted)", fontStyle: "italic" }}>No commands captured for this task.</span>
                    )}
                  </div>

                  {/* Files Touched */}
                  <div>
                    <h5 style={{ margin: "0 0 0.4rem", fontSize: "0.82rem", color: "var(--text-main)" }}>Files Touched ({item.files.length})</h5>
                    {item.files.length > 0 ? (
                      <div style={{ display: "flex", flexDirection: "column", gap: "0.35rem" }}>
                        {item.files.map((file, idx) => {
                          const bg = operationColor[file.operation] || "rgba(255,255,255,0.1)";
                          const tc = operationTextColor[file.operation] || "#fff";
                          return (
                            <div
                              key={idx}
                              style={{
                                display: "flex",
                                justifyContent: "space-between",
                                alignItems: "center",
                                fontSize: "0.78rem",
                                background: "rgba(0,0,0,0.2)",
                                padding: "0.3rem 0.5rem",
                                borderRadius: "4px",
                                flexWrap: "wrap",
                                gap: "0.4rem",
                              }}
                            >
                              <div style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
                                <span style={{ ...badgeStyle(bg, tc), fontSize: "0.68rem", padding: "0.1rem 0.4rem" }}>
                                  {file.operation}
                                </span>
                                <code style={{ color: "var(--text-main)" }}>{file.path}</code>
                              </div>
                              <span style={{ fontSize: "0.7rem", color: "var(--text-muted)" }}>
                                {new Date(file.observed_at).toLocaleTimeString()}
                              </span>
                            </div>
                          );
                        })}
                      </div>
                    ) : (
                      <span style={{ fontSize: "0.75rem", color: "var(--text-muted)", fontStyle: "italic" }}>No files touched recorded for this task.</span>
                    )}
                  </div>
                </div>
              )}
            </div>
          );
        })}

        {filteredItems.length === 0 && !loading && (
          <div style={{ ...cardStyle, textAlign: "center", color: "var(--text-muted)", padding: "2rem" }}>
            No tool or workspace activities found matching the selected criteria.
          </div>
        )}
      </div>
    </div>
  );
}
