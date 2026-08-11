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

interface WakePolicy {
  default_requires_human_gate: boolean;
  allow_auto_wake: boolean;
}

type HubTab = "memory" | "inbox" | "wakes" | "policy";

const cardStyle: React.CSSProperties = {
  border: "1px solid var(--border-color)",
  borderRadius: "12px",
  padding: "1.5rem",
  background: "rgba(0, 0, 0, 0.3)",
  boxShadow: "0 4px 6px rgba(0,0,0,0.1)",
  transition: "all 0.2s ease"
};

const inputStyle: React.CSSProperties = {
  padding: '0.75rem',
  borderRadius: '8px',
  background: 'rgba(0,0,0,0.4)',
  color: 'white',
  border: '1px solid var(--border-color)',
  outline: 'none',
  transition: 'border-color 0.2s'
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

  const [wakePolicy, setWakePolicy] = useState<WakePolicy | null>(null);

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
    const list = await run("inbox refreshed", () => invoke<MessageRecord[]>("hub_list_messages"));
    if (list) setMessages(list);
  }, [run]);

  const refreshWakes = useCallback(async () => {
    const list = await run("wakes refreshed", () => invoke<WakeRecord[]>("hub_list_wakes"));
    if (list) setWakes(list);
  }, [run]);

  const refreshPolicy = useCallback(async () => {
    const policy = await run("policy refreshed", () => invoke<WakePolicy>("hub_get_wake_policy"));
    if (policy) setWakePolicy(policy);
  }, [run]);

  useEffect(() => {
    invoke<string>("hub_get_data_dir").then(setDataDir).catch((e) => setError(String(e)));
    invoke<AgentRecord[]>("hub_list_agents").then((list) => {
      setAgents(list);
      if (list.length > 0) {
        setMemAgent(list[0].id);
        setMsgFrom(list[0].id);
        setMsgTo(list[0].id);
        setPollTo(list[0].id);
        setWakeTarget(list[0].id);
      }
    }).catch((e) => setError(String(e)));
  }, []);

  useEffect(() => {
    if (hubTab === "memory") refreshMemories();
    else if (hubTab === "inbox") refreshMessages();
    else if (hubTab === "wakes") refreshWakes();
    else if (hubTab === "policy") refreshPolicy();
  }, [hubTab, refreshMemories, refreshMessages, refreshWakes, refreshPolicy]);

  const writeMemory = async () => {
    if (!memBody.trim()) return;
    await run("memory written", () =>
      invoke("hub_write_memory", {
        tier: memTier,
        agentId: memAgent,
        title: memTitle || null,
        body: memBody,
        tags: [],
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

  const updatePolicy = async (updates: Partial<WakePolicy>) => {
    if (!wakePolicy) return;
    const newPolicy = { ...wakePolicy, ...updates };
    await run("policy updated", () => invoke("hub_set_wake_policy", { policy: newPolicy }));
    setWakePolicy(newPolicy);
  };

  const tabBtn = (id: HubTab, label: string) => (
    <button
      key={id}
      className={hubTab === id ? "btn-primary" : "btn-secondary"}
      style={{ padding: "0.5rem 1rem", fontSize: "0.9rem", borderRadius: "8px", transition: "all 0.2s ease" }}
      onClick={() => setHubTab(id)}
    >
      {label}
    </button>
  );

  return (
    <div className="glass-card fade-in" style={{ animationDelay: '0.1s' }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", gap: "1rem", flexWrap: "wrap", marginBottom: "1rem" }}>
        <h2 style={{ margin: 0, fontSize: "1.5rem", background: "linear-gradient(to right, #fff, var(--primary))", WebkitBackgroundClip: "text", WebkitTextFillColor: "transparent" }}>
          Shared Hub
        </h2>
        <div style={{ display: "flex", gap: "0.5rem", background: "rgba(0,0,0,0.2)", padding: "0.25rem", borderRadius: "10px" }}>
          {tabBtn("memory", "Memory")}
          {tabBtn("inbox", "Inbox")}
          {tabBtn("wakes", "Wakes")}
          {tabBtn("policy", "Policy")}
        </div>
      </div>
      
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "1.5rem", paddingBottom: "1rem", borderBottom: "1px solid var(--border-color)" }}>
        <p style={{ fontSize: "0.85rem", color: "var(--text-muted)", margin: 0 }}>
          Data dir: <code style={{ background: "rgba(0,0,0,0.3)", padding: "0.2rem 0.5rem", borderRadius: "4px" }}>{dataDir || "…"}</code>
        </p>
        <div>
          {error && <span style={{ color: "#ef4444", fontSize: "0.85rem", background: "rgba(239, 68, 68, 0.1)", padding: "0.2rem 0.5rem", borderRadius: "4px" }}>Error: {error}</span>}
          {status && !error && <span style={{ color: "#22c55e", fontSize: "0.85rem", background: "rgba(34, 197, 94, 0.1)", padding: "0.2rem 0.5rem", borderRadius: "4px", display: "inline-flex", alignItems: "center", gap: "0.25rem" }}>
            <div style={{ width: "6px", height: "6px", borderRadius: "50%", background: "#22c55e" }} /> {status}
          </span>}
        </div>
      </div>

      {hubTab === "memory" && (
        <div className="fade-in" style={{ display: "flex", flexDirection: "column", gap: "1.5rem" }}>
          <div style={{ display: "flex", gap: "0.75rem", flexWrap: "wrap", alignItems: "center", background: "rgba(0,0,0,0.2)", padding: "1rem", borderRadius: "12px", border: "1px solid var(--border-color)" }}>
            <input
              placeholder="Search memories…"
              value={searchQ}
              onChange={(e) => setSearchQ(e.target.value)}
              style={{ ...inputStyle, flex: 1, minWidth: 200 }}
              onFocus={e => e.target.style.borderColor = 'var(--primary)'}
              onBlur={e => e.target.style.borderColor = 'var(--border-color)'}
            />
            <button className="btn-secondary" onClick={searchMemories}>Search</button>
            <select value={tierFilter} onChange={(e) => setTierFilter(e.target.value)} style={inputStyle}>
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

          <div style={{ ...cardStyle, display: "grid", gap: "1rem" }}>
            <h3 style={{ margin: 0, fontSize: "1rem", fontWeight: 600, color: "var(--text-main)" }}>Write Memory</h3>
            <div style={{ display: "flex", gap: "0.75rem", flexWrap: "wrap" }}>
              <select value={memTier} onChange={(e) => setMemTier(e.target.value)} style={inputStyle}>
                <option value="short_term">short_term</option>
                <option value="episodic">episodic</option>
                <option value="semantic">semantic</option>
              </select>
              <select value={memAgent} onChange={(e) => setMemAgent(e.target.value)} style={inputStyle}>
                {agents.map((a) => (
                  <option key={a.id} value={a.id}>{a.display_name}</option>
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

          <div style={{ display: "flex", flexDirection: "column", gap: "1rem", maxHeight: 500, overflowY: "auto", paddingRight: "0.5rem" }}>
            {memories.length === 0 && (
              <div style={{ padding: "3rem", textAlign: "center", background: "rgba(0,0,0,0.2)", borderRadius: "12px", border: "1px dashed var(--border-color)" }}>
                <p style={{ color: "var(--text-muted)", fontSize: "0.95rem", margin: 0 }}>No memories found in the hub.</p>
              </div>
            )}
            {memories.map((m) => (
              <div key={m.id} style={{ ...cardStyle, position: "relative", overflow: "hidden" }} onMouseEnter={e => e.currentTarget.style.borderColor = 'var(--primary)'} onMouseLeave={e => e.currentTarget.style.borderColor = 'var(--border-color)'}>
                <div style={{ display: "flex", justifyContent: "space-between", gap: "1rem", flexWrap: "wrap", marginBottom: "0.75rem", paddingBottom: "0.75rem", borderBottom: "1px solid var(--border-color)" }}>
                  <div>
                    <strong style={{ fontSize: "1.1rem", color: "var(--primary)" }}>{m.title || "(untitled)"}</strong>
                    <div style={{ display: "flex", gap: "0.5rem", marginTop: "0.35rem" }}>
                      <span style={{ fontSize: "0.7rem", padding: "0.1rem 0.4rem", background: "rgba(255,255,255,0.1)", borderRadius: "4px", color: "var(--text-main)" }}>{m.tier}</span>
                      <span style={{ fontSize: "0.7rem", padding: "0.1rem 0.4rem", background: "rgba(56, 189, 248, 0.1)", borderRadius: "4px", color: "#38bdf8" }}>{m.scope}</span>
                      <span style={{ fontSize: "0.7rem", padding: "0.1rem 0.4rem", background: "rgba(168, 85, 247, 0.1)", borderRadius: "4px", color: "#a855f7" }}>{m.agent_id || "global"}</span>
                    </div>
                  </div>
                  <div style={{ display: "flex", gap: "0.4rem", alignItems: "flex-start" }}>
                    {m.tier === "short_term" && (
                      <button className="btn-secondary" style={{ fontSize: "0.75rem", padding: "0.2rem 0.5rem" }} onClick={async () => {
                        await run("promoted", () => invoke("hub_promote_memory", { id: m.id, toTier: "episodic" }));
                        await refreshMemories();
                      }}>→ episodic</button>
                    )}
                    {m.tier === "episodic" && (
                      <button className="btn-secondary" style={{ fontSize: "0.75rem", padding: "0.2rem 0.5rem" }} onClick={async () => {
                        await run("promoted", () => invoke("hub_promote_memory", { id: m.id, toTier: "semantic" }));
                        await refreshMemories();
                      }}>→ semantic</button>
                    )}
                    <button className="btn-secondary" style={{ fontSize: "0.75rem", padding: "0.2rem 0.5rem" }} onClick={async () => {
                      await run("stale", () => invoke("hub_mark_memory_stale", { id: m.id, stale: true }));
                      await refreshMemories();
                    }}>Stale</button>
                    <button className="btn-secondary" style={{ fontSize: "0.75rem", padding: "0.2rem 0.5rem", borderColor: "rgba(239, 68, 68, 0.3)", color: "#ef4444" }} onClick={async () => {
                      await run("deleted", () => invoke("hub_delete_memory", { id: m.id }));
                      await refreshMemories();
                    }}>Delete</button>
                  </div>
                </div>
                <pre style={{ margin: "0", whiteSpace: "pre-wrap", fontSize: "0.9rem", color: "var(--text-main)", fontFamily: "var(--font-sans)", lineHeight: 1.5 }}>
                  {m.body}
                </pre>
                <div style={{ fontSize: "0.7rem", color: "var(--text-muted)", marginTop: "1rem", textAlign: "right" }}>
                  {m.created_at} · <span style={{ fontFamily: "var(--font-mono)" }}>{m.id.slice(0, 8)}</span>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {hubTab === "inbox" && (
        <div className="fade-in" style={{ display: "flex", flexDirection: "column", gap: "1.5rem" }}>
          <div style={{ ...cardStyle, display: "grid", gap: "1rem" }}>
            <h3 style={{ margin: 0, fontSize: "1rem", fontWeight: 600, color: "var(--text-main)" }}>Send Message / Handoff</h3>
            <div style={{ display: "flex", gap: "0.75rem", flexWrap: "wrap", alignItems: "center" }}>
              <select value={msgFrom} onChange={(e) => setMsgFrom(e.target.value)} style={inputStyle}>
                {agents.map((a) => <option key={a.id} value={a.id}>{a.display_name}</option>)}
              </select>
              <span style={{ color: "var(--text-muted)" }}>→</span>
              <select value={msgTo} onChange={(e) => setMsgTo(e.target.value)} style={inputStyle}>
                {agents.map((a) => <option key={a.id} value={a.id}>{a.display_name}</option>)}
              </select>
              <select value={msgKind} onChange={(e) => setMsgKind(e.target.value)} style={{ ...inputStyle, marginLeft: "auto" }}>
                <option value="message">message</option>
                <option value="handoff">handoff</option>
                <option value="system">system</option>
              </select>
            </div>
            <textarea rows={4} value={msgBody} onChange={(e) => setMsgBody(e.target.value)} placeholder="Message body…" style={{ ...inputStyle, resize: "vertical" }} />
            <div style={{ display: "flex", justifyContent: "flex-end" }}>
              <button className="btn-primary" onClick={sendMessage} disabled={!msgBody.trim()}>Send Message</button>
            </div>
          </div>

          <div style={{ display: "flex", gap: "0.75rem", alignItems: "center", background: "rgba(0,0,0,0.2)", padding: "1rem", borderRadius: "12px", border: "1px solid var(--border-color)" }}>
            <span style={{ fontSize: "0.9rem", color: "var(--text-main)" }}>Poll inbox for:</span>
            <select value={pollTo} onChange={(e) => setPollTo(e.target.value)} style={inputStyle}>
              {agents.map((a) => <option key={a.id} value={a.id}>{a.display_name}</option>)}
            </select>
            <button className="btn-secondary" onClick={pollInbox}>Poll (ack)</button>
            <button className="btn-secondary" onClick={refreshMessages}>Refresh List</button>
          </div>

          <div style={{ display: "flex", flexDirection: "column", gap: "1rem", maxHeight: 400, overflowY: "auto", paddingRight: "0.5rem" }}>
            {messages.length === 0 && (
              <div style={{ padding: "3rem", textAlign: "center", background: "rgba(0,0,0,0.2)", borderRadius: "12px", border: "1px dashed var(--border-color)" }}>
                <p style={{ color: "var(--text-muted)", fontSize: "0.95rem", margin: 0 }}>Inbox is empty.</p>
              </div>
            )}
            {messages.map((m) => (
              <div key={m.id} style={cardStyle}>
                <div style={{ display: "flex", justifyContent: "space-between", marginBottom: "0.75rem", paddingBottom: "0.75rem", borderBottom: "1px solid var(--border-color)" }}>
                  <div style={{ fontSize: "0.95rem" }}>
                    <strong style={{ color: "var(--primary)" }}>{m.from_agent}</strong> <span style={{ color: "var(--text-muted)" }}>→</span> <strong>{m.to_agent}</strong>
                  </div>
                  <div style={{ display: "flex", gap: "0.5rem" }}>
                    <span style={{ fontSize: "0.7rem", padding: "0.1rem 0.4rem", background: "rgba(255,255,255,0.1)", borderRadius: "4px" }}>{m.kind}</span>
                    <span style={{ fontSize: "0.7rem", padding: "0.1rem 0.4rem", background: m.status === 'read' ? "rgba(34, 197, 94, 0.1)" : "rgba(234, 179, 8, 0.1)", color: m.status === 'read' ? "#22c55e" : "#eab308", borderRadius: "4px" }}>{m.status}</span>
                  </div>
                </div>
                {m.subject && <div style={{ fontWeight: 600, marginBottom: "0.5rem", color: "var(--text-main)" }}>{m.subject}</div>}
                <pre style={{ margin: 0, whiteSpace: "pre-wrap", fontSize: "0.9rem", color: "var(--text-main)", fontFamily: "var(--font-sans)", lineHeight: 1.5 }}>
                  {m.body}
                </pre>
              </div>
            ))}
          </div>
        </div>
      )}

      {hubTab === "wakes" && (
        <div className="fade-in" style={{ display: "flex", flexDirection: "column", gap: "1.5rem" }}>
          <div style={{ ...cardStyle, display: "grid", gap: "1rem" }}>
            <h3 style={{ margin: 0, fontSize: "1rem", fontWeight: 600, color: "var(--text-main)" }}>Request Wake</h3>
            <div style={{ display: "flex", gap: "0.75rem", flexWrap: "wrap" }}>
              <select value={wakeTarget} onChange={(e) => setWakeTarget(e.target.value)} style={inputStyle}>
                {agents.map((a) => <option key={a.id} value={a.id}>{a.display_name}</option>)}
              </select>
              <input
                style={{ ...inputStyle, flex: 1, minWidth: 200 }}
                placeholder="Reason for waking..."
                value={wakeReason}
                onChange={(e) => setWakeReason(e.target.value)}
              />
              <button className="btn-primary" onClick={requestWake}>Wake (human gate)</button>
              <button className="btn-secondary" onClick={refreshWakes}>Refresh List</button>
            </div>
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: "1rem", maxHeight: 400, overflowY: "auto", paddingRight: "0.5rem" }}>
            {wakes.length === 0 && (
              <div style={{ padding: "3rem", textAlign: "center", background: "rgba(0,0,0,0.2)", borderRadius: "12px", border: "1px dashed var(--border-color)" }}>
                <p style={{ color: "var(--text-muted)", fontSize: "0.95rem", margin: 0 }}>No wakes recorded.</p>
              </div>
            )}
            {wakes.map((w) => (
              <div key={w.id} style={{ ...cardStyle, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <div>
                  <div style={{ marginBottom: "0.25rem", fontSize: "1.05rem", fontWeight: 600, color: "var(--primary)" }}>
                    {w.target_agent}
                  </div>
                  <div style={{ fontSize: "0.9rem", color: "var(--text-main)" }}>
                    {w.reason || <span style={{ color: "var(--text-muted)", fontStyle: "italic" }}>No reason provided</span>}
                  </div>
                  <div style={{ fontSize: "0.75rem", color: "var(--text-muted)", marginTop: "0.5rem" }}>
                    {w.created_at}
                  </div>
                </div>
                <div style={{ display: "flex", flexDirection: "column", alignItems: "flex-end", gap: "0.5rem" }}>
                  <span style={{ fontSize: "0.75rem", padding: "0.2rem 0.6rem", background: "rgba(255,255,255,0.1)", borderRadius: "20px" }}>
                    {w.status}
                  </span>
                  {w.requires_human_gate && (
                    <span style={{ fontSize: "0.7rem", color: "#eab308", display: "flex", alignItems: "center", gap: "0.25rem" }}>
                      <span style={{ display: "inline-block", width: "6px", height: "6px", background: "#eab308", borderRadius: "50%" }} />
                      Human Gate Required
                    </span>
                  )}
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
      {hubTab === "policy" && wakePolicy && (
        <div className="fade-in" style={{ display: "flex", flexDirection: "column", gap: "1.5rem" }}>
          <div style={{ ...cardStyle, display: "grid", gap: "1.5rem" }}>
            <h3 style={{ margin: 0, fontSize: "1.2rem", fontWeight: 600, color: "var(--text-main)" }}>Wake Policy Controls</h3>
            <p style={{ margin: 0, fontSize: "0.9rem", color: "var(--text-muted)", lineHeight: 1.5 }}>
              Configure standing policies for agent-to-agent wakeups. This policy applies to all agents operating within the local hub.
            </p>

            <div style={{ display: "flex", flexDirection: "column", gap: "1.25rem", marginTop: "0.5rem" }}>
              <label style={{ display: "flex", alignItems: "flex-start", gap: "1rem", cursor: "pointer" }}>
                <input
                  type="checkbox"
                  checked={wakePolicy.default_requires_human_gate}
                  onChange={(e) => updatePolicy({ default_requires_human_gate: e.target.checked })}
                  style={{ marginTop: "0.25rem", width: "1.2rem", height: "1.2rem", accentColor: "var(--primary)" }}
                />
                <div>
                  <div style={{ fontSize: "1rem", fontWeight: 500, color: "var(--text-main)", marginBottom: "0.25rem" }}>Require Human Gate by Default</div>
                  <div style={{ fontSize: "0.85rem", color: "var(--text-muted)" }}>If enabled, all incoming wake requests must be manually approved by the human owner before the target agent is launched.</div>
                </div>
              </label>

              <label style={{ display: "flex", alignItems: "flex-start", gap: "1rem", cursor: "pointer" }}>
                <input
                  type="checkbox"
                  checked={wakePolicy.allow_auto_wake}
                  onChange={(e) => updatePolicy({ allow_auto_wake: e.target.checked })}
                  style={{ marginTop: "0.25rem", width: "1.2rem", height: "1.2rem", accentColor: "var(--primary)" }}
                />
                <div>
                  <div style={{ fontSize: "1rem", fontWeight: 500, color: "var(--text-main)", marginBottom: "0.25rem" }}>Allow Auto-Wake Requests</div>
                  <div style={{ fontSize: "0.85rem", color: "var(--text-muted)" }}>If disabled, any attempt to bypass the human gate (auto-wake) will be outright rejected. Overrides agent-specific delegations.</div>
                </div>
              </label>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
