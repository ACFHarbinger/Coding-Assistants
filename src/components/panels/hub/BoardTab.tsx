import { useCallback, useEffect, useState } from "react";
import { invoke } from "../../../lib/tauri";
import { loadWorkspaceRoot } from "../../../app/hubState";

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

  const saveMeta = async (card: BoardCard, assignees: string[] | null, deadline: string | null) => {
    setBusy(true);
    try {
      await invoke("hub_set_board_overlay", {
        kind: card.kind,
        id: card.id,
        issueNumber: card.issue_number,
        assignees,
        deadline,
      });
      await refresh();
    } catch (cause) {
      setError(String(cause));
      setBusy(false);
    }
  };

  return (
    <div>
      <div style={{ display: "flex", gap: "0.5rem", flexWrap: "wrap", marginBottom: "0.85rem", alignItems: "center" }}>
        <strong style={{ color: "var(--text-main)" }}>Board</strong>
        <button type="button" className="btn-secondary" style={{ marginTop: 0 }} disabled={busy} onClick={() => void refresh()}>
          {busy ? "Refreshing…" : "Refresh"}
        </button>
        <input
          value={draft}
          onChange={(event) => setDraft(event.target.value)}
          placeholder="Internal task title"
          style={{ flex: "1 1 160px", padding: "0.4rem 0.55rem", borderRadius: "8px", border: "1px solid var(--border-color)", background: "rgba(0,0,0,0.3)", color: "white" }}
        />
        <select
          value={draftAssignee}
          onChange={(event) => setDraftAssignee(event.target.value)}
          style={{ padding: "0.4rem 0.55rem", borderRadius: "8px", border: "1px solid var(--border-color)", background: "rgba(0,0,0,0.3)", color: "white" }}
        >
          {(agents.length ? agents : [{ id: "human", display_name: "Human" }]).map((agent) => (
            <option key={agent.id} value={agent.id}>{agent.display_name}</option>
          ))}
        </select>
        <input
          type="date"
          value={draftDeadline}
          onChange={(event) => setDraftDeadline(event.target.value)}
          style={{ padding: "0.4rem 0.55rem", borderRadius: "8px", border: "1px solid var(--border-color)", background: "rgba(0,0,0,0.3)", color: "white" }}
        />
        <button type="button" className="btn-primary" style={{ marginTop: 0 }} disabled={busy} onClick={() => void createInternal()}>
          Add internal
        </button>
      </div>
      {(error || board?.notice) && (
        <div style={{ marginBottom: "0.75rem", color: "#fecaca", fontSize: "0.82rem" }}>
          {error || board?.notice}
        </div>
      )}
      <div style={{ display: "flex", gap: "0.65rem", overflowX: "auto", paddingBottom: "0.5rem", alignItems: "flex-start" }}>
        {(board?.columns ?? []).map((column) => (
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
            style={{ minWidth: "220px", flex: "1 0 220px", background: "rgba(0,0,0,0.22)", border: "1px solid var(--border-color)", borderRadius: "10px", padding: "0.55rem" }}
          >
            <div style={{ display: "flex", justifyContent: "space-between", marginBottom: "0.45rem", color: "var(--text-muted)", fontSize: "0.78rem" }}>
              <span>{column.name}</span>
              <span>{column.cards.length}</span>
            </div>
            {column.cards.map((card) => (
              <article
                key={card.id}
                draggable
                onDragStart={(event) => event.dataTransfer.setData("application/json", JSON.stringify(card))}
                style={{ marginBottom: "0.45rem", padding: "0.5rem", borderRadius: "8px", background: "rgba(15,23,42,0.7)", border: "1px solid rgba(148,163,184,0.25)", cursor: "grab" }}
              >
                <div style={{ color: "#fff", fontSize: "0.84rem", fontWeight: 600 }}>
                  {card.issue_number ? `#${card.issue_number} ` : ""}
                  {card.title}
                </div>
                <div style={{ color: "var(--text-muted)", fontSize: "0.72rem", marginTop: "0.25rem" }}>
                  {assignees(card)}
                  {card.size ? ` · ${card.size}` : ""}
                </div>
                {card.labels.length > 0 && (
                  <div style={{ display: "flex", flexWrap: "wrap", gap: "0.25rem", marginTop: "0.3rem" }}>
                    {card.labels.slice(0, 4).map((label) => (
                      <span key={label} style={{ fontSize: "0.68rem", padding: "0.05rem 0.35rem", borderRadius: "999px", background: "rgba(6,182,212,0.18)", color: "#a5f3fc" }}>{label}</span>
                    ))}
                  </div>
                )}
                {card.linked_branches.length > 0 && (
                  <div style={{ fontSize: "0.7rem", color: "#bbf7d0", marginTop: "0.25rem" }}>
                    {card.linked_branches.join(" · ")}
                  </div>
                )}
                <select
                  value={card.roster_assignees[0] ?? ""}
                  onChange={(event) => void saveMeta(card, event.target.value ? [event.target.value] : [], card.deadline)}
                  style={{ marginTop: "0.35rem", width: "100%", fontSize: "0.72rem", background: "transparent", color: "var(--text-muted)", border: "none" }}
                >
                  <option value="">roster: none</option>
                  {(agents.length ? agents : [{ id: "human", display_name: "Human" }]).map((agent) => (
                    <option key={agent.id} value={agent.id}>{agent.display_name}</option>
                  ))}
                </select>
                <input
                  type="date"
                  value={deadlineDraft[card.id] ?? card.deadline ?? ""}
                  onChange={(event) => setDeadlineDraft((prev) => ({ ...prev, [card.id]: event.target.value }))}
                  onBlur={(event) => void saveMeta(card, null, event.currentTarget.value.trim() || null)}
                  style={{ marginTop: "0.2rem", width: "100%", fontSize: "0.72rem", background: "transparent", color: "var(--text-muted)", border: "none" }}
                />
              </article>
            ))}
          </div>
        ))}
      </div>
    </div>
  );
}
