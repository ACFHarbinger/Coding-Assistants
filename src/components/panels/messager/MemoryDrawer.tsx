// @ts-nocheck
import { useEffect, useState } from "react";
import { invoke } from "../../../lib/tauri";
import MemoryLinksSection from "./MemoryLinksSection";
import TopicBrowsePanel from "./TopicBrowsePanel";
import type { MemoryRecord, ScoredMemoryRecord } from "./types";

export default function MemoryDrawer(props: any) {
  const {
    showMemoryDrawer,
    setShowMemoryDrawer,
    memorySearch,
    setMemorySearch,
    selectedTierFilter,
    setSelectedTierFilter,
    memories,
    setMessageInput,
    workspacePath,
  } = props;

  const [browseMode, setBrowseMode] = useState<"tier" | "topic">("tier");
  const [searchMode, setSearchMode] = useState<"smart" | "exact">("smart");
  const [selectedScopeFilter, setSelectedScopeFilter] = useState<string>("all");
  const [expandedMemoryId, setExpandedMemoryId] = useState<string | null>(null);
  const [smartResults, setSmartResults] = useState<ScoredMemoryRecord[] | null>(null);
  const [searching, setSearching] = useState(false);
  const [searchError, setSearchError] = useState<string | null>(null);
  const [consolidating, setConsolidating] = useState(false);
  const [consolidationStatus, setConsolidationStatus] = useState<string | null>(null);

  if (!showMemoryDrawer) return null;

  // Execute smart hybrid search when in smart mode and query is present
  useEffect(() => {
    if (browseMode !== "tier" || searchMode !== "smart") {
      setSmartResults(null);
      setSearchError(null);
      return;
    }

    const query = memorySearch.trim();
    if (!query) {
      setSmartResults(null);
      setSearchError(null);
      return;
    }

    let isMounted = true;
    setSearching(true);
    setSearchError(null);

    const timer = setTimeout(async () => {
      try {
        const results = await invoke<ScoredMemoryRecord[]>("hub_search_memories_hybrid", {
          query,
          limit: 30,
          scope: selectedScopeFilter === "all" ? null : selectedScopeFilter,
          tier: selectedTierFilter === "all" ? null : selectedTierFilter,
          workspace: selectedScopeFilter === "workspace" ? (workspacePath || null) : null,
        });
        if (isMounted) {
          setSmartResults(results);
          setSearching(false);
        }
      } catch (err) {
        if (isMounted) {
          setSearchError(String(err));
          setSearching(false);
          setSmartResults(null);
        }
      }
    }, 200);

    return () => {
      isMounted = false;
      clearTimeout(timer);
    };
  }, [memorySearch, searchMode, selectedTierFilter, selectedScopeFilter, browseMode, workspacePath]);

  // Exact fallback filtering
  const exactFilteredMemories = memories
    .filter((m: MemoryRecord) => selectedTierFilter === "all" || m.tier === selectedTierFilter)
    .filter((m: MemoryRecord) => selectedScopeFilter === "all" || m.scope === selectedScopeFilter)
    .filter((m: MemoryRecord) => {
      const query = memorySearch.trim().toLowerCase();
      return !query || (m.title || "").toLowerCase().includes(query) || m.body.toLowerCase().includes(query);
    });

  const displayMemories = (searchMode === "smart" && memorySearch.trim() && smartResults !== null)
    ? smartResults
    : exactFilteredMemories;

  const insertMemoryLink = (memoryId: string) =>
    setMessageInput((previous: string) => `${previous} [Memory #${memoryId.slice(0, 8)}]`);

  const formatScore = (score: number) => {
    if (score >= 1.0) return "100%";
    if (score <= 0.0) return "0%";
    return `${Math.round(score * 100)}%`;
  };

  return (
    <div
      className="glass-card"
      style={{
        padding: "1.25rem 1rem",
        display: "flex",
        flexDirection: "column",
        gap: "0.85rem",
        overflowY: "auto",
      }}
    >
      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", flexWrap: "wrap", gap: "0.4rem" }}>
        <h3 style={{ margin: 0, fontSize: "1rem", fontWeight: 700, color: "var(--primary)" }}>
          🧠 Agentic Memory Hub
        </h3>
        <div style={{ display: "flex", gap: "0.35rem", alignItems: "center" }}>
          <button
            onClick={async () => {
              setConsolidating(true);
              setConsolidationStatus("Consolidating…");
              try {
                const res = await invoke<any>("hub_consolidate_memories", {
                  args: {
                    model_config: {
                      provider: "google",
                      model: "gemini-2.5-flash",
                      prompt_file: null,
                      rule_file: null,
                      workflow_file: null,
                      endpoint: null,
                    },
                    workspace: workspacePath || null,
                  },
                });
                setConsolidationStatus(
                  `Consolidated ${res.consolidated} cluster(s)${res.notice ? ` (${res.notice})` : ""}`,
                );
              } catch (e) {
                setConsolidationStatus(`Failed: ${String(e)}`);
              } finally {
                setConsolidating(false);
              }
            }}
            disabled={consolidating}
            className="btn-secondary"
            style={{ padding: "0.25rem 0.5rem", fontSize: "0.72rem" }}
            title="Cluster related short-term memories and summarize into episodic records"
          >
            {consolidating ? "Consolidating…" : "Consolidate (M3)"}
          </button>
          <button
            onClick={() => setShowMemoryDrawer(false)}
            className="btn-secondary"
            style={{ padding: "0.25rem 0.5rem", fontSize: "0.75rem" }}
          >
            ✕ Close
          </button>
        </div>
      </div>

      {consolidationStatus && (
        <div
          style={{
            fontSize: "0.72rem",
            color: consolidationStatus.startsWith("Failed") ? "#f87171" : "#4ade80",
            background: consolidationStatus.startsWith("Failed")
              ? "rgba(239, 68, 68, 0.1)"
              : "rgba(34, 197, 94, 0.1)",
            padding: "0.25rem 0.5rem",
            borderRadius: "6px",
          }}
        >
          {consolidationStatus}
        </div>
      )}

      {/* Browse mode toggle */}
      <div style={{ display: "flex", gap: "0.35rem" }}>
        {(["tier", "topic"] as const).map((mode) => (
          <button
            key={mode}
            onClick={() => setBrowseMode(mode)}
            style={{
              flex: 1,
              padding: "0.35rem 0.6rem",
              borderRadius: "6px",
              border: "none",
              background: browseMode === mode ? "var(--primary)" : "rgba(255, 255, 255, 0.08)",
              color: "#fff",
              fontSize: "0.75rem",
              fontWeight: 600,
              cursor: "pointer",
            }}
          >
            {mode === "tier" ? "Browse by tier & search" : "🔎 Browse by topic"}
          </button>
        ))}
      </div>

      {browseMode === "topic" ? (
        <TopicBrowsePanel />
      ) : (
        <>
          {/* Search bar + Mode switch */}
          <div style={{ display: "flex", flexDirection: "column", gap: "0.4rem" }}>
            <div style={{ display: "flex", gap: "0.35rem" }}>
              <input
                type="text"
                placeholder={searchMode === "smart" ? "Smart search (similarity)…" : "Exact text search…"}
                value={memorySearch}
                onChange={(e) => setMemorySearch(e.target.value)}
                style={{
                  flex: 1,
                  padding: "0.5rem 0.75rem",
                  borderRadius: "8px",
                  background: "rgba(0,0,0,0.4)",
                  border: "1px solid var(--border-color)",
                  color: "#fff",
                  fontSize: "0.85rem",
                  outline: "none",
                }}
              />
              {memorySearch && (
                <button
                  className="btn-secondary"
                  onClick={() => setMemorySearch("")}
                  style={{ padding: "0.3rem 0.5rem", fontSize: "0.75rem" }}
                  title="Clear search"
                >
                  ✕
                </button>
              )}
            </div>

            {/* Smart (similarity) vs Exact toggle */}
            <div style={{ display: "flex", gap: "0.35rem", alignItems: "center", justifyContent: "space-between" }}>
              <div style={{ display: "flex", gap: "0.25rem" }}>
                <button
                  type="button"
                  onClick={() => setSearchMode("smart")}
                  style={{
                    padding: "0.2rem 0.5rem",
                    borderRadius: "5px",
                    border: "none",
                    background: searchMode === "smart" ? "rgba(56, 189, 248, 0.25)" : "rgba(255, 255, 255, 0.06)",
                    color: searchMode === "smart" ? "#38bdf8" : "var(--text-muted)",
                    fontSize: "0.72rem",
                    fontWeight: 600,
                    cursor: "pointer",
                    display: "inline-flex",
                    alignItems: "center",
                    gap: "0.25rem",
                  }}
                >
                  <span>⚡</span> Smart (similarity)
                </button>
                <button
                  type="button"
                  onClick={() => setSearchMode("exact")}
                  style={{
                    padding: "0.2rem 0.5rem",
                    borderRadius: "5px",
                    border: "none",
                    background: searchMode === "exact" ? "rgba(168, 85, 247, 0.25)" : "rgba(255, 255, 255, 0.06)",
                    color: searchMode === "exact" ? "#c084fc" : "var(--text-muted)",
                    fontSize: "0.72rem",
                    fontWeight: 600,
                    cursor: "pointer",
                    display: "inline-flex",
                    alignItems: "center",
                    gap: "0.25rem",
                  }}
                >
                  <span>🔤</span> Exact (text)
                </button>
              </div>
              {searching && (
                <span style={{ fontSize: "0.72rem", color: "var(--text-muted)", fontStyle: "italic" }}>
                  Searching…
                </span>
              )}
            </div>
          </div>

          {/* Scope Filters */}
          <div style={{ display: "flex", gap: "0.25rem", overflowX: "auto", paddingBottom: "0.1rem" }}>
            <span style={{ fontSize: "0.7rem", color: "var(--text-muted)", alignSelf: "center", marginRight: "0.2rem" }}>
              Scope:
            </span>
            {["all", "workspace", "global", "session"].map((s) => (
              <button
                key={s}
                onClick={() => setSelectedScopeFilter(s)}
                style={{
                  padding: "0.18rem 0.45rem",
                  borderRadius: "5px",
                  border: "none",
                  background: selectedScopeFilter === s ? "rgba(34, 197, 94, 0.2)" : "rgba(255, 255, 255, 0.06)",
                  color: selectedScopeFilter === s ? "#4ade80" : "var(--text-muted)",
                  fontSize: "0.7rem",
                  fontWeight: selectedScopeFilter === s ? 600 : 400,
                  cursor: "pointer",
                  whiteSpace: "nowrap",
                }}
              >
                {s}
              </button>
            ))}
          </div>

          {/* Memory Tier Filter Pills */}
          <div style={{ display: "flex", gap: "0.25rem", overflowX: "auto", paddingBottom: "0.15rem" }}>
            <span style={{ fontSize: "0.7rem", color: "var(--text-muted)", alignSelf: "center", marginRight: "0.2rem" }}>
              Tier:
            </span>
            {["all", "short_term", "episodic", "semantic"].map((t) => (
              <button
                key={t}
                onClick={() => setSelectedTierFilter(t)}
                style={{
                  padding: "0.18rem 0.45rem",
                  borderRadius: "5px",
                  border: "none",
                  background: selectedTierFilter === t ? "var(--primary)" : "rgba(255, 255, 255, 0.06)",
                  color: "#fff",
                  fontSize: "0.7rem",
                  fontWeight: selectedTierFilter === t ? 600 : 400,
                  cursor: "pointer",
                  whiteSpace: "nowrap",
                }}
              >
                {t.replace("_", " ")}
              </button>
            ))}
          </div>

          {searchError && (
            <div style={{ fontSize: "0.75rem", color: "#f87171", background: "rgba(239, 68, 68, 0.1)", padding: "0.3rem 0.5rem", borderRadius: "6px" }}>
              Search notice: {searchError}
            </div>
          )}

          {/* Memories List */}
          <div style={{ flex: 1, overflowY: "auto", display: "flex", flexDirection: "column", gap: "0.75rem" }}>
            {displayMemories.length === 0 ? (
              <p style={{ fontSize: "0.85rem", color: "var(--text-muted)", textAlign: "center", marginTop: "1rem" }}>
                {memorySearch.trim()
                  ? `No matching records for ${searchMode === "smart" ? "smart similarity" : "exact"} search.`
                  : "No memory records found."}
              </p>
            ) : (
              displayMemories.map((m: any) => (
                <div
                  key={m.id}
                  style={{
                    background: "rgba(0,0,0,0.3)",
                    border: "1px solid var(--border-color)",
                    borderRadius: "10px",
                    padding: "0.75rem",
                    fontSize: "0.85rem",
                    display: "flex",
                    flexDirection: "column",
                    gap: "0.4rem",
                  }}
                >
                  <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", flexWrap: "wrap", gap: "0.3rem" }}>
                    <div style={{ display: "flex", gap: "0.3rem", alignItems: "center" }}>
                      <span
                        style={{
                          fontWeight: 700,
                          fontSize: "0.72rem",
                          color: "var(--accent)",
                          background: "rgba(168, 85, 247, 0.15)",
                          padding: "0.1rem 0.4rem",
                          borderRadius: "4px",
                        }}
                      >
                        {m.tier}
                      </span>
                      <span
                        style={{
                          fontSize: "0.68rem",
                          color: m.scope === "global" ? "#10b981" : "#38bdf8",
                          background: m.scope === "global" ? "rgba(16, 185, 129, 0.12)" : "rgba(56, 189, 248, 0.12)",
                          padding: "0.1rem 0.35rem",
                          borderRadius: "4px",
                        }}
                      >
                        {m.scope}
                      </span>
                      {typeof m.score === "number" && (
                        <span
                          title={`Similarity ranking score: ${m.score.toFixed(4)}`}
                          style={{
                            fontSize: "0.68rem",
                            fontWeight: 700,
                            color: "#38bdf8",
                            background: "rgba(56, 189, 248, 0.18)",
                            padding: "0.1rem 0.4rem",
                            borderRadius: "4px",
                            border: "1px solid rgba(56, 189, 248, 0.3)",
                            display: "inline-flex",
                            alignItems: "center",
                            gap: "0.2rem",
                          }}
                        >
                          ⚡ {formatScore(m.score)}
                        </span>
                      )}
                    </div>
                    <div style={{ display: "flex", gap: "0.5rem" }}>
                      <button
                        onClick={() => setExpandedMemoryId(expandedMemoryId === m.id ? null : m.id)}
                        style={{
                          background: "transparent",
                          border: "none",
                          color: "var(--accent)",
                          fontSize: "0.75rem",
                          cursor: "pointer",
                          fontWeight: 600,
                        }}
                      >
                        {expandedMemoryId === m.id ? "Hide links" : "🔗 Links"}
                      </button>
                      <button
                        onClick={() => insertMemoryLink(m.id)}
                        style={{
                          background: "transparent",
                          border: "none",
                          color: "var(--primary)",
                          fontSize: "0.75rem",
                          cursor: "pointer",
                          fontWeight: 600,
                        }}
                      >
                        + Attach
                      </button>
                    </div>
                  </div>
                  <div style={{ fontWeight: 600, color: "var(--text-main)" }}>
                    {m.title || `Memory #${m.id.slice(0, 8)}`}
                  </div>
                  <div
                    style={{
                      color: "var(--text-muted)",
                      fontSize: "0.8rem",
                      display: "-webkit-box",
                      WebkitLineClamp: 3,
                      WebkitBoxOrient: "vertical",
                      overflow: "hidden",
                    }}
                  >
                    {m.body}
                  </div>
                  {expandedMemoryId === m.id && (
                    <MemoryLinksSection memoryId={m.id} currentAgentId="human" />
                  )}
                </div>
              ))
            )}
          </div>
        </>
      )}
    </div>
  );
}
