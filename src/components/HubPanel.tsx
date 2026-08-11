import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface MemoryRecord {
  id: string;
  scope: string;
  workspace_path?: string | null;
  tier: string;
  agent_id?: string | null;
  title?: string | null;
  body: string;
  tags_json: string;
  created_at: string;
  updated_at: string;
  stale: boolean;
}

interface MessageRecord {
  id: string;
  from_agent: string;
  to_agent: string;
  kind: string;
  status: string;
  subject?: string | null;
  body: string;
  created_at: string;
}

interface WakeRecord {
  id: string;
  target_agent: string;
  reason?: string | null;
  status: string;
  requires_human_gate: boolean;
  created_at: string;
}

interface AgentRecord {
  id: string;
  display_name: string;
}

type HubTab = "memory" | "inbox" | "wakes";

const card: React.CSSProperties = {
  border: "1px solid var(--border-color)",
  borderRadius: "0.5rem",
  padding: "0.75rem 1rem",
  background: "rgba(255,255,255,0.03)",
};

export default function HubPanel() {
  const [hubTab, setHubTab] = useState<HubTab>("memory");
  const [dataDir, setDataDir] = useState<string>("");
  const [error, setError] = useState<string>("");
  const [status, setStatus] = useState<string>("");

  const [memories, setMemories] = useState<MemoryRecord[]>([]);
  const [messages, setMessages] = useState<MessageRecord[]>([]);
  const [wakes, setWakes] = useState<WakeRecord[]>([]);
  const [agents, setAgents] = useState<AgentRecord[]>([]);

  const [searchQ, setSearchQ] = useState("");
  const [tierFilter, setTierFilter] = useState<string>("");
  const [memTitle, setMemTitle] = useState("");
  const [memBody, setMemBody] = useState("");
  const [memTier, setMemTier] = useState("short_term");
  const [memAgent, setMemAgent] = useState("grok");

  const [msgFrom, setMsgFrom] = useState("human");
  const [msgTo, setMsgTo] = useState("claude");
  const [msgBody, setMsgBody] = useState("");
  const [msgKind, setMsgKind] = useState("message");
  const [pollTo, setPollTo] = useState("claude");

  const [wakeTarget, setWakeTarget] = useState("claude");
  const [wakeReason, setWakeReason] = useState("");

  const run = useCallback(async <T,>(label: string, fn: () => Promise<T>): Promise<T | null> => {
    setError("");
    try {
      const result = await fn();
      setStatus(label);
      return result;
    } catch (e) {
      setError(String(e));
      return null;
    }
  }, []);

  const refreshMemories = useCallback(async () => {
    const list = await run("memories refreshed", () =>
      invoke<MemoryRecord[]>("hub_list_memories", {
        scope: null,
        tier: tierFilter || null,
        workspace: null,
        includeStale: false,
      })
    );
    if (list) setMemories(list);
  }, [run, tierFilter]);

  const refreshMessages = useCallback(async () => {
    const list = await run("inbox refreshed", () =>
      invoke<MessageRecord[]>("hub_list_messages", { to: null, status: null })
    );
    if (list) setMessages(list);
  }, [run]);

  const refreshWakes = useCallback(async () => {
    const list = await run("wakes refreshed", () =>
      invoke<WakeRecord[]>("hub_list_wakes", { target: null, pendingOnly: false })
    );
    if (list) setWakes(list);
  }, [run]);

  useEffect(() => {
    (async () => {
      const dir = await run("hub ready", () => invoke<string>("hub_init"));
      if (dir) setDataDir(dir);
      const a = await invoke<AgentRecord[]>("hub_list_agents").catch(() => []);
      setAgents(a);
      await refreshMemories();
      await refreshMessages();
      await refreshWakes();
    })();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    if (hubTab === "memory") void refreshMemories();
    if (hubTab === "inbox") void refreshMessages();
    if (hubTab === "wakes") void refreshWakes();
  }, [hubTab, refreshMemories, refreshMessages, refreshWakes]);

  const writeMemory = async () => {
    if (!memBody.trim()) return;
    await run("memory written", () =>
      invoke("hub_write_memory", {
        args: {
          tier: memTier,
          scope: "global",
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

  const searchMemories = async () => {
    if (!searchQ.trim()) {
      await refreshMemories();
      return;
    }
    const list = await run("search done", () =>
      invoke<MemoryRecord[]>("hub_search_memories", { query: searchQ })
    );
    if (list) setMemories(list);
  };

  const sendMessage = async () => {
    if (!msgBody.trim()) return;
    await run("message sent", () =>
      invoke("hub_send_message", {
        args: {
          from: msgFrom,
          to: msgTo,
          kind: msgKind,
          subject: null,
          workspace: null,
          task: null,
          body: msgBody,
        },
      })
    );
    setMsgBody("");
    await refreshMessages();
  };

  const pollInbox = async () => {
    const list = await run(`polled ${pollTo}`, () =>
      invoke<MessageRecord[]>("hub_poll_messages", { to: pollTo, markAcked: true })
    );
    if (list) {
      await refreshMessages();
      setStatus(`polled ${list.length} for ${pollTo}`);
    }
  };

  const requestWake = async () => {
    await run("wake requested", () =>
      invoke("hub_request_wake", {
        target: wakeTarget,
        reason: wakeReason || null,
        messageId: null,
        humanGate: true,
      })
    );
    setWakeReason("");
    await refreshWakes();
  };

  const tabBtn = (id: HubTab, label: string) => (
    <button
      key={id}
      className={hubTab === id ? "btn-primary" : "btn-secondary"}
      style={{ padding: "0.4rem 0.9rem", fontSize: "0.85rem" }}
      onClick={() => setHubTab(id)}
    >
      {label}
    </button>
  );

  return (
    <div className="glass-card">
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", gap: "1rem", flexWrap: "wrap" }}>
        <h2 style={{ margin: 0 }}>Shared Hub</h2>
        <div style={{ display: "flex", gap: "0.5rem" }}>
          {tabBtn("memory", "Memory")}
          {tabBtn("inbox", "Inbox")}
          {tabBtn("wakes", "Wakes")}
        </div>
      </div>
      <p style={{ fontSize: "0.8rem", color: "var(--text-muted)", marginTop: "0.5rem" }}>
        Data dir: <code>{dataDir || "…"}</code> · same as <code>ca</code> CLI ($CA_HOME)
      </p>
      {error && (
        <p style={{ color: "#f87171", fontSize: "0.85rem" }}>{error}</p>
      )}
      {status && !error && (
        <p style={{ color: "#86efac", fontSize: "0.8rem" }}>{status}</p>
      )}

      {hubTab === "memory" && (
        <div style={{ display: "flex", flexDirection: "column", gap: "1rem", marginTop: "1rem" }}>
          <div style={{ display: "flex", gap: "0.5rem", flexWrap: "wrap" }}>
            <input
              placeholder="Search memories…"
              value={searchQ}
              onChange={(e) => setSearchQ(e.target.value)}
              style={{ flex: 1, minWidth: 160 }}
            />
            <button className="btn-secondary" onClick={searchMemories}>Search</button>
            <select value={tierFilter} onChange={(e) => setTierFilter(e.target.value)}>
              <option value="">All tiers</option>
              <option value="short_term">short_term</option>
              <option value="episodic">episodic</option>
              <option value="semantic">semantic</option>
            </select>
            <button className="btn-secondary" onClick={refreshMemories}>Refresh</button>
            <button
              className="btn-secondary"
              onClick={async () => {
                await run("compacted", () => invoke("hub_compact_short_term", { keepNewest: 20 }));
                await refreshMemories();
              }}
            >
              Compact ST
            </button>
            <button
              className="btn-secondary"
              onClick={async () => {
                const path = await run("exported", () => invoke<string>("hub_export_markdown"));
                if (path) setStatus(`exported → ${path}`);
              }}
            >
              Export MD
            </button>
            <button
              className="btn-secondary"
              title="git add + git commit the export if it's inside a Git work tree"
              onClick={async () => {
                const outcome = await run("export_committed", () =>
                  invoke<{ path: string; committed: boolean; detail: string }>(
                    "hub_export_markdown_git",
                    { message: null },
                  ),
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

          <div style={{ ...card, display: "grid", gap: "0.5rem" }}>
            <strong>Write memory</strong>
            <div style={{ display: "flex", gap: "0.5rem", flexWrap: "wrap" }}>
              <select value={memTier} onChange={(e) => setMemTier(e.target.value)}>
                <option value="short_term">short_term</option>
                <option value="episodic">episodic</option>
                <option value="semantic">semantic</option>
              </select>
              <select value={memAgent} onChange={(e) => setMemAgent(e.target.value)}>
                {agents.map((a) => (
                  <option key={a.id} value={a.id}>{a.display_name}</option>
                ))}
              </select>
              <input
                placeholder="Title"
                value={memTitle}
                onChange={(e) => setMemTitle(e.target.value)}
                style={{ flex: 1, minWidth: 120 }}
              />
            </div>
            <textarea
              rows={3}
              placeholder="Memory body…"
              value={memBody}
              onChange={(e) => setMemBody(e.target.value)}
            />
            <div style={{ display: "flex", justifyContent: "flex-end" }}>
              <button className="btn-primary" onClick={writeMemory} disabled={!memBody.trim()}>
                Save to hub
              </button>
            </div>
          </div>

          <div style={{ display: "flex", flexDirection: "column", gap: "0.6rem", maxHeight: 420, overflowY: "auto" }}>
            {memories.length === 0 && (
              <p style={{ color: "var(--text-muted)", fontSize: "0.9rem" }}>No memories yet.</p>
            )}
            {memories.map((m) => (
              <div key={m.id} style={card}>
                <div style={{ display: "flex", justifyContent: "space-between", gap: "0.5rem", flexWrap: "wrap" }}>
                  <div>
                    <strong>{m.title || "(untitled)"}</strong>{" "}
                    <span style={{ fontSize: "0.75rem", color: "var(--text-muted)" }}>
                      {m.tier} · {m.scope} · {m.agent_id || "—"}
                    </span>
                  </div>
                  <div style={{ display: "flex", gap: "0.35rem" }}>
                    {m.tier === "short_term" && (
                      <button
                        className="btn-secondary"
                        style={{ fontSize: "0.75rem", padding: "0.15rem 0.45rem" }}
                        onClick={async () => {
                          await run("promoted", () =>
                            invoke("hub_promote_memory", { id: m.id, toTier: "episodic" })
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
                        style={{ fontSize: "0.75rem", padding: "0.15rem 0.45rem" }}
                        onClick={async () => {
                          await run("promoted", () =>
                            invoke("hub_promote_memory", { id: m.id, toTier: "semantic" })
                          );
                          await refreshMemories();
                        }}
                      >
                        → semantic
                      </button>
                    )}
                    <button
                      className="btn-secondary"
                      style={{ fontSize: "0.75rem", padding: "0.15rem 0.45rem" }}
                      onClick={async () => {
                        await run("stale", () =>
                          invoke("hub_mark_memory_stale", { id: m.id, stale: true })
                        );
                        await refreshMemories();
                      }}
                    >
                      Stale
                    </button>
                    <button
                      className="btn-secondary"
                      style={{ fontSize: "0.75rem", padding: "0.15rem 0.45rem", color: "#f87171" }}
                      onClick={async () => {
                        await run("deleted", () => invoke("hub_delete_memory", { id: m.id }));
                        await refreshMemories();
                      }}
                    >
                      Delete
                    </button>
                  </div>
                </div>
                <pre style={{ margin: "0.5rem 0 0", whiteSpace: "pre-wrap", fontSize: "0.85rem", color: "var(--text-primary)" }}>
                  {m.body}
                </pre>
                <div style={{ fontSize: "0.7rem", color: "var(--text-muted)" }}>{m.created_at} · {m.id.slice(0, 8)}</div>
              </div>
            ))}
          </div>
        </div>
      )}

      {hubTab === "inbox" && (
        <div style={{ display: "flex", flexDirection: "column", gap: "1rem", marginTop: "1rem" }}>
          <div style={{ ...card, display: "grid", gap: "0.5rem" }}>
            <strong>Send message / handoff</strong>
            <div style={{ display: "flex", gap: "0.5rem", flexWrap: "wrap" }}>
              <select value={msgFrom} onChange={(e) => setMsgFrom(e.target.value)}>
                {agents.map((a) => <option key={a.id} value={a.id}>{a.id}</option>)}
              </select>
              <span style={{ alignSelf: "center" }}>→</span>
              <select value={msgTo} onChange={(e) => setMsgTo(e.target.value)}>
                {agents.map((a) => <option key={a.id} value={a.id}>{a.id}</option>)}
              </select>
              <select value={msgKind} onChange={(e) => setMsgKind(e.target.value)}>
                <option value="message">message</option>
                <option value="handoff">handoff</option>
                <option value="system">system</option>
              </select>
            </div>
            <textarea rows={3} value={msgBody} onChange={(e) => setMsgBody(e.target.value)} placeholder="Body…" />
            <div style={{ display: "flex", gap: "0.5rem", justifyContent: "flex-end" }}>
              <button className="btn-primary" onClick={sendMessage} disabled={!msgBody.trim()}>Send</button>
            </div>
          </div>
          <div style={{ display: "flex", gap: "0.5rem", alignItems: "center" }}>
            <span style={{ fontSize: "0.85rem" }}>Poll for</span>
            <select value={pollTo} onChange={(e) => setPollTo(e.target.value)}>
              {agents.map((a) => <option key={a.id} value={a.id}>{a.id}</option>)}
            </select>
            <button className="btn-secondary" onClick={pollInbox}>Poll (ack)</button>
            <button className="btn-secondary" onClick={refreshMessages}>Refresh</button>
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: "0.6rem", maxHeight: 360, overflowY: "auto" }}>
            {messages.map((m) => (
              <div key={m.id} style={card}>
                <div style={{ fontSize: "0.85rem" }}>
                  <strong>{m.from_agent}</strong> → <strong>{m.to_agent}</strong>{" "}
                  <span style={{ color: "var(--text-muted)" }}>{m.kind} · {m.status}</span>
                </div>
                {m.subject && <div style={{ fontWeight: 600 }}>{m.subject}</div>}
                <pre style={{ margin: "0.4rem 0 0", whiteSpace: "pre-wrap", fontSize: "0.85rem" }}>{m.body}</pre>
              </div>
            ))}
          </div>
        </div>
      )}

      {hubTab === "wakes" && (
        <div style={{ display: "flex", flexDirection: "column", gap: "1rem", marginTop: "1rem" }}>
          <div style={{ ...card, display: "grid", gap: "0.5rem" }}>
            <strong>Request wake</strong>
            <div style={{ display: "flex", gap: "0.5rem", flexWrap: "wrap" }}>
              <select value={wakeTarget} onChange={(e) => setWakeTarget(e.target.value)}>
                {agents.map((a) => <option key={a.id} value={a.id}>{a.id}</option>)}
              </select>
              <input
                style={{ flex: 1, minWidth: 160 }}
                placeholder="Reason"
                value={wakeReason}
                onChange={(e) => setWakeReason(e.target.value)}
              />
              <button className="btn-primary" onClick={requestWake}>Wake (human gate)</button>
              <button className="btn-secondary" onClick={refreshWakes}>Refresh</button>
            </div>
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: "0.6rem", maxHeight: 360, overflowY: "auto" }}>
            {wakes.map((w) => (
              <div key={w.id} style={card}>
                <div>
                  <strong>{w.target_agent}</strong> · {w.status}
                  {w.requires_human_gate && (
                    <span style={{ marginLeft: 8, color: "#eab308", fontSize: "0.8rem" }}>human gate</span>
                  )}
                </div>
                <div style={{ fontSize: "0.85rem", color: "var(--text-muted)" }}>{w.reason || "(no reason)"}</div>
                <div style={{ fontSize: "0.7rem", color: "var(--text-muted)" }}>{w.created_at}</div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
