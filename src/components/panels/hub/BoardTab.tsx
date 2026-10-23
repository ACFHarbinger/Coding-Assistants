import { useCallback, useEffect, useState } from "react";
import { invoke } from "../../../lib/tauri";
import { loadWorkspaceRoot } from "../../../app/hubState";
import { nocPanelStyle, nocCardDenseStyle } from "../../common/telemetry/panel";
import { EmptyStateChip } from "../../common/telemetry/status";
import { NOC_COLORS, NOC_SURFACES } from "../../common/telemetry/tokens";

export interface BoardCard {
  id: string;
  kind: string;
  title: string;
  status: string;
  issue_number: number | null;
  url: string | null;
  labels: string[];
  github_assignees: string[];
  roster_assignees: string[];
  deadline: string | null;
  linked_branches: string[];
  size: string | null;
}

export interface BoardColumn {
  name: string;
  cards: BoardCard[];
}

export interface ProjectBoard {
  owner: string;
  project_number: number;
  columns: BoardColumn[];
  notice: string | null;
}

function assignees(card: BoardCard): string {
  const names = [...card.roster_assignees, ...card.github_assignees];
  return names.length ? names.join(", ") : "unassigned";
}

function getColumnAccent(columnName: string): { border: string; bg: string } {
  const name = columnName.toLowerCase();
  if (name.includes("done") || name.includes("closed")) {
    return { border: NOC_COLORS.ok, bg: "rgba(16, 185, 129, 0.15)" };
  }
  if (name.includes("progress") || name.includes("active") || name.includes("review")) {
    return { border: NOC_COLORS.cyan, bg: "rgba(56, 189, 248, 0.15)" };
  }
  if (name.includes("blocked") || name.includes("urgent")) {
    return { border: NOC_COLORS.crit, bg: "rgba(239, 68, 68, 0.15)" };
  }
  return { border: NOC_COLORS.indigo, bg: "rgba(99, 102, 241, 0.15)" };
}

interface AgentOption {
  id: string;
  display_name: string;
}

export default function BoardTab() {
  const [board, setBoard] = useState<ProjectBoard | null>(null);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [draft, setDraft] = useState("");
  const [draftDeadline, setDraftDeadline] = useState("");
  const [draftAssignee, setDraftAssignee] = useState("human");
  const [agents, setAgents] = useState<AgentOption[]>([]);
  const [deadlineDraft, setDeadlineDraft] = useState<Record<string, string>>({});

  const refresh = useCallback(async () => {
    setBusy(true);
    try {
      const next = await invoke<ProjectBoard>("hub_list_project_board", {
        workspace: loadWorkspaceRoot() || null,
      });
      setBoard(next);
      setError("");
    } catch (cause) {
      setError(String(cause));
    } finally {
      setBusy(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
    void invoke<AgentOption[]>("hub_list_agents")
      .then((list) => {
        if (list.length) setAgents(list);
      })
      .catch(() => undefined);
    const timer = window.setInterval(() => void refresh(), 60_000);
    return () => window.clearInterval(timer);
  }, [refresh]);

  const move = async (card: BoardCard, status: string) => {
    if (status === card.status) return;
    setBusy(true);
    try {
      await invoke("hub_move_board_card", {
        kind: card.kind,
        id: card.id,
        url: card.url,
        status,
      });
      await refresh();
    } catch (cause) {
      setError(String(cause));
      setBusy(false);
    }
  };

  const createInternal = async () => {
    const title = draft.trim();
    if (!title) return;
    setBusy(true);
    try {
      await invoke("hub_create_internal_board_card", {
        title,
        status: "Backlog",
        assignees: [draftAssignee],
        deadline: draftDeadline.trim() || null,
      });
      setDraft("");
      setDraftDeadline("");
      await refresh();
    } catch (cause) {
      setError(String(cause));
      setBusy(false);
    }
  };

  const saveMeta = async (card: BoardCard, rosterAssignees: string[] | null, deadline: string | null) => {
    setBusy(true);
    try {
      await invoke("hub_set_board_overlay", {
        kind: card.kind,
        id: card.id,
        issueNumber: card.issue_number,
        assignees: rosterAssignees,
        deadline,
      });
      await refresh();
    } catch (cause) {
      setError(String(cause));
      setBusy(false);
    }
  };

  const inputStyle: React.CSSProperties = {
    padding: "0.4rem 0.65rem",
    borderRadius: "8px",
    border: "1px solid rgba(148, 163, 184, 0.2)",
    background: "rgba(0,0,0,0.35)",
    color: NOC_SURFACES.textPrimary,
    fontSize: "0.82rem",
    outline: "none",
  };

  return (
    <div className="fade-in" style={{ display: "flex", flexDirection: "column", gap: "1.25rem" }}>
      {/* Top Action & Create Bar */}
      <div style={{ ...nocPanelStyle, display: "flex", gap: "0.65rem", flexWrap: "wrap", alignItems: "center" }}>
        <h3 style={{ margin: 0, fontSize: "1.05rem", fontWeight: 700, color: NOC_SURFACES.textPrimary }}>Board</h3>
        <button
          type="button"
          className="btn-secondary noc-focusable"
          style={{ marginTop: 0, fontSize: "0.78rem", padding: "0.35rem 0.75rem" }}
          disabled={busy}
          onClick={() => void refresh()}
        >
          {busy ? "Refreshing…" : "Refresh"}
        </button>

        <div style={{ display: "flex", gap: "0.5rem", flex: "1 1 320px", flexWrap: "wrap", alignItems: "center" }}>
          <input
            className="noc-focusable"
            value={draft}
            onChange={(event) => setDraft(event.target.value)}
            placeholder="Internal task title…"
            style={{ ...inputStyle, flex: "1 1 180px" }}
          />
          <select
            className="noc-focusable"
            value={draftAssignee}
            onChange={(event) => setDraftAssignee(event.target.value)}
            style={inputStyle}
          >
            {(agents.length ? agents : [{ id: "human", display_name: "Human" }]).map((agent) => (
              <option key={agent.id} value={agent.id}>
                {agent.display_name}
              </option>
            ))}
          </select>
          <input
            type="date"
            className="noc-focusable"
            value={draftDeadline}
            onChange={(event) => setDraftDeadline(event.target.value)}
            style={inputStyle}
          />
          <button
            type="button"
            className="btn-primary noc-focusable"
            style={{ marginTop: 0, fontSize: "0.82rem" }}
            disabled={busy || !draft.trim()}
            onClick={() => void createInternal()}
          >
            Add internal
          </button>
        </div>
      </div>

      {(error || board?.notice) && (
        <div style={{ color: NOC_COLORS.crit, fontSize: "0.82rem", background: "rgba(239, 68, 68, 0.1)", padding: "0.45rem 0.75rem", borderRadius: "8px" }}>
          {error || board?.notice}
        </div>
      )}

      {/* Columns Container */}
      <div style={{ display: "flex", gap: "0.75rem", overflowX: "auto", paddingBottom: "0.5rem", alignItems: "flex-start" }}>
        {(board?.columns ?? []).map((column) => {
          const accent = getColumnAccent(column.name);
          return (
            <div
              key={column.name}
              onDragOver={(event) => event.preventDefault()}
              onDrop={(event) => {
                event.preventDefault();
                const raw = event.dataTransfer.getData("application/json");
                if (!raw) return;
                const card = JSON.parse(raw) as BoardCard;
                void move(card, column.name);
              }}
              style={{
                minWidth: "240px",
                flex: "1 0 240px",
                background: "rgba(2, 6, 23, 0.6)",
                border: "1px solid rgba(148, 163, 184, 0.18)",
                borderRadius: "10px",
                padding: "0.65rem",
              }}
            >
              {/* NOC Column Header */}
              <div
                style={{
                  display: "flex",
                  justifyContent: "space-between",
                  alignItems: "center",
                  marginBottom: "0.65rem",
                  paddingBottom: "0.45rem",
                  borderBottom: `2px solid ${accent.border}`,
                }}
              >
                <span style={{ fontWeight: 700, fontSize: "0.85rem", color: NOC_SURFACES.textPrimary }}>
                  {column.name}
                </span>
                <span
                  className="tabular-nums"
                  style={{
                    fontSize: "0.72rem",
                    fontWeight: 700,
                    padding: "0.1rem 0.45rem",
                    borderRadius: "10px",
                    background: accent.bg,
                    color: accent.border,
                    border: `1px solid ${accent.border}44`,
                  }}
                >
                  {column.cards.length}
                </span>
              </div>

              {column.cards.map((card) => (
                <article
                  key={card.id}
                  draggable
                  onDragStart={(event) => event.dataTransfer.setData("application/json", JSON.stringify(card))}
                  style={{
                    ...nocCardDenseStyle,
                    marginBottom: "0.5rem",
                    cursor: "grab",
                    borderLeft: `3px solid ${accent.border}`,
                    padding: "0.6rem 0.75rem",
                  }}
                >
                  <div style={{ color: NOC_SURFACES.textPrimary, fontSize: "0.86rem", fontWeight: 600 }}>
                    {card.issue_number ? <span style={{ color: NOC_COLORS.cyan }}>#{card.issue_number} </span> : ""}
                    {card.title}
                  </div>
                  <div style={{ color: NOC_SURFACES.textMuted, fontSize: "0.72rem", marginTop: "0.25rem" }}>
                    {assignees(card)}
                    {card.size ? ` · ${card.size}` : ""}
                  </div>
                  {card.labels.length > 0 && (
                    <div style={{ display: "flex", flexWrap: "wrap", gap: "0.25rem", marginTop: "0.35rem" }}>
                      {card.labels.slice(0, 4).map((label) => (
                        <span
                          key={label}
                          style={{
                            fontSize: "0.68rem",
                            padding: "0.08rem 0.4rem",
                            borderRadius: "999px",
                            background: "rgba(56, 189, 248, 0.15)",
                            color: "#a5f3fc",
                            border: "1px solid rgba(56, 189, 248, 0.3)",
                            fontWeight: 600,
                          }}
                        >
                          {label}
                        </span>
                      ))}
                    </div>
                  )}
                  {card.linked_branches.length > 0 && (
                    <div style={{ fontSize: "0.7rem", color: NOC_COLORS.ok, marginTop: "0.25rem", fontFamily: "var(--font-mono)" }}>
                      {card.linked_branches.join(" · ")}
                    </div>
                  )}
                  <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "0.4rem", marginTop: "0.45rem", borderTop: "1px solid rgba(148, 163, 184, 0.1)", paddingTop: "0.35rem" }}>
                    <select
                      className="noc-focusable"
                      value={card.roster_assignees[0] ?? ""}
                      onChange={(event) => void saveMeta(card, event.target.value ? [event.target.value] : [], card.deadline)}
                      style={{ fontSize: "0.72rem", background: "rgba(0,0,0,0.3)", color: NOC_SURFACES.textSecondary, border: "1px solid rgba(148, 163, 184, 0.2)", borderRadius: "4px", padding: "0.15rem 0.3rem" }}
                    >
                      <option value="">assignee: none</option>
                      {(agents.length ? agents : [{ id: "human", display_name: "Human" }]).map((agent) => (
                        <option key={agent.id} value={agent.id}>
                          {agent.display_name}
                        </option>
                      ))}
                    </select>
                    <input
                      type="date"
                      className="noc-focusable"
                      value={deadlineDraft[card.id] ?? card.deadline ?? ""}
                      onChange={(event) => setDeadlineDraft((prev) => ({ ...prev, [card.id]: event.target.value }))}
                      onBlur={(event) => void saveMeta(card, null, event.currentTarget.value.trim() || null)}
                      style={{ fontSize: "0.72rem", background: "rgba(0,0,0,0.3)", color: NOC_SURFACES.textSecondary, border: "1px solid rgba(148, 163, 184, 0.2)", borderRadius: "4px", padding: "0.15rem 0.3rem" }}
                    />
                  </div>
                </article>
              ))}
            </div>
          );
        })}
        {board && board.columns.length === 0 && (
          <EmptyStateChip title="No board columns found" detail="No project board configured for this repository." tone="idle" />
        )}
      </div>
    </div>
  );
}
