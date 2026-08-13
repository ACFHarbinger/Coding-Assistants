import { useState, useEffect, useLayoutEffect, useRef } from "react";
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
  team_member?: boolean;
}

export interface WorkSession {
  id: string;
  name: string;
  created_at: string;
  member_ids: string[];
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
  workSessions: WorkSession[];
  activeWorkSessionId: string | null;
  focusSessionId?: string | null;
  onSelectWorkSession: (sessionId: string | null) => void;
  onRefresh: () => Promise<void>;
}

const AGENT_COLORS: Record<string, { bg: string; text: string; role: string }> = {
  human: { bg: "linear-gradient(135deg, #3b82f6, #1d4ed8)", text: "#93c5fd", role: "Human Developer" },
  grok: { bg: "linear-gradient(135deg, #10b981, #047857)", text: "#a7f3d0", role: "Lead Orchestrator" },
  chat: { bg: "linear-gradient(135deg, #06b6d4, #0e7490)", text: "#cffafe", role: "Co-Lead / Codex" },
  codex: { bg: "linear-gradient(135deg, #06b6d4, #0e7490)", text: "#cffafe", role: "Co-Lead / Codex" },
  claude: { bg: "linear-gradient(135deg, #f97316, #c2410c)", text: "#ffedd5", role: "Code Agent" },
  gemini: { bg: "linear-gradient(135deg, #a855f7, #7e22ce)", text: "#e9d5ff", role: "Supporting" },
};

const DEFAULT_CHANNELS = [
  { id: "general", name: "#general", topic: "Team-wide coordination and announcement hub" },
  { id: "team-coordination", name: "#team-coordination", topic: "Inter-agent task claims, handoffs, and bus updates" },
  { id: "agent-memory", name: "#agent-memory", topic: "Shared memory insights, context tags, and audit events" },
  { id: "wakes-alerts", name: "#wakes-alerts", topic: "System wake requests and human approval gates" },
];

const FALLBACK_ROSTER = ["human", "grok", "chat", "claude", "gemini"];

function rosterAgentIds(hubAgents: HubAgent[]): string[] {
  const enrolled = hubAgents
    .filter(agent => agent.team_member && agent.id !== "system")
    .map(agent => agent.id);
  const ids = enrolled.length > 0 ? enrolled : FALLBACK_ROSTER;
  const rest = ids.filter(id => id !== "human");
  return ids.includes("human") ? ["human", ...rest] : ids;
}

function teamWakeTargets(hubAgents: HubAgent[]): string[] {
  return rosterAgentIds(hubAgents).filter(id => id !== "human" && id !== "system");
}

const NEAR_BOTTOM_PX = 96;

function isNearBottom(el: HTMLElement): boolean {
  return el.scrollHeight - el.scrollTop - el.clientHeight <= NEAR_BOTTOM_PX;
}

/** Collapse team fan-out copies of one post without merging later distinct sends. */
function channelDedupeKey(msg: HubMessage, channel: string): string {
  const prefix = `channel:${channel}`;
  if (msg.subject && msg.subject.startsWith(`${prefix}:`) && msg.subject.length > prefix.length + 1) {
    return msg.subject;
  }
  return `${msg.from_agent}|${msg.body}|${(msg.created_at || "").slice(0, 19)}`;
}

const LAST_READ_STORAGE_KEY = "ca-slack-last-read";

function loadLastRead(): Record<string, string> {
  try {
    const raw = localStorage.getItem(LAST_READ_STORAGE_KEY);
    if (!raw) return {};
    const parsed = JSON.parse(raw) as Record<string, string>;
    return parsed && typeof parsed === "object" ? parsed : {};
  } catch {
    return {};
  }
}

function belongsToChannel(message: HubMessage, channelId: string): boolean {
  if (message.status === "cancelled") return false;
  if (channelId === "general" && !message.subject?.startsWith("channel:")) {
    return true;
  }
  return message.subject === `channel:${channelId}`
    || Boolean(message.subject?.startsWith(`channel:${channelId}:`));
}

function uniqueChannelPosts(messages: HubMessage[], channelId: string): HubMessage[] {
  const seen = new Set<string>();
  const posts: HubMessage[] = [];
  for (const message of messages) {
    if (!belongsToChannel(message, channelId)) continue;
    const key = channelDedupeKey(message, channelId);
    if (seen.has(key)) continue;
    seen.add(key);
    posts.push(message);
  }
  return posts;
}

function persistLastRead(next: Record<string, string>): Record<string, string> {
  try {
    localStorage.setItem(LAST_READ_STORAGE_KEY, JSON.stringify(next));
  } catch {
    /* ignore quota / private-mode failures */
  }
  return next;
}

function unreadPosts(messages: HubMessage[], channelId: string, watermark: string | undefined): HubMessage[] {
  return uniqueChannelPosts(messages, channelId).filter(message =>
    message.from_agent !== "human" && message.created_at > (watermark || "")
  );
}

function latestCreatedAt(messages: HubMessage[]): string | null {
  if (messages.length === 0) return null;
  return messages.reduce((latest, message) =>
    message.created_at > latest ? message.created_at : latest,
  messages[0].created_at);
}

interface ContextMenuState {
  messageId: string;
  x: number;
  y: number;
}

interface ReplyTarget {
  id: string;
  fromAgent: string;
  preview: string;
}

function threadRootId(message: HubMessage, channel: string): string | null {
  const prefix = `channel:${channel}:thread:`;
  if (!message.subject?.startsWith(prefix)) return null;
  const rootId = message.subject.slice(prefix.length).split(":", 1)[0];
  return rootId || null;
}

export default function SlackChatPanel({ hubMessages, hubAgents, workSessions, activeWorkSessionId, focusSessionId, onSelectWorkSession, onRefresh }: SlackChatPanelProps) {
  const [activeChannel, setActiveChannel] = useState<string>("general");
  const [messageInput, setMessageInput] = useState<string>("");
  const [wakePolicyGate, setWakePolicyGate] = useState<boolean>(false);
  const [sending, setSending] = useState<boolean>(false);
  const [searchTerm, setSearchTerm] = useState<string>("");
  const [lastReadAt, setLastReadAt] = useState<Record<string, string>>(loadLastRead);
  const [channelRecords, setChannelRecords] = useState<HubMessage[]>([]);
  const [linkedMemories, setLinkedMemories] = useState<Record<string, MemoryRecord[]>>({});
  const [replyTo, setReplyTo] = useState<ReplyTarget | null>(null);
  const [sessionWakeTargets, setSessionWakeTargets] = useState<Record<string, boolean>>({});

  // Canonical U12 / C10 recipient selection & intent tag state
  const [recipientMode, setRecipientMode] = useState<"all" | "subset" | "single">("all");
  const [selectedSubset, setSelectedSubset] = useState<Record<string, boolean>>({});
  const [singleRecipient, setSingleRecipient] = useState<string>("grok");
  const [isTaskTag, setIsTaskTag] = useState<boolean>(false);
  const [isWakeTag, setIsWakeTag] = useState<boolean>(false);

  // Memories side drawer state
  const [memories, setMemories] = useState<MemoryRecord[]>([]);
  const [showMemoryDrawer, setShowMemoryDrawer] = useState<boolean>(false);
  const [memorySearch, setMemorySearch] = useState<string>("");
  const [selectedTierFilter, setSelectedTierFilter] = useState<string>("all");

  // Running processes state for presence
  const [runningProcesses, setRunningProcesses] = useState<DetectedProcess[]>([]);

  // Message context menu (CA-106: right-click Edit / Delete, Harbinger's posts only)
  const [contextMenu, setContextMenu] = useState<ContextMenuState | null>(null);
  const [hoveredMessageId, setHoveredMessageId] = useState<string | null>(null);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editDraft, setEditDraft] = useState<string>("");
  const [mutating, setMutating] = useState<boolean>(false);

  const scrollBoxRef = useRef<HTMLDivElement>(null);
  const stickToBottomRef = useRef(true);
  const forceScrollRef = useRef(false);
  const prevChannelRef = useRef(activeChannel);
  const [jumpToLatest, setJumpToLatest] = useState(false);
  const activeWorkSession = workSessions.find(session => session.id === activeWorkSessionId) || null;

  useEffect(() => {
    if (!focusSessionId) return;
    setActiveChannel(`session:${focusSessionId}`);
  }, [focusSessionId]);

  useEffect(() => {
    if (!activeWorkSession) return;
    setSessionWakeTargets(previous => {
      const next: Record<string, boolean> = {};
      for (const agentId of activeWorkSession.member_ids) {
        if (agentId !== "human") next[agentId] = previous[agentId] ?? true;
      }
      return next;
    });
  }, [activeWorkSessionId, activeWorkSession?.member_ids.join(",")]);

  // Close the context menu on outside click or Escape.
  useEffect(() => {
    if (!contextMenu) return;
    const closeMenu = () => setContextMenu(null);
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") setContextMenu(null);
    };
    window.addEventListener("click", closeMenu);
    window.addEventListener("contextmenu", closeMenu);
    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("click", closeMenu);
      window.removeEventListener("contextmenu", closeMenu);
      window.removeEventListener("keydown", onKeyDown);
    };
  }, [contextMenu]);

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

  useEffect(() => {
    const pool = activeChannel.startsWith("dm-")
      ? hubMessages
      : [...hubMessages, ...channelRecords];
    const posts = uniqueChannelPosts(pool, activeChannel);
    const latest = latestCreatedAt(posts);
    if (!latest) return;
    setLastReadAt(prev => {
      if ((prev[activeChannel] || "") >= latest) return prev;
      return persistLastRead({ ...prev, [activeChannel]: latest });
    });
  }, [activeChannel, hubMessages, channelRecords]);

  useEffect(() => {
    setLastReadAt(prev => {
      let changed = false;
      const next = { ...prev };
      for (const channel of DEFAULT_CHANNELS) {
        if (next[channel.id]) continue;
        const latest = latestCreatedAt(uniqueChannelPosts(hubMessages, channel.id));
        if (!latest) continue;
        next[channel.id] = latest;
        changed = true;
      }
      return changed ? persistLastRead(next) : prev;
    });
  }, [hubMessages]);

  // Channel views use the bounded Hub query instead of filtering the entire
  // transcript in the renderer. DMs stay local because their privacy predicate
  // is participant-specific rather than a channel subject.
  useEffect(() => {
    let disposed = false;
    setReplyTo(null);
    if (activeChannel.startsWith("dm-")) {
      setChannelRecords([]);
      return () => { disposed = true; };
    }
    if (!isTauriRuntime()) return;
    invoke<HubMessage[]>("hub_list_channel_messages", { channel: activeChannel, limit: 200 })
      .then((records) => {
        if (!disposed) setChannelRecords(records);
      })
      .catch((error) => console.error("Failed to load channel messages:", error));
    return () => { disposed = true; };
  }, [activeChannel, hubMessages]);

  const handleSendMessage = async () => {
    if (!messageInput.trim() || sending) return;
    const dmTarget = activeChannel.startsWith("dm-")
      ? activeChannel.replace("dm-", "")
      : null;
    if (dmTarget === "human") return;
    setSending(true);
    try {
      const sessionChannel = activeChannel.startsWith("session:") ? activeChannel : null;
      let bodyText = messageInput.trim();
      const enrolledRoster = rosterAgentIds(hubAgents).filter(id => id !== "human" && id !== "system");

      let targetAgents: string[] = [];
      if (dmTarget) {
        targetAgents = [dmTarget];
      } else if (sessionChannel && activeWorkSession) {
        targetAgents = activeWorkSession.member_ids.filter(id => id !== "human" && id !== "system");
      } else if (recipientMode === "single") {
        targetAgents = [singleRecipient];
      } else if (recipientMode === "subset") {
        targetAgents = Object.keys(selectedSubset).filter(id => selectedSubset[id]);
        if (targetAgents.length === 0) {
          alert("Please select at least one recipient agent for subset messaging.");
          setSending(false);
          return;
        }
      } else {
        targetAgents = enrolledRoster;
      }

      // C11 Validation: Task-tagged messages MUST target existing team members
      if (isTaskTag) {
        const nonTeamTargets = targetAgents.filter(id => !enrolledRoster.includes(id));
        if (nonTeamTargets.length > 0) {
          alert(`Task-tagged messages must target existing team members. Target(s) not on team: ${nonTeamTargets.join(", ")}. Please enroll the agent or use [WAKE] tag to spawn a new instance.`);
          setSending(false);
          return;
        }
      }

      // Ensure tags are in body text
      if (isTaskTag && !bodyText.startsWith("[TASK]")) {
        bodyText = `[TASK] ${bodyText}`;
      }
      if (isWakeTag && !bodyText.startsWith("[WAKE]")) {
        bodyText = `[WAKE] ${bodyText}`;
      }

      const messageKind = isTaskTag ? "task" : isWakeTag ? "wake" : "message";
      let subject = dmTarget
        ? `private:${crypto.randomUUID()}`
        : replyTo
          ? `channel:${activeChannel}:thread:${replyTo.id}:${crypto.randomUUID()}`
        : `channel:${activeChannel}:${crypto.randomUUID()}`;

      if (isTaskTag) subject += `:kind:task`;
      else if (isWakeTag) subject += `:kind:wake`;

      const toField = dmTarget
        ? dmTarget
        : recipientMode === "all" && !sessionChannel
          ? "team"
          : targetAgents.join(",");

      if (sessionChannel && activeWorkSession) {
        if (targetAgents.length === 0) throw new Error("The active work session has no members");
        const messages = await Promise.all(targetAgents.map(recipient =>
          invoke<{ id: string }>("hub_send_message", {
            args: { from: "human", to: recipient, kind: messageKind, subject, workspace: null, task: isTaskTag ? bodyText : null, body: bodyText }
          }).then(message => ({ recipient, message }))
        ));
        await Promise.all(messages
          .filter(({ recipient }) => recipient !== "human" && (isWakeTag || sessionWakeTargets[recipient]))
          .map(({ recipient, message }) => invoke("hub_request_wake", {
            target: recipient,
            reason: `Work session: ${activeWorkSession.name}`,
            messageId: message.id,
            humanGate: wakePolicyGate
          })));
      } else {
        const sentMsg = await invoke<{ id: string }>("hub_send_message", {
          args: { from: "human", to: toField, kind: messageKind, subject, workspace: null, task: isTaskTag ? bodyText : null, body: bodyText }
        });
        const wakeTargets = toField === "team" ? teamWakeTargets(hubAgents) : targetAgents;
        if (isWakeTag || wakePolicyGate) {
          await Promise.all(wakeTargets.map(target => invoke("hub_request_wake", {
            target, reason: `Chat & Memory message in ${activeChannel}`, messageId: sentMsg.id, humanGate: wakePolicyGate
          })));
        }
      }

      setMessageInput("");
      setReplyTo(null);
      forceScrollRef.current = true;
      stickToBottomRef.current = true;
      await onRefresh();
    } catch (err) {
      alert(`Failed to send message: ${err}`);
    } finally {
      setSending(false);
    }
  };

  const startReply = (message: HubMessage) => {
    const rootId = threadRootId(message, activeChannel) || message.id;
    setReplyTo({
      id: rootId,
      fromAgent: message.from_agent,
      preview: message.body,
    });
  };

  const openMessageMenu = (e: React.MouseEvent, msg: HubMessage) => {
    if (msg.from_agent !== "human") return;
    e.preventDefault();
    // Stop this same click/right-click from reaching the window-level
    // listener that closes the menu (attached below): without this, opening
    // a new message's menu while another one is already open would
    // immediately re-close itself via the still-attached prior listener.
    e.stopPropagation();
    setContextMenu({ messageId: msg.id, x: e.clientX, y: e.clientY });
  };

  const startEdit = (msg: HubMessage) => {
    setEditingId(msg.id);
    setEditDraft(msg.body);
    setContextMenu(null);
  };

  const cancelEdit = () => {
    setEditingId(null);
    setEditDraft("");
  };

  const saveEdit = async () => {
    if (!editingId || !editDraft.trim() || mutating) return;
    setMutating(true);
    try {
      await invoke("hub_update_message", { id: editingId, body: editDraft.trim() });
      cancelEdit();
      await onRefresh();
    } catch (err) {
      alert(`Failed to edit message: ${err}`);
    } finally {
      setMutating(false);
    }
  };

  const deleteMessage = async (messageId: string) => {
    setContextMenu(null);
    if (!window.confirm("Delete this message for everyone?")) return;
    setMutating(true);
    try {
      await invoke("hub_delete_message", { id: messageId });
      await onRefresh();
    } catch (err) {
      alert(`Failed to delete message: ${err}`);
    } finally {
      setMutating(false);
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
  const channelMessages = (() => {
    const records = activeChannel.startsWith("dm-")
      ? hubMessages
      : [
          ...channelRecords,
          // Preserve pre-channel legacy messages in #general only.
          ...(activeChannel === "general"
            ? hubMessages.filter(msg => !msg.subject?.startsWith("channel:"))
            : []),
        ];
    const matches = records.filter(msg => {
      if (msg.status === "cancelled") return false;
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

      const prefix = `channel:${activeChannel}`;
      if (msg.subject === prefix || msg.subject?.startsWith(`${prefix}:`)) {
        return true;
      }

      // Default general fallback for non-channel prefixed messages
      return activeChannel === "general" && !msg.subject?.startsWith("channel:");
    });

    if (activeChannel.startsWith("dm-")) {
      return matches;
    }

    const seen = new Set<string>();
    return matches.filter(msg => {
      const key = channelDedupeKey(msg, activeChannel);
      if (seen.has(key)) return false;
      seen.add(key);
      return true;
    });
  })();

  const lastThreadId = channelMessages.length > 0 ? channelMessages[channelMessages.length - 1].id : "";
  const threadKey = `${activeChannel}:${channelMessages.length}:${lastThreadId}`;

  useEffect(() => {
    let disposed = false;
    const messagesWithReferences = channelMessages.filter(message => message.body.includes("[Memory #"));
    if (messagesWithReferences.length === 0) {
      setLinkedMemories({});
      return () => { disposed = true; };
    }
    Promise.all(messagesWithReferences.map(async (message) => [
      message.id,
      await invoke<MemoryRecord[]>("hub_list_message_memories", { messageId: message.id }),
    ] as const))
      .then((entries) => {
        if (!disposed) setLinkedMemories(Object.fromEntries(entries));
      })
      .catch((error) => console.error("Failed to resolve message memory links:", error));
    return () => { disposed = true; };
  }, [threadKey]);

  useLayoutEffect(() => {
    const el = scrollBoxRef.current;
    if (!el) return;
    const channelChanged = prevChannelRef.current !== activeChannel;
    prevChannelRef.current = activeChannel;
    if (channelChanged || forceScrollRef.current || stickToBottomRef.current) {
      el.scrollTop = el.scrollHeight;
      stickToBottomRef.current = true;
      forceScrollRef.current = false;
      setJumpToLatest(false);
    } else {
      setJumpToLatest(true);
    }
  }, [threadKey, activeChannel]);

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
              const unreadCount = isActive
                ? 0
                : unreadPosts(
                    ch.id === activeChannel ? [...hubMessages, ...channelRecords] : hubMessages,
                    ch.id,
                    lastReadAt[ch.id]
                  ).length;
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
        {workSessions.length > 0 && (
          <div>
            <div style={{ fontSize: "0.75rem", fontWeight: 700, color: "var(--text-muted)", letterSpacing: "0.05em", textTransform: "uppercase", marginBottom: "0.5rem", paddingLeft: "0.5rem" }}>
              Work Sessions
            </div>
            <div style={{ display: "flex", flexDirection: "column", gap: "0.25rem" }}>
              {workSessions.map(session => {
                const channel = `session:${session.id}`;
                const isActive = activeChannel === channel;
                return <button key={session.id} onClick={() => { onSelectWorkSession(session.id); setActiveChannel(channel); }} style={{ display: "flex", justifyContent: "space-between", gap: "0.5rem", padding: "0.5rem 0.75rem", borderRadius: "8px", border: "none", background: isActive ? "rgba(6, 182, 212, 0.2)" : "transparent", color: isActive ? "#fff" : "var(--text-muted)", cursor: "pointer", textAlign: "left", fontSize: "0.85rem" }}>
                  <span style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>◈ {session.name}</span>
                  <span style={{ fontSize: "0.72rem" }}>{session.member_ids.length}</span>
                </button>;
              })}
            </div>
          </div>
        )}

        {/* Direct Messages Roster */}
        <div>
          <div style={{ fontSize: "0.75rem", fontWeight: 700, color: "var(--text-muted)", letterSpacing: "0.05em", textTransform: "uppercase", marginBottom: "0.5rem", paddingLeft: "0.5rem" }}>
            Direct Messages & Agents
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: "0.25rem" }}>
            {rosterAgentIds(hubAgents).map(agentId => {
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
              <span>{activeChannel.startsWith("dm-") ? `💬 Direct Message: ${getAgentInfo(activeChannel.replace("dm-", "")).displayName}` : activeWorkSession && activeChannel === `session:${activeWorkSession.id}` ? `◈ Work session: ${activeWorkSession.name}` : `#${activeChannel}`}</span>
            </h2>
            <p style={{ fontSize: "0.8rem", color: "var(--text-muted)", margin: "0.2rem 0 0 0" }}>
              {activeWorkSession && activeChannel === `session:${activeWorkSession.id}` ? `${activeWorkSession.member_ids.length} members · messages from the human and agent harnesses` : DEFAULT_CHANNELS.find(c => c.id === activeChannel)?.topic || "Agent interaction stream"}
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
        <div style={{ flex: 1, position: "relative", minHeight: 0 }}>
        <div
          ref={scrollBoxRef}
          onScroll={() => {
            const el = scrollBoxRef.current;
            if (!el) return;
            const near = isNearBottom(el);
            stickToBottomRef.current = near;
            if (near) setJumpToLatest(false);
          }}
          style={{
          height: "100%",
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
              const rootId = threadRootId(msg, activeChannel);
              const rootMessage = rootId ? channelMessages.find(candidate => candidate.id === rootId) : null;
              const rootSender = rootMessage ? getAgentInfo(rootMessage.from_agent) : null;
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
                    <div style={{ display: "flex", alignItems: "center", gap: "0.5rem", flexWrap: "wrap", marginBottom: "0.25rem" }}>
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

                      {/* Intent Badges */}
                      {(msg.kind === "task" || msg.body.includes("[TASK]")) && (
                        <span style={{
                          fontSize: "0.68rem",
                          padding: "0.1rem 0.45rem",
                          borderRadius: "6px",
                          background: "rgba(234, 179, 8, 0.2)",
                          color: "#fef08a",
                          border: "1px solid rgba(234, 179, 8, 0.4)",
                          fontWeight: 700
                        }}>
                          ⚡ TASK
                        </span>
                      )}
                      {(msg.kind === "wake" || msg.body.includes("[WAKE]")) && (
                        <span style={{
                          fontSize: "0.68rem",
                          padding: "0.1rem 0.45rem",
                          borderRadius: "6px",
                          background: "rgba(16, 185, 129, 0.2)",
                          color: "#a7f3d0",
                          border: "1px solid rgba(16, 185, 129, 0.4)",
                          fontWeight: 700
                        }}>
                          🔔 WAKE
                        </span>
                      )}

                      {/* Recipient Badge */}
                      {msg.to_agent && (
                        <span style={{
                          fontSize: "0.68rem",
                          padding: "0.1rem 0.45rem",
                          borderRadius: "6px",
                          background: "rgba(99, 102, 241, 0.15)",
                          color: "#c7d2fe",
                          border: "1px solid rgba(99, 102, 241, 0.3)"
                        }}>
                          To: {msg.to_agent === "team" ? "All Team" : msg.to_agent}
                        </span>
                      )}

                      <span style={{ fontSize: "0.75rem", color: "var(--text-muted)", marginLeft: "auto" }}>{formattedTime}</span>
                    </div>

                    {editingId === msg.id ? (
                      <div style={{ display: "flex", flexDirection: "column", gap: "0.5rem" }}>
                        <textarea
                          autoFocus
                          rows={3}
                          value={editDraft}
                          onChange={e => setEditDraft(e.target.value)}
                          onKeyDown={e => {
                            if (e.key === "Escape") {
                              e.preventDefault();
                              cancelEdit();
                            } else if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
                              e.preventDefault();
                              saveEdit();
                            }
                          }}
                          style={{
                            width: "100%",
                            background: "rgba(0,0,0,0.4)",
                            border: "1px solid var(--primary)",
                            borderRadius: "10px",
                            padding: "0.7rem 0.9rem",
                            color: "#fff",
                            fontSize: "0.95rem",
                            outline: "none",
                            resize: "none"
                          }}
                        />
                        <div style={{ display: "flex", gap: "0.5rem" }}>
                          <button
                            className="btn-primary"
                            onClick={saveEdit}
                            disabled={!editDraft.trim() || mutating}
                            style={{ padding: "0.35rem 0.9rem", fontSize: "0.8rem" }}
                          >
                            Save
                          </button>
                          <button
                            className="btn-secondary"
                            onClick={cancelEdit}
                            style={{ padding: "0.35rem 0.9rem", fontSize: "0.8rem" }}
                          >
                            Cancel
                          </button>
                        </div>
                      </div>
                    ) : (
                      <div style={{ display: "grid", gap: "0.45rem" }}>
                        {rootId && (
                          <div style={{
                            borderLeft: "2px solid rgba(99, 102, 241, 0.8)",
                            color: "var(--text-muted)",
                            fontSize: "0.78rem",
                            lineHeight: 1.35,
                            marginLeft: "0.25rem",
                            paddingLeft: "0.55rem"
                          }}>
                            ↳ Replying to {rootSender?.displayName || "an earlier message"}
                            {rootMessage ? `: ${rootMessage.body.slice(0, 96)}${rootMessage.body.length > 96 ? "…" : ""}` : ""}
                          </div>
                        )}
                        <div
                          onContextMenu={e => openMessageMenu(e, msg)}
                          onMouseEnter={() => setHoveredMessageId(msg.id)}
                          onMouseLeave={() => setHoveredMessageId(prev => (prev === msg.id ? null : prev))}
                          style={{
                            position: "relative",
                            background: "rgba(0,0,0,0.35)",
                            border: "1px solid var(--border-color)",
                            borderRadius: "12px",
                            padding: "0.85rem 1.1rem",
                            fontSize: "0.95rem",
                            lineHeight: "1.5",
                            whiteSpace: "pre-wrap",
                            wordBreak: "break-word",
                            cursor: msg.from_agent === "human" ? "context-menu" : "default",
                            boxShadow: contextMenu?.messageId === msg.id ? "0 0 0 2px var(--primary)" : "none",
                            transition: "box-shadow 0.12s ease"
                          }}
                        >
                          {msg.body}
                          {msg.from_agent === "human" && (
                            <button
                              type="button"
                              onClick={e => openMessageMenu(e, msg)}
                              title="Message actions (Edit, Delete)"
                              aria-label="Message actions"
                              style={{
                                position: "absolute",
                                top: "0.35rem",
                                right: "0.35rem",
                                width: "22px",
                                height: "22px",
                                lineHeight: "22px",
                                textAlign: "center",
                                padding: 0,
                                borderRadius: "6px",
                                border: "none",
                                background: "rgba(255,255,255,0.1)",
                                color: "var(--text-main)",
                                fontSize: "0.85rem",
                                cursor: "pointer",
                                opacity: hoveredMessageId === msg.id || contextMenu?.messageId === msg.id ? 1 : 0,
                                pointerEvents: hoveredMessageId === msg.id || contextMenu?.messageId === msg.id ? "auto" : "none",
                                transition: "opacity 0.12s ease"
                              }}
                            >
                              ⋯
                            </button>
                          )}
                        </div>
                        {!activeChannel.startsWith("dm-") && (
                          <button
                            type="button"
                            onClick={() => startReply(msg)}
                            style={{
                              justifySelf: "start",
                              background: "transparent",
                              border: "none",
                              color: "var(--text-muted)",
                              cursor: "pointer",
                              fontSize: "0.78rem",
                              padding: "0.1rem 0.2rem"
                            }}
                            title="Reply in this thread"
                          >
                            ↩ Reply
                          </button>
                        )}
                        {(linkedMemories[msg.id] || []).map(memory => (
                          <button
                            key={memory.id}
                            className="btn-secondary"
                            onClick={() => {
                              setShowMemoryDrawer(true);
                              setMemorySearch(memory.id.slice(0, 8));
                            }}
                            style={{ justifySelf: "start", padding: "0.3rem 0.55rem", fontSize: "0.75rem", color: "var(--accent)" }}
                            title="Open this durable memory in the Memory Hub"
                          >
                            🧠 {memory.title || `Memory #${memory.id.slice(0, 8)}`}
                          </button>
                        ))}
                      </div>
                    )}
                  </div>
                </div>
              );
            })
          )}
        </div>
        {jumpToLatest && (
          <button
            type="button"
            className="btn-primary"
            onClick={() => {
              const el = scrollBoxRef.current;
              if (el) el.scrollTop = el.scrollHeight;
              stickToBottomRef.current = true;
              forceScrollRef.current = false;
              setJumpToLatest(false);
            }}
            style={{
              position: "absolute",
              left: "50%",
              bottom: "0.85rem",
              transform: "translateX(-50%)",
              padding: "0.4rem 0.9rem",
              fontSize: "0.8rem",
              borderRadius: "999px",
              boxShadow: "0 8px 20px rgba(0,0,0,0.35)",
              zIndex: 2
            }}
          >
            Jump to latest
          </button>
        )}
        </div>

        {/* Slack Message Input Box */}
        <div style={{
          padding: "1.25rem 1.5rem",
          borderTop: "1px solid var(--border-color)",
          background: "rgba(2, 6, 23, 0.8)"
        }}>
          <div style={{ display: "flex", flexDirection: "column", gap: "0.75rem" }}>
            {replyTo && (
              <div style={{
                alignItems: "center",
                background: "rgba(99, 102, 241, 0.12)",
                border: "1px solid rgba(129, 140, 248, 0.3)",
                borderRadius: "8px",
                color: "var(--text-muted)",
                display: "flex",
                fontSize: "0.8rem",
                gap: "0.5rem",
                justifyContent: "space-between",
                padding: "0.45rem 0.65rem"
              }}>
                <span style={{ minWidth: 0, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                  ↩ Replying to {getAgentInfo(replyTo.fromAgent).displayName}: {replyTo.preview.slice(0, 96)}{replyTo.preview.length > 96 ? "…" : ""}
                </span>
                <button
                  type="button"
                  onClick={() => setReplyTo(null)}
                  style={{ background: "transparent", border: "none", color: "var(--text-main)", cursor: "pointer", fontSize: "0.85rem" }}
                  title="Cancel reply"
                >
                  ✕
                </button>
              </div>
            )}
            <textarea
              rows={3}
              placeholder={
                activeChannel.startsWith("dm-")
                  ? `Message ${getAgentInfo(activeChannel.replace("dm-", "")).displayName}… (Enter to send, Shift+Enter for a new line)`
                  : activeWorkSession && activeChannel === `session:${activeWorkSession.id}`
                    ? `Message work session ${activeWorkSession.name}… (Enter to send, Shift+Enter for a new line)`
                  : `Message #${activeChannel}… (Enter to send, Shift+Enter for a new line)`
              }
              value={messageInput}
              onChange={e => setMessageInput(e.target.value)}
              onKeyDown={e => {
                if (e.key !== "Enter" || e.shiftKey || e.nativeEvent.isComposing) return;
                e.preventDefault();
                handleSendMessage();
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
            <div style={{ display: "flex", flexDirection: "column", gap: "0.75rem" }}>
              <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: "1rem", flexWrap: "wrap" }}>
                {activeChannel.startsWith("dm-") ? (
                  <span style={{ fontSize: "0.85rem", color: "var(--text-muted)", fontWeight: 600 }}>
                    Direct message to {getAgentInfo(activeChannel.replace("dm-", "")).displayName}
                  </span>
                ) : activeWorkSession && activeChannel === `session:${activeWorkSession.id}` ? (
                  <div style={{ display: "flex", flexDirection: "column", gap: "0.35rem" }}>
                    <span style={{ fontSize: "0.8rem", color: "var(--text-muted)", fontWeight: 600 }}>Wake selected session members:</span>
                    <div style={{ display: "flex", gap: "0.6rem", flexWrap: "wrap" }}>
                      {activeWorkSession.member_ids.filter(id => id !== "human" && id !== "system").map(agentId => (
                        <label key={agentId} style={{ display: "flex", alignItems: "center", gap: "0.3rem", fontSize: "0.8rem", color: "var(--text-muted)", cursor: "pointer" }}>
                          <input
                            type="checkbox"
                            checked={sessionWakeTargets[agentId] ?? true}
                            onChange={event => setSessionWakeTargets(previous => ({ ...previous, [agentId]: event.target.checked }))}
                          />
                          {getAgentInfo(agentId).displayName}
                        </label>
                      ))}
                    </div>
                  </div>
                ) : (
                  <div style={{ display: "flex", gap: "0.5rem", alignItems: "center", flexWrap: "wrap" }}>
                    <span style={{ fontSize: "0.8rem", color: "var(--text-muted)", fontWeight: 600 }}>Recipients:</span>
                    <button
                      type="button"
                      onClick={() => setRecipientMode("all")}
                      style={{
                        padding: "0.3rem 0.65rem",
                        borderRadius: "6px",
                        border: "1px solid var(--border-color)",
                        background: recipientMode === "all" ? "var(--primary)" : "rgba(0,0,0,0.3)",
                        color: "#fff",
                        fontSize: "0.78rem",
                        cursor: "pointer",
                        fontWeight: recipientMode === "all" ? 700 : 400
                      }}
                    >
                      🌐 All Team
                    </button>
                    <button
                      type="button"
                      onClick={() => setRecipientMode("subset")}
                      style={{
                        padding: "0.3rem 0.65rem",
                        borderRadius: "6px",
                        border: "1px solid var(--border-color)",
                        background: recipientMode === "subset" ? "var(--primary)" : "rgba(0,0,0,0.3)",
                        color: "#fff",
                        fontSize: "0.78rem",
                        cursor: "pointer",
                        fontWeight: recipientMode === "subset" ? 700 : 400
                      }}
                    >
                      👥 Subset
                    </button>
                    <button
                      type="button"
                      onClick={() => setRecipientMode("single")}
                      style={{
                        padding: "0.3rem 0.65rem",
                        borderRadius: "6px",
                        border: "1px solid var(--border-color)",
                        background: recipientMode === "single" ? "var(--primary)" : "rgba(0,0,0,0.3)",
                        color: "#fff",
                        fontSize: "0.78rem",
                        cursor: "pointer",
                        fontWeight: recipientMode === "single" ? 700 : 400
                      }}
                    >
                      🎯 Single Agent
                    </button>

                    {recipientMode === "subset" && (
                      <div style={{ display: "flex", gap: "0.6rem", flexWrap: "wrap", background: "rgba(0,0,0,0.3)", padding: "0.35rem 0.65rem", borderRadius: "8px", border: "1px solid var(--border-color)" }}>
                        {rosterAgentIds(hubAgents).filter(id => id !== "human" && id !== "system").map(agentId => (
                          <label key={agentId} style={{ display: "flex", alignItems: "center", gap: "0.3rem", fontSize: "0.8rem", color: "var(--text-main)", cursor: "pointer" }}>
                            <input
                              type="checkbox"
                              checked={selectedSubset[agentId] ?? true}
                              onChange={e => setSelectedSubset(prev => ({ ...prev, [agentId]: e.target.checked }))}
                            />
                            {getAgentInfo(agentId).displayName}
                          </label>
                        ))}
                      </div>
                    )}

                    {recipientMode === "single" && (
                      <select
                        value={singleRecipient}
                        onChange={e => setSingleRecipient(e.target.value)}
                        style={{
                          padding: "0.35rem 0.75rem",
                          borderRadius: "8px",
                          background: "rgba(0,0,0,0.4)",
                          color: "var(--text-main)",
                          border: "1px solid var(--border-color)",
                          fontSize: "0.85rem",
                          outline: "none"
                        }}
                      >
                        {rosterAgentIds(hubAgents).filter(id => id !== "human" && id !== "system").map(agentId => (
                          <option key={agentId} value={agentId}>
                            {getAgentInfo(agentId).displayName} ({getAgentInfo(agentId).role})
                          </option>
                        ))}
                      </select>
                    )}
                  </div>
                )}

                <button
                  className={sending ? "btn-secondary" : "btn-primary"}
                  onClick={handleSendMessage}
                  disabled={!messageInput.trim() || sending || activeChannel === "dm-human"}
                  style={{ padding: "0.6rem 1.5rem", fontSize: "0.9rem", marginLeft: "auto" }}
                >
                  {sending ? "Sending..." : "Send Message"}
                </button>
              </div>

              {/* Intent Tags Row */}
              <div style={{ display: "flex", alignItems: "center", gap: "0.75rem", flexWrap: "wrap", paddingTop: "0.35rem", borderTop: "1px solid rgba(255,255,255,0.06)" }}>
                <span style={{ fontSize: "0.8rem", color: "var(--text-muted)", fontWeight: 600 }}>Intent Tags:</span>
                <button
                  type="button"
                  onClick={() => setIsTaskTag(prev => !prev)}
                  style={{
                    padding: "0.3rem 0.65rem",
                    borderRadius: "6px",
                    border: isTaskTag ? "1px solid #eab308" : "1px solid var(--border-color)",
                    background: isTaskTag ? "rgba(234, 179, 8, 0.25)" : "rgba(0,0,0,0.3)",
                    color: isTaskTag ? "#fef08a" : "var(--text-muted)",
                    fontSize: "0.78rem",
                    cursor: "pointer",
                    fontWeight: isTaskTag ? 700 : 400
                  }}
                  title="Mark as Task execution request (targets existing team members only)"
                >
                  ⚡ [TASK]
                </button>
                <button
                  type="button"
                  onClick={() => setIsWakeTag(prev => !prev)}
                  style={{
                    padding: "0.3rem 0.65rem",
                    borderRadius: "6px",
                    border: isWakeTag ? "1px solid #10b981" : "1px solid var(--border-color)",
                    background: isWakeTag ? "rgba(16, 185, 129, 0.25)" : "rgba(0,0,0,0.3)",
                    color: isWakeTag ? "#a7f3d0" : "var(--text-muted)",
                    fontSize: "0.78rem",
                    cursor: "pointer",
                    fontWeight: isWakeTag ? 700 : 400
                  }}
                  title="Mark as Wake request (can wake/spawn team instances)"
                >
                  🔔 [WAKE]
                </button>

                <label style={{ display: "flex", alignItems: "center", gap: "0.4rem", fontSize: "0.8rem", color: "var(--text-muted)", cursor: "pointer", marginLeft: "auto" }}>
                  <input
                    type="checkbox"
                    checked={wakePolicyGate}
                    onChange={e => setWakePolicyGate(e.target.checked)}
                  />
                  Require Human Approval Gate
                </label>
              </div>
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

      {/* Message Context Menu (CA-106: right-click Edit / Delete) */}
      {contextMenu && (() => {
        const msg = hubMessages.find(m => m.id === contextMenu.messageId);
        if (!msg) return null;
        return (
          <div
            className="glass-card"
            onClick={e => e.stopPropagation()}
            onContextMenu={e => e.preventDefault()}
            style={{
              position: "fixed",
              top: contextMenu.y,
              left: contextMenu.x,
              zIndex: 1000,
              padding: "0.35rem",
              display: "flex",
              flexDirection: "column",
              gap: "0.15rem",
              minWidth: "140px",
              boxShadow: "0 8px 24px rgba(0,0,0,0.4)"
            }}
          >
            <button
              onClick={() => startEdit(msg)}
              style={{
                background: "transparent",
                border: "none",
                color: "var(--text-main)",
                textAlign: "left",
                padding: "0.45rem 0.6rem",
                borderRadius: "6px",
                fontSize: "0.85rem",
                cursor: "pointer"
              }}
              onMouseEnter={e => (e.currentTarget.style.background = "rgba(255,255,255,0.08)")}
              onMouseLeave={e => (e.currentTarget.style.background = "transparent")}
            >
              ✏️ Edit
            </button>
            <button
              onClick={() => deleteMessage(msg.id)}
              style={{
                background: "transparent",
                border: "none",
                color: "#f87171",
                textAlign: "left",
                padding: "0.45rem 0.6rem",
                borderRadius: "6px",
                fontSize: "0.85rem",
                cursor: "pointer"
              }}
              onMouseEnter={e => (e.currentTarget.style.background = "rgba(248,113,113,0.1)")}
              onMouseLeave={e => (e.currentTarget.style.background = "transparent")}
            >
              🗑️ Delete
            </button>
          </div>
        );
      })()}
    </div>
  );
}
