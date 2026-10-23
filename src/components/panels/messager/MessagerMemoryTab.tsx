import { useCallback, useEffect, useState } from "react";
import { invoke } from "../../../lib/tauri";
import { cardStyle, inputStyle } from "../hub/HubCharts";
import { consolidateMemories } from "../hub/memoryApi";
import type { AgentRecord } from "../hub/types";
import MemoryLinksSection from "./MemoryLinksSection";
import TopicBrowsePanel from "./TopicBrowsePanel";
import type { MemoryRecord, ScoredMemoryRecord } from "./types";

type AnyMemory = MemoryRecord | ScoredMemoryRecord;

function formatScore(score: number) {
  if (score >= 1.0) return "100%";
  if (score <= 0.0) return "0%";
  return `${Math.round(score * 100)}%`;
}

export default function MessagerMemoryTab({
  workspacePath,
  initialSearchQuery,
}: {
  workspacePath: string;
  initialSearchQuery?: string;
}) {
  const [memories, setMemories] = useState<AnyMemory[]>([]);
  const [agents, setAgents] = useState<AgentRecord[]>([]);
  const [error, setError] = useState<string>("");
  const [status, setStatus] = useState<string>("");

  const [browseMode, setBrowseMode] = useState<"tier" | "topic">("tier");
  const [query, setQuery] = useState<string>(initialSearchQuery ?? "");
  const [searchMode, setSearchMode] = useState<"smart" | "exact">("smart");
  const [tierFilter, setTierFilter] = useState<string>("");
  const [scopeFilter, setScopeFilter] = useState<string>("");
  const [expandedMemoryId, setExpandedMemoryId] = useState<string | null>(null);
  const [searching, setSearching] = useState(false);
  const [searchError, setSearchError] = useState<string | null>(null);
  const [consolidating, setConsolidating] = useState(false);
  const [consolidationStatus, setConsolidationStatus] = useState<string | null>(null);

  const [memTitle, setMemTitle] = useState("");
  const [memBody, setMemBody] = useState("");
  const [memTier, setMemTier] = useState("short_term");
  const [memScope, setMemScope] = useState("workspace");
  const [memAgent, setMemAgent] = useState("grok");

  const [editingMemory, setEditingMemory] = useState<string | null>(null);
  const [editTitle, setEditTitle] = useState("");
  const [editBody, setEditBody] = useState("");

  const run = useCallback(async <T,>(label: string, fn: () => Promise<T>, pendingMessage?: string): Promise<T | null> => {
    setError("");
    if (pendingMessage) setStatus(pendingMessage);
    try {
      const result = await fn();
      setStatus(label);
      return result;
    } catch (e) {
      setError(String(e));
      setStatus("");
      return null;
    }
  }, []);

  const refreshMemories = useCallback(async () => {
    const list = await run("memories refreshed", () =>
      invoke<MemoryRecord[]>("hub_list_memories", {
        scope: scopeFilter || null,
        tier: tierFilter || null,
        workspace: null,
        includeStale: false,
      })
    );
    if (list) setMemories(list);
  }, [run, tierFilter, scopeFilter]);

  const searchMemories = useCallback(async () => {
    if (!query.trim()) {
      await refreshMemories();
      return;
    }
    if (searchMode === "smart") {
      setSearching(true);
      setSearchError(null);
      try {
        const results = await invoke<ScoredMemoryRecord[]>("hub_search_memories_hybrid", {
          query,
          limit: 30,
          scope: scopeFilter || null,
          tier: tierFilter || null,
          workspace: scopeFilter === "workspace" ? (workspacePath || null) : null,
        });
        setMemories(results);
      } catch (err) {
        setSearchError(String(err));
      } finally {
        setSearching(false);
      }
    } else {
      const list = await run("exact search done", () =>
        invoke<MemoryRecord[]>("hub_search_memories", { query })
      );
      if (list) {
        let filtered = list;
        if (tierFilter) filtered = filtered.filter((m) => m.tier === tierFilter);
        if (scopeFilter) filtered = filtered.filter((m) => m.scope === scopeFilter);
        setMemories(filtered);
      }
    }
  }, [query, searchMode, tierFilter, scopeFilter, workspacePath, run, refreshMemories]);

  // Live smart search while typing, mirroring the old Memory Drawer's debounce.
  useEffect(() => {
    if (browseMode !== "tier" || searchMode !== "smart" || !query.trim()) return;
    const timer = setTimeout(() => {
      searchMemories();
    }, 200);
    return () => clearTimeout(timer);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [query, searchMode, tierFilter, scopeFilter, browseMode]);

  const writeMemory = async () => {
    if (!memBody.trim()) return;
    await run("memory written", () =>
      invoke("hub_write_memory", {
        args: {
          tier: memTier,
          scope: memScope,
          agent: memAgent || null,
          workspace: null,
          title: memTitle || null,
          body: memBody,
          tags: [],
        },
      })
    );
    setMemBody("");
    setMemTitle("");
    await refreshMemories();
  };

  const saveEditedMemory = async (id: string) => {
    if (!editBody.trim()) return;
    await run("memory updated", () =>
      invoke("hub_update_memory", {
        args: { id, title: editTitle || null, body: editBody, tags: null },
      })
    );
    setEditingMemory(null);
    await refreshMemories();
  };

  const reindexVectors = async () => {
    const count = await run(
      "vectors reindexed",
      () => invoke<number>("hub_reindex_memory_vectors"),
      "Re-indexing memory vector embeddings…"
    );
    if (count !== null) setStatus(`Re-indexed ${count} memory vector embedding(s)`);
  };

  const consolidate = async () => {
    setConsolidating(true);
    setConsolidationStatus("Consolidating…");
    try {
      const res = await consolidateMemories(workspacePath || null);
      setConsolidationStatus(`Consolidated ${res.consolidated} cluster(s)${res.notice ? ` (${res.notice})` : ""}`);
      await refreshMemories();
    } catch (e) {
      setConsolidationStatus(`Failed: ${String(e)}`);
    } finally {
      setConsolidating(false);
    }
  };

  // The Memory tab is full-width with no composer in view, so "attach to
  // message" (the old drawer's messageInput insert) becomes a clipboard
  // copy instead — a deliberate UX change, not a silent side effect.
  const copyMemoryReference = async (id: string) => {
    const reference = `[Memory #${id.slice(0, 8)}]`;
    try {
      await navigator.clipboard.writeText(reference);
      setStatus(`Copied ${reference} to clipboard`);
    } catch (e) {
      setError(String(e));
    }
  };

  useEffect(() => {
    invoke<AgentRecord[]>("hub_list_agents").then(setAgents).catch((e) => setError(String(e)));
    refreshMemories();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <div className="fade-in" style={{ display: "flex", flexDirection: "column", gap: "1.5rem", overflowY: "auto", padding: "0 0.25rem" }}>
      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", flexWrap: "wrap", gap: "0.5rem" }}>
        <h2 style={{ margin: 0, fontSize: "1.3rem", color: "var(--primary)" }}>🧠 Memory</h2>
        <div style={{ display: "flex", gap: "0.5rem", alignItems: "center" }}>
          {error && <span style={{ color: "#ef4444", fontSize: "0.85rem", background: "rgba(239, 68, 68, 0.1)", padding: "0.2rem 0.5rem", borderRadius: "4px" }}>Error: {error}</span>}
          {status && !error && <span style={{ color: "#22c55e", fontSize: "0.85rem", background: "rgba(34, 197, 94, 0.1)", padding: "0.2rem 0.5rem", borderRadius: "4px" }}>{status}</span>}
          <button className="btn-secondary" onClick={consolidate} disabled={consolidating} title="Cluster related short-term memories and summarize into episodic records">
            {consolidating ? "Consolidating…" : "Consolidate (M3)"}
          </button>
        </div>
      </div>

      {consolidationStatus && (
        <div style={{ fontSize: "0.8rem", color: consolidationStatus.startsWith("Failed") ? "#f87171" : "#4ade80", background: consolidationStatus.startsWith("Failed") ? "rgba(239, 68, 68, 0.1)" : "rgba(34, 197, 94, 0.1)", padding: "0.4rem 0.6rem", borderRadius: "6px" }}>
          {consolidationStatus}
        </div>
      )}

      <div style={{ display: "flex", gap: "0.5rem" }}>
        {(["tier", "topic"] as const).map((mode) => (
          <button
            key={mode}
            onClick={() => setBrowseMode(mode)}
            className={browseMode === mode ? "btn-primary" : "btn-secondary"}
            style={{ flex: 1, padding: "0.5rem" }}
          >
            {mode === "tier" ? "Browse by tier & search" : "🔎 Browse by topic"}
          </button>
        ))}
      </div>

      {browseMode === "topic" ? (
        <TopicBrowsePanel />
      ) : (
        <>
          <div style={{ display: "flex", gap: "0.75rem", flexWrap: "wrap", alignItems: "center", background: "rgba(0,0,0,0.2)", padding: "1rem", borderRadius: "12px", border: "1px solid var(--border-color)" }}>
            <div style={{ display: "flex", gap: "0.5rem", flex: 1, minWidth: 260 }}>
              <input
                placeholder={searchMode === "smart" ? "Smart search (similarity)…" : "Exact text search…"}
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                onKeyDown={(e) => { if (e.key === "Enter") searchMemories(); }}
                style={{ ...inputStyle, flex: 1 }}
              />
              <button className="btn-primary" onClick={searchMemories}>Search</button>
            </div>
            <div style={{ display: "inline-flex", background: "rgba(0,0,0,0.35)", padding: "0.2rem", borderRadius: "8px", border: "1px solid var(--border-color)" }}>
              <button type="button" onClick={() => setSearchMode("smart")} style={{ padding: "0.3rem 0.6rem", borderRadius: "6px", border: "none", background: searchMode === "smart" ? "rgba(56, 189, 248, 0.25)" : "transparent", color: searchMode === "smart" ? "#38bdf8" : "var(--text-muted)", fontSize: "0.75rem", fontWeight: 600, cursor: "pointer" }}>⚡ Smart</button>
              <button type="button" onClick={() => setSearchMode("exact")} style={{ padding: "0.3rem 0.6rem", borderRadius: "6px", border: "none", background: searchMode === "exact" ? "rgba(168, 85, 247, 0.25)" : "transparent", color: searchMode === "exact" ? "#c084fc" : "var(--text-muted)", fontSize: "0.75rem", fontWeight: 600, cursor: "pointer" }}>🔤 Exact</button>
            </div>
            {searching && <span style={{ fontSize: "0.75rem", color: "var(--text-muted)", fontStyle: "italic" }}>Searching…</span>}
            <select value={scopeFilter} onChange={(e) => setScopeFilter(e.target.value)} style={inputStyle}>
              <option value="">All scopes</option>
              <option value="workspace">workspace</option>
              <option value="global">global</option>
              <option value="session">session</option>
            </select>
            <select value={tierFilter} onChange={(e) => setTierFilter(e.target.value)} style={inputStyle}>
              <option value="">All tiers</option>
              <option value="short_term">short_term</option>
              <option value="episodic">episodic</option>
              <option value="semantic">semantic</option>
            </select>
            <button className="btn-secondary" onClick={refreshMemories}>Refresh</button>
            <button className="btn-secondary" onClick={async () => { await run("compacted", () => invoke("hub_compact_short_term", { keepNewest: 20 }), "Compacting short-term memories…"); await refreshMemories(); }}>Compact ST</button>
            <button className="btn-secondary" onClick={reindexVectors} title="Re-index missing vector embeddings for similarity search">Reindex Vectors</button>
          </div>

          {searchError && (
            <div style={{ fontSize: "0.8rem", color: "#f87171", background: "rgba(239, 68, 68, 0.1)", padding: "0.4rem 0.6rem", borderRadius: "6px" }}>
              Search notice: {searchError}
            </div>
          )}

          <div style={{ ...cardStyle, display: "grid", gap: "1rem" }}>
            <h3 style={{ margin: 0, fontSize: "1rem", fontWeight: 600, color: "var(--text-main)" }}>Write Memory</h3>
            <div style={{ display: "flex", gap: "0.75rem", flexWrap: "wrap" }}>
              <select value={memTier} onChange={(e) => setMemTier(e.target.value)} style={inputStyle}>
                <option value="short_term">short_term</option>
                <option value="episodic">episodic</option>
                <option value="semantic">semantic</option>
              </select>
              <select value={memScope} onChange={(e) => setMemScope(e.target.value)} style={inputStyle}>
                <option value="workspace">workspace</option>
                <option value="global">global</option>
                <option value="session">session</option>
              </select>
              <select value={memAgent} onChange={(e) => setMemAgent(e.target.value)} style={inputStyle}>
                {agents.map((a) => <option key={a.id} value={a.id}>{a.display_name}</option>)}
              </select>
              <input placeholder="Title (optional)" value={memTitle} onChange={(e) => setMemTitle(e.target.value)} style={{ ...inputStyle, flex: 1, minWidth: 150 }} />
            </div>
            <textarea rows={3} placeholder="Memory body…" value={memBody} onChange={(e) => setMemBody(e.target.value)} style={{ ...inputStyle, resize: "vertical", fontFamily: "var(--font-sans)" }} />
            <div style={{ display: "flex", justifyContent: "flex-end" }}>
              <button className="btn-primary" onClick={writeMemory} disabled={!memBody.trim()}>Save to hub</button>
            </div>
          </div>

          <div style={{ display: "flex", flexDirection: "column", gap: "1rem", paddingRight: "0.5rem" }}>
            {memories.length === 0 && (
              <div style={{ padding: "3rem", textAlign: "center", background: "rgba(0,0,0,0.2)", borderRadius: "12px", border: "1px dashed var(--border-color)" }}>
                <p style={{ color: "var(--text-muted)", fontSize: "0.95rem", margin: 0 }}>
                  {query.trim() ? `No memories matching ${searchMode === "smart" ? "smart similarity" : "exact text"} query.` : "No memories found in the hub."}
                </p>
              </div>
            )}
            {memories.map((m) => (
              <div key={m.id} style={{ ...cardStyle, position: "relative", overflow: "hidden" }}>
                <div style={{ display: "flex", justifyContent: "space-between", gap: "1rem", flexWrap: "wrap", marginBottom: "0.75rem", paddingBottom: "0.75rem", borderBottom: "1px solid var(--border-color)" }}>
                  <div style={{ flex: 1 }}>
                    {editingMemory === m.id ? (
                      <input value={editTitle} onChange={(e) => setEditTitle(e.target.value)} placeholder="Memory title (optional)" style={{ ...inputStyle, width: "100%", marginBottom: "0.5rem" }} />
                    ) : (
                      <strong style={{ fontSize: "1.1rem", color: m.stale ? "var(--text-muted)" : "var(--primary)", textDecoration: m.stale ? "line-through" : "none" }}>
                        {m.title || "(untitled)"}
                      </strong>
                    )}
                    <div style={{ display: "flex", gap: "0.5rem", marginTop: "0.35rem", alignItems: "center", flexWrap: "wrap" }}>
                      <span style={{ fontSize: "0.7rem", padding: "0.1rem 0.4rem", background: "rgba(255,255,255,0.1)", borderRadius: "4px", color: "var(--text-main)" }}>{m.tier}</span>
                      <span style={{ fontSize: "0.7rem", padding: "0.1rem 0.4rem", background: m.scope === "global" ? "rgba(16, 185, 129, 0.1)" : "rgba(56, 189, 248, 0.1)", borderRadius: "4px", color: m.scope === "global" ? "#10b981" : "#38bdf8" }}>{m.scope}</span>
                      {m.agent_id && <span style={{ fontSize: "0.7rem", padding: "0.1rem 0.4rem", background: "rgba(168, 85, 247, 0.1)", borderRadius: "4px", color: "#a855f7" }}>{m.agent_id}</span>}
                      {"score" in m && typeof m.score === "number" && (
                        <span title={`Similarity score: ${m.score.toFixed(4)}`} style={{ fontSize: "0.7rem", padding: "0.1rem 0.45rem", background: "rgba(56, 189, 248, 0.18)", borderRadius: "4px", color: "#38bdf8", border: "1px solid rgba(56, 189, 248, 0.3)", fontWeight: 700 }}>
                          ⚡ {formatScore(m.score)} match
                        </span>
                      )}
                      {m.stale && <span style={{ fontSize: "0.7rem", padding: "0.1rem 0.4rem", background: "rgba(239, 68, 68, 0.1)", borderRadius: "4px", color: "#ef4444" }}>STALE</span>}
                    </div>
                  </div>
                  <div style={{ display: "flex", gap: "0.4rem", alignItems: "flex-start", flexWrap: "wrap" }}>
                    {editingMemory === m.id ? (
                      <>
                        <button className="btn-primary" style={{ fontSize: "0.75rem", padding: "0.2rem 0.5rem" }} onClick={() => saveEditedMemory(m.id)}>Save</button>
                        <button className="btn-secondary" style={{ fontSize: "0.75rem", padding: "0.2rem 0.5rem" }} onClick={() => setEditingMemory(null)}>Cancel</button>
                      </>
                    ) : (
                      <>
                        <button className="btn-secondary" style={{ fontSize: "0.75rem", padding: "0.2rem 0.5rem" }} onClick={() => setExpandedMemoryId(expandedMemoryId === m.id ? null : m.id)}>
                          {expandedMemoryId === m.id ? "Hide links" : "🔗 Links"}
                        </button>
                        <button className="btn-secondary" style={{ fontSize: "0.75rem", padding: "0.2rem 0.5rem" }} onClick={() => copyMemoryReference(m.id)}>Copy reference</button>
                        <button className="btn-secondary" style={{ fontSize: "0.75rem", padding: "0.2rem 0.5rem" }} onClick={() => { setEditingMemory(m.id); setEditTitle(m.title || ""); setEditBody(m.body); }}>Edit</button>
                        {m.tier === "short_term" && (
                          <button className="btn-secondary" style={{ fontSize: "0.75rem", padding: "0.2rem 0.5rem" }} onClick={async () => { await run("promoted", () => invoke("hub_promote_memory", { id: m.id, toTier: "episodic" })); await refreshMemories(); }}>→ episodic</button>
                        )}
                        {m.tier === "episodic" && (
                          <button className="btn-secondary" style={{ fontSize: "0.75rem", padding: "0.2rem 0.5rem" }} onClick={async () => { await run("promoted", () => invoke("hub_promote_memory", { id: m.id, toTier: "semantic" })); await refreshMemories(); }}>→ semantic</button>
                        )}
                        <button className="btn-secondary" style={{ fontSize: "0.75rem", padding: "0.2rem 0.5rem" }} onClick={async () => { await run("stale", () => invoke("hub_mark_memory_stale", { id: m.id, stale: true })); await refreshMemories(); }}>Stale</button>
                        <button className="btn-secondary" style={{ fontSize: "0.75rem", padding: "0.2rem 0.5rem", borderColor: "rgba(239, 68, 68, 0.3)", color: "#ef4444" }} onClick={async () => { await run("deleted", () => invoke("hub_delete_memory", { id: m.id })); await refreshMemories(); }}>Delete</button>
                      </>
                    )}
                  </div>
                </div>
                {editingMemory === m.id ? (
                  <textarea rows={4} value={editBody} onChange={(e) => setEditBody(e.target.value)} style={{ ...inputStyle, width: "100%", resize: "vertical", fontFamily: "var(--font-sans)" }} />
                ) : (
                  <pre style={{ margin: 0, whiteSpace: "pre-wrap", fontSize: "0.9rem", color: m.stale ? "var(--text-muted)" : "var(--text-main)", fontFamily: "var(--font-sans)", lineHeight: 1.5 }}>{m.body}</pre>
                )}
                {expandedMemoryId === m.id && <MemoryLinksSection memoryId={m.id} currentAgentId="human" />}
                <div style={{ fontSize: "0.7rem", color: "var(--text-muted)", marginTop: "1rem", textAlign: "right" }}>
                  {m.created_at} · <span style={{ fontFamily: "var(--font-mono)" }}>{m.id.slice(0, 8)}</span>
                </div>
              </div>
            ))}
          </div>
        </>
      )}
    </div>
  );
}
