// @ts-nocheck
export default function MemoryDrawer(props: any) {
  const { showMemoryDrawer, setShowMemoryDrawer, memorySearch, setMemorySearch, selectedTierFilter, setSelectedTierFilter, memories, setMessageInput } = props;
  if (!showMemoryDrawer) return null;
  const filteredMemories = memories.filter((memory) => selectedTierFilter === "all" || memory.tier === selectedTierFilter).filter((memory) => {
    const query = memorySearch.trim().toLowerCase();
    return !query || (memory.title || "").toLowerCase().includes(query) || memory.body.toLowerCase().includes(query);
  });
  const insertMemoryLink = (memoryId) => setMessageInput((previous) => `${previous} [Memory #${memoryId.slice(0, 8)}]`);
  return (
        <div className="glass-card" style={{
          padding: "1.25rem 1rem",
          display: "flex",
          flexDirection: "column",
          gap: "1rem",
          overflowY: "auto"
        }}>
          <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
            <h3 style={{ margin: 0, fontSize: "1rem", fontWeight: 700, color: "var(--primary)" }}>
              🧠 Agentic Memory Hub
            </h3>
            <button
              onClick={() => setShowMemoryDrawer(false)}
              className="btn-secondary"
              style={{ padding: "0.25rem 0.5rem", fontSize: "0.75rem" }}
            >
              ✕ Close
            </button>
          </div>

          {/* Search Memories */}
          <input
            type="text"
            placeholder="Filter memories..."
            value={memorySearch}
            onChange={e => setMemorySearch(e.target.value)}
            style={{
              padding: "0.5rem 0.75rem",
              borderRadius: "8px",
              background: "rgba(0,0,0,0.4)",
              border: "1px solid var(--border-color)",
              color: "#fff",
              fontSize: "0.85rem",
              outline: "none"
            }}
          />

          {/* Memory Tier Filter Pills */}
          <div style={{ display: "flex", gap: "0.35rem", overflowX: "auto", paddingBottom: "0.25rem" }}>
            {["all", "short_term", "episodic", "semantic"].map(t => (
              <button
                key={t}
                onClick={() => setSelectedTierFilter(t)}
                style={{
                  padding: "0.25rem 0.55rem",
                  borderRadius: "6px",
                  border: "none",
                  background: selectedTierFilter === t ? "var(--primary)" : "rgba(255, 255, 255, 0.08)",
                  color: "#fff",
                  fontSize: "0.75rem",
                  cursor: "pointer",
                  whiteSpace: "nowrap"
                }}
              >
                {t.replace("_", " ")}
              </button>
            ))}
          </div>

          {/* Memories List */}
          <div style={{ flex: 1, overflowY: "auto", display: "flex", flexDirection: "column", gap: "0.75rem" }}>
            {filteredMemories.length === 0 ? (
              <p style={{ fontSize: "0.85rem", color: "var(--text-muted)", textAlign: "center", marginTop: "1rem" }}>
                No matching memory records.
              </p>
            ) : (
              filteredMemories.map(m => (
                <div key={m.id} style={{
                  background: "rgba(0,0,0,0.3)",
                  border: "1px solid var(--border-color)",
                  borderRadius: "10px",
                  padding: "0.75rem",
                  fontSize: "0.85rem",
                  display: "flex",
                  flexDirection: "column",
                  gap: "0.4rem"
                }}>
                  <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
                    <span style={{
                      fontWeight: 700,
                      fontSize: "0.75rem",
                      color: "var(--accent)",
                      background: "rgba(168, 85, 247, 0.15)",
                      padding: "0.1rem 0.4rem",
                      borderRadius: "4px"
                    }}>
                      {m.tier}
                    </span>
                    <button
                      onClick={() => insertMemoryLink(m.id)}
                      style={{
                        background: "transparent",
                        border: "none",
                        color: "var(--primary)",
                        fontSize: "0.75rem",
                        cursor: "pointer",
                        fontWeight: 600
                      }}
                    >
                      + Attach
                    </button>
                  </div>
                  <div style={{ fontWeight: 600, color: "var(--text-main)" }}>
                    {m.title || `Memory #${m.id.slice(0, 8)}`}
                  </div>
                  <div style={{ color: "var(--text-muted)", fontSize: "0.8rem", display: "-webkit-box", WebkitLineClamp: 3, WebkitBoxOrient: "vertical", overflow: "hidden" }}>
                    {m.body}
                  </div>
                </div>
              ))
            )}
          </div>
        </div>
  );
}
