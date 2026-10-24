import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "../../../lib/tauri";
import { loadWorkspaceRoot } from "../../../app/hubState";
import { nocPanelStyle, nocCardDenseStyle } from "../../common/telemetry/panel";
import { EmptyStateChip, KpiCardGrid, MonitorTileStrip, StalenessDot, type KpiCardItem, type MonitorTileItem } from "../../common/telemetry/status";
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

function getColumnAccent(columnName: string): { border: string; bg: string; kind: "done" | "active" | "blocked" | "todo" } {
  const name = columnName.toLowerCase();
  if (name.includes("done") || name.includes("closed") || name.includes("complete")) {
    return { border: NOC_COLORS.ok, bg: "rgba(16, 185, 129, 0.15)", kind: "done" };
  }
  if (name.includes("progress") || name.includes("active") || name.includes("review") || name.includes("doing")) {
    return { border: NOC_COLORS.cyan, bg: "rgba(56, 189, 248, 0.15)", kind: "active" };
  }
  if (name.includes("blocked") || name.includes("urgent")) {
    return { border: NOC_COLORS.crit, bg: "rgba(239, 68, 68, 0.15)", kind: "blocked" };
  }
  return { border: NOC_COLORS.indigo, bg: "rgba(99, 102, 241, 0.15)", kind: "todo" };
}

function isOverdue(deadline: string | null): boolean {
  if (!deadline) return false;
  const parsed = Date.parse(deadline);
  if (Number.isNaN(parsed)) return false;
  const todayStart = new Date();
  todayStart.setHours(0, 0, 0, 0);
  return parsed < todayStart.getTime();
}

interface AgentOption {
  id: string;
  display_name: string;
}

export default function BoardTab() {
  const [board, setBoard] = useState<ProjectBoard | null>(null);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [lastUpdatedSec, setLastUpdatedSec] = useState<number | null>(null);
  const [filterText, setFilterText] = useState("");
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
      setBoard((prev) => {
        if (next.columns.length === 0 && prev && prev.columns.length > 0 && next.notice) {
          return { ...prev, notice: next.notice };
        }
        return next;
      });
      setLastUpdatedSec(Math.floor(Date.now() / 1000));
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
      await invoke("hub_move_board_card", { kind: card.kind, id: card.id, url: card.url, status });
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

  const { kpis, columnTiles, totalCards } = useMemo(() => {
    const cols = board?.columns ?? [];
    let total = 0;
    let activeCount = 0;
    let doneCount = 0;
    let blockedOrOverdue = 0;
    const tiles: MonitorTileItem[] = [];

    for (const col of cols) {
      const accent = getColumnAccent(col.name);
      const count = col.cards.length;
      total += count;
      if (accent.kind === "active") activeCount += count;
      if (accent.kind === "done") doneCount += count;
      for (const c of col.cards) {
        if (accent.kind === "blocked" || (accent.kind !== "done" && isOverdue(c.deadline))) {
          blockedOrOverdue += 1;
        }
      }
      tiles.push({
        id: col.name,
        title: col.name,
        subtitle: `${count} card${count === 1 ? "" : "s"}`,
        tone: accent.kind === "blocked" ? "crit" : accent.kind === "active" ? "warn" : count > 0 ? "ok" : "idle",
      });
    }

    const donePct = total > 0 ? Math.round((doneCount / total) * 100) : 0;
    const cards: KpiCardItem[] = [
      { title: "Total Cards", subtitle: "All columns", value: String(total), footer: `${cols.length} columns`, accent: NOC_COLORS.indigo },
      { title: "Active / In Progress", subtitle: "Current sprint", value: String(activeCount), footer: "In progress & review", accent: NOC_COLORS.cyan },
      { title: "Completed", subtitle: "Closed / Done", value: String(doneCount), footer: `${donePct}% of board`, accent: NOC_COLORS.ok },
      {
        title: "Blocked / Overdue",
        subtitle: "Risk monitor",
        value: String(blockedOrOverdue),
        footer: blockedOrOverdue > 0 ? "Needs attention" : "All on schedule",
        accent: blockedOrOverdue > 0 ? NOC_COLORS.crit : NOC_COLORS.idle,
      },
    ];

    return { kpis: cards, columnTiles: tiles, totalCards: total };
  }, [board]);

  const normalizedFilter = filterText.trim().toLowerCase();
  const columnNames = useMemo(() => (board?.columns ?? []).map((c) => c.name), [board]);
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
    <div className="fade-in" style={{ display: "flex", flexDirection: "column", gap: "1rem" }}>
      <div style={{ ...nocPanelStyle, display: "flex", flexDirection: "column", gap: "0.75rem" }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", gap: "0.75rem", flexWrap: "wrap" }}>
          <div style={{ display: "flex", alignItems: "center", gap: "0.75rem", flexWrap: "wrap" }}>
            <h3 style={{ margin: 0, fontSize: "1.05rem", fontWeight: 700, color: NOC_SURFACES.textPrimary }}>Board</h3>
            {board && (
              <span style={{ fontSize: "0.74rem", color: NOC_SURFACES.textMuted, fontFamily: "var(--font-mono)" }}>
                {board.owner} · project #{board.project_number}
              </span>
            )}
            <StalenessDot lastUpdatedUnixSeconds={lastUpdatedSec} pollIntervalSeconds={60} />
          </div>
          <div style={{ display: "flex", gap: "0.5rem", alignItems: "center", flexWrap: "wrap" }}>
            <input
              type="search"
              className="noc-focusable"
              value={filterText}
              onChange={(event) => setFilterText(event.target.value)}
              placeholder="Filter cards (#issue, title, assignee, label)…"
              style={{ ...inputStyle, minWidth: "230px" }}
            />
            <button
              type="button"
              className="btn-secondary noc-focusable"
              style={{ marginTop: 0, fontSize: "0.78rem", padding: "0.38rem 0.75rem" }}
              disabled={busy}
              onClick={() => void refresh()}
            >
              {busy ? "Refreshing…" : "Refresh"}
            </button>
          </div>
        </div>

        <div style={{ display: "flex", gap: "0.5rem", flexWrap: "wrap", alignItems: "center", paddingTop: "0.55rem", borderTop: "1px solid rgba(148, 163, 184, 0.12)" }}>
          <span style={{ fontSize: "0.72rem", fontWeight: 700, color: NOC_SURFACES.textMuted, textTransform: "uppercase", letterSpacing: "0.05em" }}>
            Quick Task
          </span>
          <input
            className="noc-focusable"
            value={draft}
            onChange={(event) => setDraft(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter" && draft.trim() && !busy) void createInternal();
            }}
            placeholder="Internal task title…"
            style={{ ...inputStyle, flex: "1 1 200px" }}
          />
          <select className="noc-focusable" value={draftAssignee} onChange={(event) => setDraftAssignee(event.target.value)} style={inputStyle}>
            {(agents.length ? agents : [{ id: "human", display_name: "Human" }]).map((agent) => (
              <option key={agent.id} value={agent.id}>{agent.display_name}</option>
            ))}
          </select>
          <input type="date" className="noc-focusable" value={draftDeadline} onChange={(event) => setDraftDeadline(event.target.value)} style={inputStyle} />
          <button
            type="button"
            className="btn-primary noc-focusable"
            style={{ marginTop: 0, fontSize: "0.8rem", padding: "0.4rem 0.85rem" }}
            disabled={busy || !draft.trim()}
            onClick={() => void createInternal()}
          >
            Add internal
          </button>
        </div>
      </div>

      {board && board.columns.length > 0 && (
        <>
          <KpiCardGrid cards={kpis} />
          <MonitorTileStrip title="Column Distribution" tiles={columnTiles} legendLabels={{ ok: "Has cards", warn: "In progress", crit: "Blocked" }} />
        </>
      )}

      {(error || board?.notice) && (
        <div style={{ color: error ? NOC_COLORS.crit : NOC_COLORS.amber, fontSize: "0.82rem", background: error ? "rgba(239, 68, 68, 0.1)" : "rgba(245, 158, 11, 0.1)", border: `1px solid ${error ? "rgba(239, 68, 68, 0.3)" : "rgba(245, 158, 11, 0.3)"}`, padding: "0.5rem 0.8rem", borderRadius: "8px" }}>
          {error || board?.notice}
        </div>
      )}

      <div style={{ display: "flex", gap: "0.8rem", overflowX: "auto", paddingBottom: "0.5rem", alignItems: "flex-start" }}>
        {(board?.columns ?? []).map((column) => {
          const accent = getColumnAccent(column.name);
          const filteredCards = normalizedFilter
            ? column.cards.filter((card) =>
                [card.title, card.issue_number ? `#${card.issue_number}` : "", assignees(card), ...card.labels, ...card.linked_branches]
                  .join(" ")
                  .toLowerCase()
                  .includes(normalizedFilter)
              )
            : column.cards;
          const sharePct = totalCards > 0 ? Math.round((column.cards.length / totalCards) * 100) : 0;

          return (
            <div
              key={column.name}
              onDragOver={(event) => event.preventDefault()}
              onDrop={(event) => {
                event.preventDefault();
                const raw = event.dataTransfer.getData("application/json");
                if (!raw) return;
                void move(JSON.parse(raw) as BoardCard, column.name);
              }}
              style={{
                minWidth: "265px",
                flex: "1 0 265px",
                background: "rgba(9, 14, 26, 0.78)",
                border: "1px solid rgba(148, 163, 184, 0.16)",
                borderRadius: "10px",
                padding: "0.75rem",
              }}
            >
              <div style={{ marginBottom: "0.65rem" }}>
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "0.35rem" }}>
                  <span style={{ fontWeight: 700, fontSize: "0.86rem", color: NOC_SURFACES.textPrimary }}>{column.name}</span>
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
                    {filteredCards.length}
                  </span>
                </div>
                <div style={{ height: 3, borderRadius: 2, background: "rgba(148, 163, 184, 0.15)", overflow: "hidden" }}>
                  <div style={{ width: `${Math.max(column.cards.length > 0 ? 8 : 0, sharePct)}%`, height: "100%", background: accent.border }} />
                </div>
              </div>

              {filteredCards.length === 0 && (
                <div style={{ padding: "0.85rem 0.5rem", textAlign: "center", fontSize: "0.75rem", color: NOC_SURFACES.textMuted, border: "1px dashed rgba(148, 163, 184, 0.14)", borderRadius: "8px" }}>
                  {normalizedFilter ? "No matching cards" : "No cards in column"}
                </div>
              )}

              {filteredCards.map((card) => {
                const cardOverdue = accent.kind !== "done" && isOverdue(card.deadline);
                return (
                  <article
                    key={card.id}
                    draggable
                    onDragStart={(event) => event.dataTransfer.setData("application/json", JSON.stringify(card))}
                    style={{
                      ...nocCardDenseStyle,
                      marginBottom: "0.55rem",
                      cursor: "grab",
                      borderLeft: `3px solid ${cardOverdue ? NOC_COLORS.crit : accent.border}`,
                      padding: "0.65rem 0.75rem",
                    }}
                  >
                    <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start", gap: "0.4rem" }}>
                      <div style={{ color: NOC_SURFACES.textPrimary, fontSize: "0.85rem", fontWeight: 600, lineHeight: 1.35 }}>
                        {card.issue_number ? <span style={{ color: NOC_COLORS.cyan }}>#{card.issue_number} </span> : null}
                        {card.title}
                      </div>
                      <span
                        style={{
                          fontSize: "0.64rem",
                          fontWeight: 700,
                          textTransform: "uppercase",
                          padding: "0.08rem 0.35rem",
                          borderRadius: "4px",
                          background: card.kind === "internal" ? "rgba(168, 85, 247, 0.16)" : "rgba(56, 189, 248, 0.14)",
                          color: card.kind === "internal" ? NOC_COLORS.purple : NOC_COLORS.cyan,
                          flexShrink: 0,
                        }}
                      >
                        {card.kind === "internal" ? "hub" : "gh"}
                      </span>
                    </div>

                    <div style={{ display: "flex", alignItems: "center", gap: "0.4rem", flexWrap: "wrap", color: NOC_SURFACES.textMuted, fontSize: "0.72rem", marginTop: "0.3rem" }}>
                      <span>👤 {assignees(card)}</span>
                      {card.size && (
                        <span style={{ padding: "0.05rem 0.35rem", borderRadius: "4px", background: "rgba(148, 163, 184, 0.14)", color: NOC_SURFACES.textSecondary, fontWeight: 700 }}>
                          {card.size}
                        </span>
                      )}
                      {card.deadline && (
                        <span
                          className="tabular-nums"
                          style={{
                            padding: "0.05rem 0.35rem",
                            borderRadius: "4px",
                            background: cardOverdue ? "rgba(239, 68, 68, 0.16)" : "rgba(16, 185, 129, 0.12)",
                            color: cardOverdue ? NOC_COLORS.crit : NOC_COLORS.ok,
                            fontWeight: 600,
                          }}
                        >
                          {cardOverdue ? "⚠ " : ""}due {card.deadline}
                        </span>
                      )}
                    </div>

                    {card.labels.length > 0 && (
                      <div style={{ display: "flex", flexWrap: "wrap", gap: "0.25rem", marginTop: "0.35rem" }}>
                        {card.labels.slice(0, 4).map((label) => (
                          <span
                            key={label}
                            style={{
                              fontSize: "0.66rem",
                              padding: "0.08rem 0.4rem",
                              borderRadius: "999px",
                              background: "rgba(56, 189, 248, 0.14)",
                              color: "#a5f3fc",
                              border: "1px solid rgba(56, 189, 248, 0.28)",
                              fontWeight: 600,
                            }}
                          >
                            {label}
                          </span>
                        ))}
                      </div>
                    )}

                    {card.linked_branches.length > 0 && (
                      <div style={{ fontSize: "0.68rem", color: NOC_COLORS.ok, marginTop: "0.3rem", fontFamily: "var(--font-mono)" }}>
                        ⎇ {card.linked_branches.join(" · ")}
                      </div>
                    )}

                    <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "0.35rem", marginTop: "0.45rem", borderTop: "1px solid rgba(148, 163, 184, 0.1)", paddingTop: "0.35rem" }}>
                      <select
                        className="noc-focusable"
                        aria-label={`Assignee for ${card.title}`}
                        value={card.roster_assignees[0] ?? ""}
                        onChange={(event) => void saveMeta(card, event.target.value ? [event.target.value] : [], card.deadline)}
                        style={{ fontSize: "0.7rem", background: "rgba(0,0,0,0.3)", color: NOC_SURFACES.textSecondary, border: "1px solid rgba(148, 163, 184, 0.2)", borderRadius: "4px", padding: "0.15rem 0.25rem" }}
                      >
                        <option value="">assignee: none</option>
                        {(agents.length ? agents : [{ id: "human", display_name: "Human" }]).map((agent) => (
                          <option key={agent.id} value={agent.id}>{agent.display_name}</option>
                        ))}
                      </select>
                      <input
                        type="date"
                        className="noc-focusable"
                        aria-label={`Deadline for ${card.title}`}
                        value={deadlineDraft[card.id] ?? card.deadline ?? ""}
                        onChange={(event) => setDeadlineDraft((prev) => ({ ...prev, [card.id]: event.target.value }))}
                        onBlur={(event) => void saveMeta(card, null, event.currentTarget.value.trim() || null)}
                        style={{ fontSize: "0.7rem", background: "rgba(0,0,0,0.3)", color: NOC_SURFACES.textSecondary, border: "1px solid rgba(148, 163, 184, 0.2)", borderRadius: "4px", padding: "0.15rem 0.25rem" }}
                      />
                    </div>
                    {columnNames.length > 1 && (
                      <select
                        className="noc-focusable"
                        aria-label={`Move ${card.title} to column`}
                        value={column.name}
                        onChange={(event) => void move(card, event.target.value)}
                        style={{ width: "100%", marginTop: "0.3rem", fontSize: "0.7rem", background: "rgba(0,0,0,0.28)", color: NOC_SURFACES.textMuted, border: "1px solid rgba(148, 163, 184, 0.16)", borderRadius: "4px", padding: "0.15rem 0.25rem" }}
                      >
                        {columnNames.map((colName) => (
                          <option key={colName} value={colName}>Status: {colName}</option>
                        ))}
                      </select>
                    )}
                  </article>
                );
              })}
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
