import { useState, useEffect, useRef } from "react";
import { invoke, isTauriRuntime } from "../../lib/tauri";

export interface HubMessage {
  id: string;
  from_agent: string;
  to_agent: string;
  body: string;
  subject: string | null;
  kind: string;
  status: string;
  created_at: string;
}

export interface HubAgent {
  id: string;
  display_name: string;
}

export interface MemoryRecord {
  id: string;
  scope: string;
  tier: string;
  agent_id?: string | null;
  title?: string | null;
  body: string;
  tags_json: string;
  created_at: string;
  stale: boolean;
}

export interface DetectedProcess {
  pid: number;
  agent: string;
  provider: string;
  model: string;
  command: string;
}

export interface SlackChatPanelProps {
  hubMessages: HubMessage[];
  hubAgents: HubAgent[];
  onRefresh: () => Promise<void>;
}

const AGENT_COLORS: Record<string, { bg: string; text: string; role: string }> = {
  human: { bg: "linear-gradient(135deg, #3b82f6, #1d4ed8)", text: "#93c5fd", role: "Human Developer" },
  gemini: { bg: "linear-gradient(135deg, #a855f7, #7e22ce)", text: "#e9d5ff", role: "Lead Orchestrator" },
  claude: { bg: "linear-gradient(135deg, #f97316, #c2410c)", text: "#ffedd5", role: "Code Agent" },
  grok: { bg: "linear-gradient(135deg, #10b981, #047857)", text: "#a7f3d0", role: "Build & Infra" },
  chat: { bg: "linear-gradient(135deg, #06b6d4, #0e7490)", text: "#cffafe", role: "Chat & Codex" },
  codex: { bg: "linear-gradient(135deg, #06b6d4, #0e7490)", text: "#cffafe", role: "Chat & Codex" },
};

const DEFAULT_CHANNELS = [
  { id: "general", name: "#general", topic: "Team-wide coordination and announcement hub" },
  { id: "team-coordination", name: "#team-coordination", topic: "Inter-agent task claims, handoffs, and bus updates" },
  { id: "agent-memory", name: "#agent-memory", topic: "Shared memory insights, context tags, and audit events" },
  { id: "wakes-alerts", name: "#wakes-alerts", topic: "System wake requests and human approval gates" },
];

export default function SlackChatPanel({ hubMessages, hubAgents, onRefresh }: SlackChatPanelProps) {
  const [activeChannel, setActiveChannel] = useState<string>("general");
  const [messageInput, setMessageInput] = useState<string>("");
  const [targetRecipient, setTargetRecipient] = useState<string>("team");
  const [wakePolicyGate, setWakePolicyGate] = useState<boolean>(false);
  const [sending, setSending] = useState<boolean>(false);
  const [searchTerm, setSearchTerm] = useState<string>("");

  // Memories side drawer state
  const [memories, setMemories] = useState<MemoryRecord[]>([]);
  const [showMemoryDrawer, setShowMemoryDrawer] = useState<boolean>(false);
  const [memorySearch, setMemorySearch] = useState<string>("");
  const [selectedTierFilter, setSelectedTierFilter] = useState<string>("all");

  // Running processes state for presence
  const [runningProcesses, setRunningProcesses] = useState<DetectedProcess[]>([]);

  const messagesEndRef = useRef<HTMLDivElement>(null);

  // Fetch memories and process presence
  useEffect(() => {
    async function loadHubData() {
      if (!isTauriRuntime()) return;
      try {
        const mems = await invoke<MemoryRecord[]>("hub_list_memories", { scope: null, tier: null });
        setMemories(mems);
      } catch (err) {
        console.error("Failed to load hub memories:", err);
      }

      try {
        const procs = await invoke<DetectedProcess[]>("detect_agent_processes");
        setRunningProcesses(procs);
      } catch (err) {
        console.error("Failed to detect agent processes:", err);
      }
    }
    loadHubData();
    const interval = setInterval(loadHubData, 4000);
    return () => clearInterval(interval);
  }, []);

  // Auto-scroll to bottom of chat on message update
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [hubMessages, activeChannel]);

  const handleSendMessage = async () => {
    if (!messageInput.trim() || sending) return;
    setSending(true);
    try {
      const subject = `channel:${activeChannel}`;
      const to = targetRecipient === "team" ? "team" : targetRecipient;

      const sentMsg = await invoke<{ id: string }>("hub_send_message", {
        args: {
          from: "human",
          to,
          kind: "message",
          subject,
          workspace: null,
          task: null,
          body: messageInput.trim(),
        }
      });

      await invoke("hub_request_wake", {
        target: to === "team" ? "chat" : to,
        reason: `Slack Chat message in ${activeChannel}`,
        messageId: sentMsg.id,
        humanGate: wakePolicyGate
      });

      setMessageInput("");
      await onRefresh();
    } catch (err) {
      alert(`Failed to send message: ${err}`);
    } finally {
      setSending(false);
    }
  };

  const getAgentInfo = (agentId: string) => {
    const key = agentId.toLowerCase();
    const info = AGENT_COLORS[key] || {
      bg: "linear-gradient(135deg, #64748b, #334155)",
      text: "#e2e8f0",
      role: "Agent Participant"
    };
    const displayName = agentId === "human"
      ? "Harbinger (Human Dev)"
      : hubAgents.find(a => a.id === agentId)?.display_name || agentId;

    const isRunning = runningProcesses.some(p => {
      const detected = p.agent.toLowerCase();
      return detected === key || (key === "chat" && detected === "codex");
    });
    return { ...info, displayName, isRunning };
  };

  // Filter messages for active channel / DM view
  const channelMessages = hubMessages.filter(msg => {
    if (searchTerm.trim()) {
      const q = searchTerm.toLowerCase();
      if (!msg.body.toLowerCase().includes(q) && !(msg.subject || "").toLowerCase().includes(q)) {
        return false;
      }
    }

    if (activeChannel.startsWith("dm-")) {
      const dmTarget = activeChannel.replace("dm-", "");
      return (msg.from_agent === dmTarget && msg.to_agent === "human") ||
             (msg.from_agent === "human" && msg.to_agent === dmTarget);
    }

    if (msg.subject && msg.subject.startsWith("channel:")) {
      return msg.subject === `channel:${activeChannel}`;
    }

    // Default general fallback for non-channel prefixed messages
    return activeChannel === "general";
  });

  const filteredMemories = memories.filter(m => {
    if (selectedTierFilter !== "all" && m.tier !== selectedTierFilter) return false;
    if (memorySearch.trim()) {
      const q = memorySearch.toLowerCase();
      return (m.title || "").toLowerCase().includes(q) || m.body.toLowerCase().includes(q);
    }
    return true;
  });

  const insertMemoryLink = (memId: string) => {
    setMessageInput(prev => `${prev} [Memory #${memId.slice(0, 8)}]`);
  };

  return (
    <div style={{
      display: "grid",
      gridTemplateColumns: showMemoryDrawer ? "260px 1fr 340px" : "260px 1fr",
      height: "calc(100vh - 120px)",
      gap: "1rem",
      color: "var(--text-main)",
      fontFamily: "'Inter', sans-serif"
    }}>
      {/* Sidebar: Slack Channels & Direct Messages */}
      <div className="glass-card" style={{
        padding: "1.25rem 1rem",
        display: "flex",
        flexDirection: "column",
        gap: "1.5rem",
        overflowY: "auto"
      }}>
        {/* Workspace Header */}
        <div style={{ display: "flex", alignItems: "center", gap: "0.75rem", paddingBottom: "0.75rem", borderBottom: "1px solid var(--border-color)" }}>
          <div style={{
            width: "36px", height: "36px", borderRadius: "10px",
            background: "linear-gradient(135deg, var(--primary), var(--accent))",
            display: "flex", alignItems: "center", justifyContent: "center",
            fontWeight: 800, fontSize: "1.1rem", color: "#fff"
          }}>
            CA
          </div>
          <div>
            <div style={{ fontWeight: 700, fontSize: "0.95rem" }}>Coding Assistants</div>
            <div style={{ fontSize: "0.75rem", color: "var(--text-muted)", display: "flex", alignItems: "center", gap: "0.35rem" }}>
              <span style={{ width: "8px", height: "8px", borderRadius: "50%", background: "#10b981" }}></span>
              Multi-Agent Hub Online
            </div>
          </div>
        </div>

        {/* Channels List */}
        <div>
          <div style={{ fontSize: "0.75rem", fontWeight: 700, color: "var(--text-muted)", letterSpacing: "0.05em", textTransform: "uppercase", marginBottom: "0.5rem", paddingLeft: "0.5rem" }}>
            Channels
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: "0.25rem" }}>
            {DEFAULT_CHANNELS.map(ch => {
              const isActive = activeChannel === ch.id;
              const unreadCount = hubMessages.filter(m => m.subject === `channel:${ch.id}`).length;
              return (
                <button
                  key={ch.id}
                  onClick={() => setActiveChannel(ch.id)}
                  style={{
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "space-between",
                    padding: "0.5rem 0.75rem",
                    borderRadius: "8px",
                    border: "none",
                    background: isActive ? "rgba(99, 102, 241, 0.2)" : "transparent",
                    color: isActive ? "#fff" : "var(--text-muted)",
                    fontWeight: isActive ? 600 : 400,
                    fontSize: "0.875rem",
                    cursor: "pointer",
                    textAlign: "left",
                    transition: "all 0.15s ease"
                  }}
                >
                  <span style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{ch.name}</span>
                  {unreadCount > 0 && (
                    <span style={{
                      background: isActive ? "var(--primary)" : "rgba(255, 255, 255, 0.1)",
                      color: "#fff",
                      fontSize: "0.7rem",
                      padding: "0.15rem 0.45rem",
                      borderRadius: "10px",
                      fontWeight: 600
                    }}>
                      {unreadCount}
                    </span>
                  )}
                </button>
              );
            })}
          </div>
        </div>

        {/* Direct Messages Roster */}
        <div>
          <div style={{ fontSize: "0.75rem", fontWeight: 700, color: "var(--text-muted)", letterSpacing: "0.05em", textTransform: "uppercase", marginBottom: "0.5rem", paddingLeft: "0.5rem" }}>
            Direct Messages & Agents
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: "0.25rem" }}>
            {["human", "gemini", "claude", "grok", "chat"].map(agentId => {
              const info = getAgentInfo(agentId);
              const dmId = `dm-${agentId}`;
              const isActive = activeChannel === dmId;
              return (
                <button
                  key={agentId}
                  onClick={() => setActiveChannel(dmId)}
                  style={{
                    display: "flex",
                    alignItems: "center",
                    gap: "0.6rem",
                    padding: "0.5rem 0.75rem",
                    borderRadius: "8px",
                    border: "none",
                    background: isActive ? "rgba(168, 85, 247, 0.2)" : "transparent",
                    color: isActive ? "#fff" : "var(--text-muted)",
                    fontWeight: isActive ? 600 : 400,
                    fontSize: "0.85rem",
                    cursor: "pointer",
                    textAlign: "left"
                  }}
                >
                  {/* Status Indicator Dot */}
                  <span style={{
                    width: "8px", height: "8px", borderRadius: "50%",
                    background: agentId === "human" ? "#3b82f6" : info.isRunning ? "#10b981" : "#64748b",
                    flexShrink: 0
                  }} />
                  <div style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", flex: 1 }}>
                    {info.displayName}
                  </div>
                </button>
              );
            })}
          </div>
        </div>

        {/* Memory Hub Drawer Toggle Button */}
        <div style={{ marginTop: "auto", paddingTop: "1rem", borderTop: "1px solid var(--border-color)" }}>
          <button
            className={showMemoryDrawer ? "btn-primary" : "btn-secondary"}
            onClick={() => setShowMemoryDrawer(prev => !prev)}
            style={{ width: "100%", padding: "0.6rem", fontSize: "0.85rem", borderRadius: "8px", display: "flex", alignItems: "center", justifyContent: "center", gap: "0.5rem" }}
          >
            <span>🧠 Memory Hub</span>
            <span style={{ fontSize: "0.75rem", opacity: 0.8 }}>({memories.length})</span>
          </button>
        </div>
      </div>

      {/* Main Slack Chat View Canvas */}
      <div className="glass-card" style={{ padding: "0", display: "flex", flexDirection: "column", overflow: "hidden" }}>
        {/* Chat Header */}
        <div style={{
          padding: "1rem 1.5rem",
          borderBottom: "1px solid var(--border-color)",
          background: "rgba(2, 6, 23, 0.6)",
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between"
        }}>
          <div>
            <h2 style={{ margin: 0, fontSize: "1.1rem", fontWeight: 700, display: "flex", alignItems: "center", gap: "0.5rem" }}>
              <span>{activeChannel.startsWith("dm-") ? `💬 Direct Message: ${getAgentInfo(activeChannel.replace("dm-", "")).displayName}` : `#${activeChannel}`}</span>
            </h2>
            <p style={{ fontSize: "0.8rem", color: "var(--text-muted)", margin: "0.2rem 0 0 0" }}>
              {DEFAULT_CHANNELS.find(c => c.id === activeChannel)?.topic || "Agent interaction stream"}
            </p>
          </div>

          {/* Search Input in Header */}
          <div style={{ display: "flex", alignItems: "center", gap: "0.75rem" }}>
            <input
              type="text"
              placeholder="Search chat..."
              value={searchTerm}
              onChange={e => setSearchTerm(e.target.value)}
              style={{
                padding: "0.45rem 0.8rem",
                borderRadius: "8px",
                background: "rgba(0,0,0,0.4)",
                border: "1px solid var(--border-color)",
                color: "#fff",
                fontSize: "0.85rem",
                outline: "none"
              }}
            />
          </div>
        </div>

        {/* Message Stream Scroll Area */}
        <div style={{
          flex: 1,
          padding: "1.5rem",
          overflowY: "auto",
          display: "flex",
          flexDirection: "column",
          gap: "1.25rem"
        }}>
          {channelMessages.length === 0 ? (
            <div style={{
              display: "flex",
              flexDirection: "column",
              alignItems: "center",
              justifyContent: "center",
              height: "100%",
              color: "var(--text-muted)",
              textAlign: "center"
            }}>
              <div style={{ fontSize: "2.5rem", marginBottom: "0.5rem" }}>💬</div>
              <p style={{ fontWeight: 600 }}>No messages in this channel yet.</p>
              <p style={{ fontSize: "0.85rem" }}>Be the first to post a note or coordinate agent tasks!</p>
            </div>
          ) : (
            channelMessages.map(msg => {
              const sender = getAgentInfo(msg.from_agent);
              const formattedTime = new Date(msg.created_at).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
              return (
                <div key={msg.id} style={{ display: "flex", gap: "1rem", alignItems: "flex-start" }}>
                  {/* Sender Avatar Bubble */}
                  <div style={{
                    width: "38px",
                    height: "38px",
                    borderRadius: "12px",
                    background: sender.bg,
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "center",
                    fontWeight: 700,
                    color: "#fff",
                    fontSize: "0.95rem",
                    flexShrink: 0,
                    boxShadow: "0 4px 10px rgba(0,0,0,0.2)"
                  }}>
                    {msg.from_agent.slice(0, 2).toUpperCase()}
                  </div>

                  {/* Message Bubble Body */}
                  <div style={{ flex: 1 }}>
                    <div style={{ display: "flex", alignItems: "center", gap: "0.6rem", marginBottom: "0.25rem" }}>
                      <span style={{ fontWeight: 700, fontSize: "0.9rem", color: sender.text }}>{sender.displayName}</span>
                      <span style={{
                        fontSize: "0.7rem",
                        padding: "0.1rem 0.4rem",
                        borderRadius: "6px",
                        background: "rgba(255, 255, 255, 0.08)",
                        color: "var(--text-muted)"
                      }}>
                        {sender.role}
                      </span>
                      <span style={{ fontSize: "0.75rem", color: "var(--text-muted)" }}>{formattedTime}</span>
                    </div>

                    <div style={{
                      background: "rgba(0,0,0,0.35)",
                      border: "1px solid var(--border-color)",
                      borderRadius: "12px",
                      padding: "0.85rem 1.1rem",
                      fontSize: "0.95rem",
                      lineHeight: "1.5",
                      whiteSpace: "pre-wrap",
                      wordBreak: "break-word"
                    }}>
                      {msg.body}
                    </div>
                  </div>
                </div>
              );
            })
          )}
          <div ref={messagesEndRef} />
        </div>

        {/* Slack Message Input Box */}
        <div style={{
          padding: "1.25rem 1.5rem",
          borderTop: "1px solid var(--border-color)",
          background: "rgba(2, 6, 23, 0.8)"
        }}>
          <div style={{ display: "flex", flexDirection: "column", gap: "0.75rem" }}>
            <textarea
              rows={3}
              placeholder={`Message #${activeChannel}... (Press Ctrl+Enter to send)`}
              value={messageInput}
              onChange={e => setMessageInput(e.target.value)}
              onKeyDown={e => {
                if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
                  e.preventDefault();
                  handleSendMessage();
                }
              }}
              style={{
                width: "100%",
                background: "rgba(0,0,0,0.4)",
                border: "1px solid var(--border-color)",
                borderRadius: "10px",
                padding: "0.85rem 1rem",
                color: "#fff",
                fontSize: "0.95rem",
                outline: "none",
                resize: "none"
              }}
            />

            {/* Input Controls Row */}
            <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
              <div style={{ display: "flex", alignItems: "center", gap: "1rem" }}>
                {/* Target Recipient Selector */}
                <select
                  value={targetRecipient}
                  onChange={e => setTargetRecipient(e.target.value)}
                  style={{
                    padding: "0.4rem 0.75rem",
                    borderRadius: "8px",
                    background: "rgba(0,0,0,0.4)",
                    color: "var(--text-main)",
                    border: "1px solid var(--border-color)",
                    fontSize: "0.85rem",
                    outline: "none"
                  }}
                >
                  <option value="team">Broadcast to Team</option>
                  <option value="gemini">Gemini (Lead Orchestrator)</option>
                  <option value="claude">Claude (Code Agent)</option>
                  <option value="grok">Grok (Build Agent)</option>
                  <option value="chat">Chat (Codex Agent)</option>
                </select>

                {/* Wake Gate Checkbox */}
                <label style={{ display: "flex", alignItems: "center", gap: "0.4rem", fontSize: "0.8rem", color: "var(--text-muted)", cursor: "pointer" }}>
                  <input
                    type="checkbox"
                    checked={wakePolicyGate}
                    onChange={e => setWakePolicyGate(e.target.checked)}
                  />
                  Require Human Approval Gate
                </label>
              </div>

              <button
                className={sending ? "btn-secondary" : "btn-primary"}
                onClick={handleSendMessage}
                disabled={!messageInput.trim() || sending}
                style={{ padding: "0.6rem 1.5rem", fontSize: "0.9rem" }}
              >
                {sending ? "Sending..." : "Send Message"}
              </button>
            </div>
          </div>
        </div>
      </div>

      {/* Memory Hub Drawer Sidebar */}
      {showMemoryDrawer && (
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
      )}
    </div>
  );
}
