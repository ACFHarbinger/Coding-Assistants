// @ts-nocheck
import { cardStyle, inputStyle } from "./HubCharts";
import { consolidateMemories } from "./memoryApi";

export default function MemoryTab(props: any) {
  const {
    memories,
    searchQ,
    setSearchQ,
    searchMode,
    setSearchMode,
    searchMemories,
    refreshMemories,
    tierFilter,
    setTierFilter,
    scopeFilter,
    setScopeFilter,
    memTier,
    setMemTier,
    memScope,
    setMemScope,
    memAgent,
    setMemAgent,
    memTitle,
    setMemTitle,
    memBody,
    setMemBody,
    writeMemory,
    editingMemory,
    setEditingMemory,
    editTitle,
    setEditTitle,
    editBody,
    setEditBody,
    saveEditedMemory,
    run,
    invoke,
    agents,
    setStatus,
    reindexVectors,
  } = props;

  const formatScore = (score: number) => {
    if (score >= 1.0) return "100%";
    if (score <= 0.0) return "0%";
    return `${Math.round(score * 100)}%`;
  };

  return (
    <div className="fade-in" style={{ display: "flex", flexDirection: "column", gap: "1.5rem" }}>
      {/* Search and control bar */}
      <div
        style={{
          display: "flex",
          gap: "0.75rem",
          flexWrap: "wrap",
          alignItems: "center",
          background: "rgba(0,0,0,0.2)",
          padding: "1rem",
          borderRadius: "12px",
          border: "1px solid var(--border-color)",
        }}
      >
        <div style={{ display: "flex", gap: "0.5rem", flex: 1, minWidth: 260 }}>
          <input
            placeholder={searchMode === "smart" ? "Smart search (similarity)…" : "Exact text search…"}
            value={searchQ}
            onChange={(e) => setSearchQ(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") searchMemories();
            }}
            style={{ ...inputStyle, flex: 1 }}
            onFocus={(e) => (e.target.style.borderColor = "var(--primary)")}
            onBlur={(e) => (e.target.style.borderColor = "var(--border-color)")}
          />
          <button className="btn-primary" onClick={searchMemories}>
            Search
          </button>
        </div>

        {/* Search mode toggle */}
        <div
          style={{
            display: "inline-flex",
            background: "rgba(0, 0, 0, 0.35)",
            padding: "0.2rem",
            borderRadius: "8px",
            border: "1px solid var(--border-color)",
          }}
        >
          <button
            type="button"
            onClick={() => setSearchMode("smart")}
            style={{
              padding: "0.3rem 0.6rem",
              borderRadius: "6px",
              border: "none",
              background: searchMode === "smart" ? "rgba(56, 189, 248, 0.25)" : "transparent",
              color: searchMode === "smart" ? "#38bdf8" : "var(--text-muted)",
              fontSize: "0.75rem",
              fontWeight: 600,
              cursor: "pointer",
            }}
          >
            ⚡ Smart (similarity)
          </button>
          <button
            type="button"
            onClick={() => setSearchMode("exact")}
            style={{
              padding: "0.3rem 0.6rem",
              borderRadius: "6px",
              border: "none",
              background: searchMode === "exact" ? "rgba(168, 85, 247, 0.25)" : "transparent",
              color: searchMode === "exact" ? "#c084fc" : "var(--text-muted)",
              fontSize: "0.75rem",
              fontWeight: 600,
              cursor: "pointer",
            }}
          >
            🔤 Exact (text)
          </button>
        </div>

        {/* Filters */}
        <select value={tierFilter} onChange={(e) => setTierFilter(e.target.value)} style={inputStyle}>
          <option value="">All tiers</option>
          <option value="short_term">short_term</option>
          <option value="episodic">episodic</option>
          <option value="semantic">semantic</option>
        </select>

        <select value={scopeFilter} onChange={(e) => setScopeFilter(e.target.value)} style={inputStyle}>
          <option value="">All scopes</option>
          <option value="workspace">workspace</option>
          <option value="global">global</option>
          <option value="session">session</option>
        </select>

        <button className="btn-secondary" onClick={refreshMemories}>
          Refresh
        </button>
        <button
          className="btn-secondary"
          onClick={async () => {
            await run("compacted", () => invoke("hub_compact_short_term", { keepNewest: 20 }), "Compacting short-term memories…");
            await refreshMemories();
          }}
        >
          Compact ST
        </button>
        <button
          className="btn-secondary"
          onClick={async () => {
            const report = await run(
              "consolidation done",
              () => consolidateMemories(null),
              "Consolidating short-term memories with LLM…",
            );
            if (report) {
              setStatus(
                `Consolidated ${report.consolidated} cluster(s) (${report.skipped} skipped)${
                  report.notice ? ` — ${report.notice}` : ""
                }`,
              );
              await refreshMemories();
            }
          }}
          title="Cluster related short-term memories and summarize into episodic records"
        >
          Consolidate (M3)
        </button>
        <button className="btn-secondary" onClick={reindexVectors} title="Re-index missing vector embeddings for similarity search">
          Reindex Vectors
        </button>
        <button
          className="btn-secondary"
          onClick={async () => {
            const outcome = await run(
              "export_committed",
              () =>
                invoke<{ path: string; committed: boolean; detail: string }>(
                  "hub_export_markdown_git",
                  { message: null },
                ),
              "Exporting + committing…",
            );
            if (outcome) {
              setStatus(
                outcome.committed
                  ? `exported + committed → ${outcome.path}`
                  : `exported → ${outcome.path} (${outcome.detail})`,
              );
            }
          }}
        >
          Export MD + Commit
        </button>
      </div>

      {/* Write Memory Form */}
      <div style={{ ...cardStyle, display: "grid", gap: "1rem" }}>
        <h3 style={{ margin: 0, fontSize: "1rem", fontWeight: 600, color: "var(--text-main)" }}>
          Write Memory
        </h3>
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
            {agents.map((a: any) => (
              <option key={a.id} value={a.id}>
                {a.display_name}
              </option>
            ))}
          </select>
          <input
            placeholder="Title (optional)"
            value={memTitle}
            onChange={(e) => setMemTitle(e.target.value)}
            style={{ ...inputStyle, flex: 1, minWidth: 150 }}
          />
        </div>
        <textarea
          rows={3}
          placeholder="Memory body…"
          value={memBody}
          onChange={(e) => setMemBody(e.target.value)}
          style={{ ...inputStyle, resize: "vertical", fontFamily: "var(--font-sans)" }}
        />
        <div style={{ display: "flex", justifyContent: "flex-end" }}>
          <button className="btn-primary" onClick={writeMemory} disabled={!memBody.trim()}>
            Save to hub
          </button>
        </div>
      </div>

      {/* Memories List */}
      <div style={{ display: "flex", flexDirection: "column", gap: "1rem", maxHeight: 500, overflowY: "auto", paddingRight: "0.5rem" }}>
        {memories.length === 0 && (
          <div
            style={{
              padding: "3rem",
              textAlign: "center",
              background: "rgba(0,0,0,0.2)",
              borderRadius: "12px",
              border: "1px dashed var(--border-color)",
            }}
          >
            <p style={{ color: "var(--text-muted)", fontSize: "0.95rem", margin: 0 }}>
              {searchQ.trim()
                ? `No memories matching ${searchMode === "smart" ? "smart similarity" : "exact text"} query.`
                : "No memories found in the hub."}
            </p>
          </div>
        )}
        {memories.map((m: any) => (
          <div
            key={m.id}
            style={{ ...cardStyle, position: "relative", overflow: "hidden" }}
            onMouseEnter={(e) => (e.currentTarget.style.borderColor = "var(--primary)")}
            onMouseLeave={(e) => (e.currentTarget.style.borderColor = "var(--border-color)")}
          >
            <div
              style={{
                display: "flex",
                justifyContent: "space-between",
                gap: "1rem",
                flexWrap: "wrap",
                marginBottom: "0.75rem",
                paddingBottom: "0.75rem",
                borderBottom: "1px solid var(--border-color)",
              }}
            >
              <div style={{ flex: 1 }}>
                {editingMemory === m.id ? (
                  <input
                    value={editTitle}
                    onChange={(e) => setEditTitle(e.target.value)}
                    placeholder="Memory title (optional)"
                    style={{ ...inputStyle, width: "100%", marginBottom: "0.5rem" }}
                  />
                ) : (
                  <strong
                    style={{
                      fontSize: "1.1rem",
                      color: m.stale ? "var(--text-muted)" : "var(--primary)",
                      textDecoration: m.stale ? "line-through" : "none",
                    }}
                  >
                    {m.title || "(untitled)"}
                  </strong>
                )}
                <div style={{ display: "flex", gap: "0.5rem", marginTop: "0.35rem", alignItems: "center", flexWrap: "wrap" }}>
                  <span
                    style={{
                      fontSize: "0.7rem",
                      padding: "0.1rem 0.4rem",
                      background: "rgba(255,255,255,0.1)",
                      borderRadius: "4px",
                      color: "var(--text-main)",
                    }}
                  >
                    {m.tier}
                  </span>
                  <span
                    style={{
                      fontSize: "0.7rem",
                      padding: "0.1rem 0.4rem",
                      background: m.scope === "global" ? "rgba(16, 185, 129, 0.1)" : "rgba(56, 189, 248, 0.1)",
                      borderRadius: "4px",
                      color: m.scope === "global" ? "#10b981" : "#38bdf8",
                    }}
                  >
                    {m.scope}
                  </span>
                  <span
                    style={{
                      fontSize: "0.7rem",
                      padding: "0.1rem 0.4rem",
                      background: "rgba(168, 85, 247, 0.1)",
                      borderRadius: "4px",
                      color: "#a855f7",
                    }}
                  >
                    {m.agent_id || "global"}
                  </span>
                  {typeof m.score === "number" && (
                    <span
                      title={`Similarity score: ${m.score.toFixed(4)}`}
                      style={{
                        fontSize: "0.7rem",
                        padding: "0.1rem 0.45rem",
                        background: "rgba(56, 189, 248, 0.18)",
                        borderRadius: "4px",
                        color: "#38bdf8",
                        border: "1px solid rgba(56, 189, 248, 0.3)",
                        fontWeight: 700,
                        display: "inline-flex",
                        alignItems: "center",
                        gap: "0.2rem",
                      }}
                    >
                      ⚡ {formatScore(m.score)} match
                    </span>
                  )}
                  {m.stale && (
                    <span
                      style={{
                        fontSize: "0.7rem",
                        padding: "0.1rem 0.4rem",
                        background: "rgba(239, 68, 68, 0.1)",
                        borderRadius: "4px",
                        color: "#ef4444",
                      }}
                    >
                      STALE
                    </span>
                  )}
                </div>
              </div>
              <div style={{ display: "flex", gap: "0.4rem", alignItems: "flex-start" }}>
                {editingMemory === m.id ? (
                  <>
                    <button
                      className="btn-primary"
                      style={{ fontSize: "0.75rem", padding: "0.2rem 0.5rem" }}
                      onClick={() => saveEditedMemory(m.id)}
                    >
                      Save
                    </button>
                    <button
                      className="btn-secondary"
                      style={{ fontSize: "0.75rem", padding: "0.2rem 0.5rem" }}
                      onClick={() => setEditingMemory(null)}
                    >
                      Cancel
                    </button>
                  </>
                ) : (
                  <>
                    <button
                      className="btn-secondary"
                      style={{ fontSize: "0.75rem", padding: "0.2rem 0.5rem" }}
                      onClick={() => {
                        setEditingMemory(m.id);
                        setEditTitle(m.title || "");
                        setEditBody(m.body);
                      }}
                    >
                      Edit
                    </button>
                    {m.tier === "short_term" && (
                      <button
                        className="btn-secondary"
                        style={{ fontSize: "0.75rem", padding: "0.2rem 0.5rem" }}
                        onClick={async () => {
                          await run("promoted", () =>
                            invoke("hub_promote_memory", { id: m.id, toTier: "episodic" }),
                          );
                          await refreshMemories();
                        }}
                      >
                        → episodic
                      </button>
                    )}
                    {m.tier === "episodic" && (
                      <button
                        className="btn-secondary"
                        style={{ fontSize: "0.75rem", padding: "0.2rem 0.5rem" }}
                        onClick={async () => {
                          await run("promoted", () =>
                            invoke("hub_promote_memory", { id: m.id, toTier: "semantic" }),
                          );
                          await refreshMemories();
                        }}
                      >
                        → semantic
                      </button>
                    )}
                    <button
                      className="btn-secondary"
                      style={{ fontSize: "0.75rem", padding: "0.2rem 0.5rem" }}
                      onClick={async () => {
                        await run("stale", () =>
                          invoke("hub_mark_memory_stale", { id: m.id, stale: true }),
                        );
                        await refreshMemories();
                      }}
                    >
                      Stale
                    </button>
                    <button
                      className="btn-secondary"
                      style={{
                        fontSize: "0.75rem",
                        padding: "0.2rem 0.5rem",
                        borderColor: "rgba(239, 68, 68, 0.3)",
                        color: "#ef4444",
                      }}
                      onClick={async () => {
                        await run("deleted", () =>
                          invoke("hub_delete_memory", { id: m.id }),
                        );
                        await refreshMemories();
                      }}
                    >
                      Delete
                    </button>
                  </>
                )}
              </div>
            </div>
            {editingMemory === m.id ? (
              <textarea
                rows={4}
                value={editBody}
                onChange={(e) => setEditBody(e.target.value)}
                style={{ ...inputStyle, width: "100%", resize: "vertical", fontFamily: "var(--font-sans)" }}
              />
            ) : (
              <pre
                style={{
                  margin: "0",
                  whiteSpace: "pre-wrap",
                  fontSize: "0.9rem",
                  color: m.stale ? "var(--text-muted)" : "var(--text-main)",
                  fontFamily: "var(--font-sans)",
                  lineHeight: 1.5,
                }}
              >
                {m.body}
              </pre>
            )}
            <div style={{ fontSize: "0.7rem", color: "var(--text-muted)", marginTop: "1rem", textAlign: "right" }}>
              {m.created_at} · <span style={{ fontFamily: "var(--font-mono)" }}>{m.id.slice(0, 8)}</span>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
