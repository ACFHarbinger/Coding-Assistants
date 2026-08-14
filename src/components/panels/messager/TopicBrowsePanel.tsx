// @ts-nocheck
import { useState } from "react";
import { invoke } from "../../../lib/tauri";
import type { MemoryRecord } from "./types";

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
    <div className="glass-card" style={{
      padding: "1.25rem 1rem",
      display: "flex",
      flexDirection: "column",
      gap: "1rem",
      overflow: "hidden"
    }}>
      <div>
        <h3 style={{ margin: 0, fontSize: "1rem", fontWeight: 700, color: "var(--primary)" }}>
          🔎 Topic Browse
        </h3>
        <p style={{ margin: "0.35rem 0 0", fontSize: "0.8rem", color: "var(--text-muted)" }}>
          Compare what each agent remembers about the same topic.
        </p>
      </div>

      <form onSubmit={(event) => { event.preventDefault(); search(); }} style={{ display: "flex", gap: "0.5rem" }}>
        <input
          type="text"
          placeholder="Search a topic..."
          value={query}
          onChange={event => setQuery(event.target.value)}
          style={{
            flex: 1,
            minWidth: 0,
            padding: "0.5rem 0.75rem",
            borderRadius: "8px",
            background: "rgba(0,0,0,0.4)",
            border: "1px solid var(--border-color)",
            color: "var(--text-main)",
            fontSize: "0.85rem",
            outline: "none"
          }}
        />
        <button
          type="submit"
          disabled={!query.trim() || loading}
          style={{
            padding: "0.5rem 0.8rem",
            borderRadius: "8px",
            border: "none",
            background: "var(--primary)",
            color: "#fff",
            fontSize: "0.8rem",
            fontWeight: 600,
            cursor: loading ? "wait" : "pointer",
            opacity: !query.trim() || loading ? 0.6 : 1
          }}
        >
          {loading ? "Searching..." : "Search"}
        </button>
      </form>

      {error ? (
        <p style={{ margin: 0, fontSize: "0.85rem", color: "var(--accent)", textAlign: "center" }}>{error}</p>
      ) : results === null ? (
        <p style={{ margin: "1rem 0", fontSize: "0.85rem", color: "var(--text-muted)", textAlign: "center" }}>
          Search for a topic to see what each agent has written about it.
        </p>
      ) : groups.length === 0 ? (
        <p style={{ margin: "1rem 0", fontSize: "0.85rem", color: "var(--text-muted)", textAlign: "center" }}>
          No memories found for that topic.
        </p>
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
              minWidth: 0,
              background: "rgba(0,0,0,0.2)",
              border: "1px solid var(--border-color)",
              borderRadius: "10px",
              padding: "0.75rem",
              display: "flex",
              flexDirection: "column",
              gap: "0.6rem"
            }}>
              <div style={{ color: "var(--accent)", fontWeight: 700, fontSize: "0.85rem" }}>
                {formatAgentName(agentId)}
              </div>
              {memories.map(memory => (
                <div key={memory.id} style={{
                  background: "rgba(0,0,0,0.3)",
                  border: "1px solid var(--border-color)",
                  borderRadius: "8px",
                  padding: "0.65rem",
                  display: "flex",
                  flexDirection: "column",
                  gap: "0.4rem"
                }}>
                  <div style={{ display: "flex", alignItems: "flex-start", justifyContent: "space-between", gap: "0.5rem" }}>
                    <div style={{ color: "var(--text-main)", fontWeight: 600, fontSize: "0.85rem" }}>
                      {memory.title || `Memory #${memory.id.slice(0, 8)}`}
                    </div>
                    <span style={{
                      flexShrink: 0,
                      color: "var(--accent)",
                      background: "rgba(168, 85, 247, 0.15)",
                      borderRadius: "4px",
                      padding: "0.1rem 0.4rem",
                      fontSize: "0.7rem",
                      fontWeight: 700
                    }}>
                      {memory.tier}
                    </span>
                  </div>
                  <div style={{
                    color: "var(--text-muted)",
                    fontSize: "0.78rem",
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
