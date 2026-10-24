// @ts-nocheck
import { useState } from "react";
import { invoke } from "../../../lib/tauri";
import type { MemoryRecord } from "./types";
import { nocCardDenseStyle, nocPanelStyle } from "../../common/telemetry/panel";
import { EmptyStateChip } from "../../common/telemetry/status";
import { NOC_COLORS, NOC_SURFACES } from "../../common/telemetry/tokens";

export default function TopicBrowsePanel() {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<Record<string, MemoryRecord[]> | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  const search = async () => {
    const trimmedQuery = query.trim();
    if (!trimmedQuery || loading) return;

    setLoading(true);
    setError("");
    try {
      setResults(await invoke<Record<string, MemoryRecord[]>>("hub_memories_for_topic", { query: trimmedQuery }));
    } catch (err) {
      setError(`Unable to search memories: ${err}`);
      setResults(null);
    } finally {
      setLoading(false);
    }
  };

  const groups = Object.entries(results || {}).filter(([, memories]) => memories.length > 0);
  const formatAgentName = (agentId) => agentId === "unattributed"
    ? "Unattributed"
    : agentId.charAt(0).toUpperCase() + agentId.slice(1);

  return (
    <div style={{
      ...nocPanelStyle,
      padding: "1.1rem 1rem",
      display: "flex",
      flexDirection: "column",
      gap: "0.9rem",
      overflow: "hidden"
    }}>
      <div>
        <h3 style={{ margin: 0, fontSize: "0.98rem", fontWeight: 700, color: NOC_COLORS.cyan }}>
          🔎 Topic Browse
        </h3>
        <p style={{ margin: "0.25rem 0 0", fontSize: "0.78rem", color: NOC_SURFACES.textMuted }}>
          Compare what each agent remembers about the same topic.
        </p>
      </div>

      <form onSubmit={(event) => { event.preventDefault(); search(); }} style={{ display: "flex", gap: "0.5rem" }}>
        <input
          type="text"
          className="noc-focusable"
          placeholder="Search a topic..."
          value={query}
          onChange={event => setQuery(event.target.value)}
          style={{
            flex: 1,
            minWidth: 0,
            padding: "0.45rem 0.75rem",
            borderRadius: "8px",
            background: "rgba(9, 14, 26, 0.78)",
            border: "1px solid rgba(148, 163, 184, 0.22)",
            color: NOC_SURFACES.textPrimary,
            fontSize: "0.82rem",
            outline: "none"
          }}
        />
        <button
          type="submit"
          className="btn-primary noc-focusable"
          disabled={!query.trim() || loading}
          style={{
            marginTop: 0,
            padding: "0.45rem 0.85rem",
            fontSize: "0.8rem",
            cursor: loading ? "wait" : "pointer",
            opacity: !query.trim() || loading ? 0.6 : 1
          }}
        >
          {loading ? "Searching..." : "Search"}
        </button>
      </form>

      {error ? (
        <p style={{ margin: 0, fontSize: "0.82rem", color: NOC_COLORS.crit, textAlign: "center" }}>{error}</p>
      ) : results === null ? (
        <EmptyStateChip
          title="Search for a topic"
          detail="See what each agent has written about it across memory tiers."
          tone="idle"
        />
      ) : groups.length === 0 ? (
        <EmptyStateChip
          title="No memories found for that topic"
          detail="Try a broader keyword or another concept."
          tone="idle"
        />
      ) : (
        <div style={{
          display: "grid",
          gridTemplateColumns: "repeat(auto-fit, minmax(220px, 1fr))",
          gap: "0.75rem",
          overflowY: "auto",
          paddingBottom: "0.25rem"
        }}>
          {groups.map(([agentId, memories]) => (
            <section key={agentId} style={{
              ...nocCardDenseStyle,
              minWidth: 0,
              borderLeft: `3px solid ${NOC_COLORS.cyan}`,
              padding: "0.75rem",
              display: "flex",
              flexDirection: "column",
              gap: "0.55rem"
            }}>
              <div style={{ color: NOC_COLORS.cyan, fontWeight: 700, fontSize: "0.84rem" }}>
                {formatAgentName(agentId)}
              </div>
              {memories.map(memory => (
                <div key={memory.id} style={{
                  background: "rgba(2, 6, 23, 0.55)",
                  border: "1px solid rgba(148, 163, 184, 0.14)",
                  borderRadius: "8px",
                  padding: "0.6rem",
                  display: "flex",
                  flexDirection: "column",
                  gap: "0.35rem"
                }}>
                  <div style={{ display: "flex", alignItems: "flex-start", justifyContent: "space-between", gap: "0.5rem" }}>
                    <div style={{ color: NOC_SURFACES.textPrimary, fontWeight: 600, fontSize: "0.83rem" }}>
                      {memory.title || `Memory #${memory.id.slice(0, 8)}`}
                    </div>
                    <span style={{
                      flexShrink: 0,
                      color: NOC_COLORS.purple,
                      background: "rgba(168, 85, 247, 0.15)",
                      border: "1px solid rgba(168, 85, 247, 0.3)",
                      borderRadius: "4px",
                      padding: "0.08rem 0.4rem",
                      fontSize: "0.68rem",
                      fontWeight: 700
                    }}>
                      {memory.tier}
                    </span>
                  </div>
                  <div style={{
                    color: NOC_SURFACES.textMuted,
                    fontSize: "0.76rem",
                    display: "-webkit-box",
                    WebkitLineClamp: 3,
                    WebkitBoxOrient: "vertical",
                    overflow: "hidden"
                  }}>
                    {memory.body}
                  </div>
                </div>
              ))}
            </section>
          ))}
        </div>
      )}
    </div>
  );
}
