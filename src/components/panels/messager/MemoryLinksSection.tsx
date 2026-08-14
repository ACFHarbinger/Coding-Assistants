// @ts-nocheck
import { useEffect, useState } from "react";
import { invoke } from "../../../lib/tauri";
import type { MemoryLinkRecord, LinkSuggestion } from "./types";

// ─── Styles ──────────────────────────────────────────────────────────────────

const sectionHeaderStyle = {
  margin: 0,
  fontSize: "0.9rem",
  fontWeight: 700,
  color: "var(--primary)",
};

const rowStyle = {
  background: "rgba(0,0,0,0.3)",
  border: "1px solid var(--border-color)",
  borderRadius: "10px",
  padding: "0.65rem 0.75rem",
  fontSize: "0.82rem",
  display: "flex",
  alignItems: "flex-start",
  justifyContent: "space-between",
  gap: "0.5rem",
};

const pillStyle = (accent) => ({
  display: "inline-block",
  padding: "0.1rem 0.45rem",
  borderRadius: "5px",
  fontSize: "0.72rem",
  fontWeight: 700,
  background: accent
    ? "rgba(168, 85, 247, 0.18)"
    : "rgba(255,255,255,0.08)",
  color: accent ? "var(--accent)" : "var(--text-muted)",
  whiteSpace: "nowrap",
});

const mutedStyle = {
  fontSize: "0.78rem",
  color: "var(--text-muted)",
};

const btnStyle = (variant = "secondary") => ({
  padding: "0.22rem 0.6rem",
  borderRadius: "6px",
  border: "none",
  fontSize: "0.75rem",
  fontWeight: 600,
  cursor: "pointer",
  whiteSpace: "nowrap",
  background:
    variant === "primary"
      ? "var(--primary)"
      : variant === "danger"
      ? "rgba(239, 68, 68, 0.2)"
      : "rgba(255,255,255,0.08)",
  color: variant === "danger" ? "#f87171" : "#fff",
});

const emptyStyle = {
  fontSize: "0.82rem",
  color: "var(--text-muted)",
  textAlign: "center",
  padding: "0.5rem 0",
};

const loadingStyle = {
  fontSize: "0.8rem",
  color: "var(--text-muted)",
  fontStyle: "italic",
};

// ─── Helpers ──────────────────────────────────────────────────────────────────

function shortId(id) {
  return "#" + id.slice(0, 8);
}

function formatScore(score) {
  return "(" + Math.round(score * 100) + "%)";
}

// ─── LinkRow ──────────────────────────────────────────────────────────────────

function LinkRow({ link, onUnlink }) {
  const [busy, setBusy] = useState(false);

  const handleUnlink = async () => {
    setBusy(true);
    try {
      await invoke("hub_unlink_memories", { linkId: link.id });
      onUnlink();
    } catch (err) {
      console.error("[MemoryLinksSection] hub_unlink_memories failed:", err);
    } finally {
      setBusy(false);
    }
  };

  const targetId = shortId(link.to_memory_id);
  const label = link.relation
    ? "\u2192 " + targetId + " (" + link.relation + ")"
    : "\u2192 " + targetId;

  return (
    <div style={rowStyle}>
      <div style={{ display: "flex", flexDirection: "column", gap: "0.25rem", flex: 1, minWidth: 0 }}>
        <span style={{ color: "var(--text-main)", fontWeight: 600, fontSize: "0.83rem" }}>
          {label}
        </span>
        <span style={mutedStyle}>by {link.created_by}</span>
      </div>
      <button
        style={btnStyle("danger")}
        disabled={busy}
        onClick={handleUnlink}
        title="Remove this memory link"
      >
        {busy ? "\u2026" : "Unlink"}
      </button>
    </div>
  );
}

// ─── SuggestionRow ────────────────────────────────────────────────────────────

function SuggestionRow({ suggestion, memoryId, createdBy, onLinked }) {
  const [busy, setBusy] = useState(false);
  const [done, setDone] = useState(false);

  const handleLink = async () => {
    setBusy(true);
    try {
      await invoke("hub_link_memories", {
        args: {
          fromMemoryId: memoryId,
          toMemoryId: suggestion.candidate.id,
          relation: null,
          createdBy,
        },
      });
      setDone(true);
      onLinked();
    } catch (err) {
      console.error("[MemoryLinksSection] hub_link_memories failed:", err);
    } finally {
      setBusy(false);
    }
  };

  const title =
    suggestion.candidate.title || "Memory " + shortId(suggestion.candidate.id);

  return (
    <div style={rowStyle}>
      <div style={{ display: "flex", flexDirection: "column", gap: "0.3rem", flex: 1, minWidth: 0 }}>
        <div style={{ display: "flex", alignItems: "center", gap: "0.4rem", flexWrap: "wrap" }}>
          <span style={{ color: "var(--text-main)", fontWeight: 600, fontSize: "0.83rem" }}>
            {title}
          </span>
          <span style={pillStyle(true)}>{formatScore(suggestion.score)}</span>
        </div>
        <span style={mutedStyle}>{suggestion.reason}</span>
        <span style={{ ...mutedStyle, fontSize: "0.73rem" }}>
          {shortId(suggestion.candidate.id)} &middot; {suggestion.candidate.tier}
        </span>
      </div>
      <button
        style={btnStyle(done ? "secondary" : "primary")}
        disabled={busy || done}
        onClick={handleLink}
        title="Create link to this memory"
      >
        {done ? "Linked \u2713" : busy ? "\u2026" : "Link"}
      </button>
    </div>
  );
}

// ─── Main component ───────────────────────────────────────────────────────────

interface MemoryLinksSectionProps {
  memoryId: string;
  currentAgentId?: string;
}

export default function MemoryLinksSection({ memoryId, currentAgentId }: MemoryLinksSectionProps) {
  const [links, setLinks] = useState([]);
  const [linksLoading, setLinksLoading] = useState(false);

  const [suggestions, setSuggestions] = useState([]);
  const [suggestLoading, setSuggestLoading] = useState(false);
  const [suggestOpen, setSuggestOpen] = useState(false);

  const createdBy = currentAgentId ?? "human";

  // ── Fetch existing links ─────────────────────────────────────────────────
  const fetchLinks = async () => {
    if (!memoryId) return;
    setLinksLoading(true);
    try {
      const result = await invoke("hub_list_memory_links", { memoryId });
      setLinks(Array.isArray(result) ? result : []);
    } catch (err) {
      console.error("[MemoryLinksSection] hub_list_memory_links failed:", err);
      setLinks([]);
    } finally {
      setLinksLoading(false);
    }
  };

  useEffect(() => {
    setSuggestions([]);
    setSuggestOpen(false);
    void fetchLinks();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [memoryId]);

  // ── Suggest related memories ─────────────────────────────────────────────
  const handleSuggest = async () => {
    setSuggestLoading(true);
    setSuggestOpen(true);
    setSuggestions([]);
    try {
      const result = await invoke("hub_suggest_links_for_memory", { memoryId, limit: 10 });
      setSuggestions(Array.isArray(result) ? result : []);
    } catch (err) {
      console.error("[MemoryLinksSection] hub_suggest_links_for_memory failed:", err);
      setSuggestions([]);
    } finally {
      setSuggestLoading(false);
    }
  };

  // After linking a suggestion, refresh the links list but keep the panel
  // open so the user can link multiple in one pass.
  const handleLinked = () => {
    void fetchLinks();
  };

  // ── Render ───────────────────────────────────────────────────────────────
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "0.75rem" }}>

      {/* Section header */}
      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: "0.5rem" }}>
        <h4 style={sectionHeaderStyle}>\uD83D\uDD17 Memory Links</h4>
        <button
          style={btnStyle("primary")}
          onClick={handleSuggest}
          disabled={suggestLoading}
          title="Find memories related to this one"
        >
          {suggestLoading ? "Searching\u2026" : "Find related"}
        </button>
      </div>

      {/* Existing links */}
      <div style={{ display: "flex", flexDirection: "column", gap: "0.45rem" }}>
        {linksLoading ? (
          <p style={loadingStyle}>Loading links\u2026</p>
        ) : links.length === 0 ? (
          <p style={emptyStyle}>No links yet.</p>
        ) : (
          links.map((link) => (
            <LinkRow
              key={link.id}
              link={link}
              onUnlink={() => void fetchLinks()}
            />
          ))
        )}
      </div>

      {/* Suggestions panel */}
      {suggestOpen && (
        <div style={{ display: "flex", flexDirection: "column", gap: "0.45rem", borderTop: "1px solid var(--border-color)", paddingTop: "0.65rem" }}>
          <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
            <span style={{ fontSize: "0.8rem", fontWeight: 600, color: "var(--text-muted)" }}>
              Related memories
            </span>
            <button
              style={{ background: "transparent", border: "none", color: "var(--text-muted)", fontSize: "0.75rem", cursor: "pointer", padding: "0.1rem 0.25rem" }}
              onClick={() => setSuggestOpen(false)}
              title="Dismiss suggestions"
            >
              \u2715
            </button>
          </div>

          {suggestLoading ? (
            <p style={loadingStyle}>Searching for related memories\u2026</p>
          ) : suggestions.length === 0 ? (
            <p style={emptyStyle}>No related memories found.</p>
          ) : (
            suggestions.map((s) => (
              <SuggestionRow
                key={s.candidate.id}
                suggestion={s}
                memoryId={memoryId}
                createdBy={createdBy}
                onLinked={handleLinked}
              />
            ))
          )}
        </div>
      )}
    </div>
  );
}
