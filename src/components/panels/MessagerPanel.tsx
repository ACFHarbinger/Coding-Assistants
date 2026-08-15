import { useState, useEffect, useLayoutEffect, useRef } from "react";
import { invoke, isTauriRuntime } from "../../lib/tauri";
import type { ChannelRecord, ContextMenuState, HubMessage, MemoryRecord, PendingAttachment, ReplyTarget, MessagerPanelProps, WorkspaceAgentPresence } from "./messager/types";
import { useHarnessDelivery } from "./messager/useHarnessDelivery";
import { useSendMessage } from "./messager/useSendMessage";
import { AGENT_COLORS, agentInfo, DEFAULT_CHANNELS, channelDedupeKey, isNearBottom, latestCreatedAt, loadLastRead, newestEdgeScrollTop, persistLastRead, rosterAgentIds, sortByCreatedAt, teamWakeTargets, threadRootId, uniqueChannelPosts, unreadPosts } from "./messager/utils";
import MessagerSidebar from "./messager/MessagerSidebar";
import ChatCanvas from "./messager/ChatCanvas";
import MemoryDrawer from "./messager/MemoryDrawer";
import MessageContextMenu from "./messager/MessageContextMenu";
export type { HubMessage, HubAgent, WorkSession, MemoryRecord, DetectedProcess, ChannelRecord, MessagerPanelProps } from "./messager/types";
export default function MessagerPanel({ hubMessages, hubAgents, workSessions, activeWorkSessionId, focusSessionId, focusSessionToken, workspacePath, onSelectWorkSession, onRefresh }: MessagerPanelProps) {
  const [activeChannel, setActiveChannel] = useState<string>("general");
  const [channels, setChannels] = useState<ChannelRecord[]>(
    DEFAULT_CHANNELS.map(channel => ({
      id: channel.id,
      name: channel.name,
      topic: channel.topic,
      builtin: true,
      created_at: "",
    }))
  );
  const [creatingChannel, setCreatingChannel] = useState(false);
  const [newChannelName, setNewChannelName] = useState("");
  const [channelActionError, setChannelActionError] = useState("");
  const [messageInput, setMessageInput] = useState<string>("");
  const [wakePolicyGate, setWakePolicyGate] = useState<boolean>(false);
  const [sending, setSending] = useState<boolean>(false);
  const [searchTerm, setSearchTerm] = useState<string>("");
  const [sortOrder, setSortOrder] = useState<"asc" | "desc">("desc");
  const [lastReadAt, setLastReadAt] = useState<Record<string, string>>(loadLastRead);
  const [readMarkers, setReadMarkers] = useState<{ agent_id: string; scope: string; last_read_at: string }[]>([]);
  const [channelRecords, setChannelRecords] = useState<HubMessage[]>([]);
  const [linkedMemories, setLinkedMemories] = useState<Record<string, MemoryRecord[]>>({});
  const [replyTo, setReplyTo] = useState<ReplyTarget | null>(null);
  // Canonical U12 / C10 recipient selection & intent tag state
  const [recipientMode, setRecipientMode] = useState<"all" | "subset" | "single">("all");
  const [selectedSubset, setSelectedSubset] = useState<Record<string, boolean>>({});
  const [singleRecipient, setSingleRecipient] = useState<string>("grok");
  const { harnessSessions, deliveryNotices, setDeliveryNotices, refreshHarnessSessions, retryDelivery, retryingHarness, dismissDelivery } = useHarnessDelivery(workspacePath, activeWorkSessionId);
  const [isTaskTag, setIsTaskTag] = useState<boolean>(false);
  const [isWakeTag, setIsWakeTag] = useState<boolean>(false);
  const [pendingAttachments, setPendingAttachments] = useState<PendingAttachment[]>([]);
  const [attachmentError, setAttachmentError] = useState<string>("");

  // Memories side drawer state
  const [memories, setMemories] = useState<MemoryRecord[]>([]);
  const [showMemoryDrawer, setShowMemoryDrawer] = useState<boolean>(false);
  const [memorySearch, setMemorySearch] = useState<string>("");
  const [selectedTierFilter, setSelectedTierFilter] = useState<string>("all");

  // Workspace-scoped harness liveness (never a global process-name scan)
  const [agentPresence, setAgentPresence] = useState<WorkspaceAgentPresence | null>(null);

  // Message context menu (CA-106: right-click Edit / Delete, Harbinger's posts only)
  const [contextMenu, setContextMenu] = useState<ContextMenuState | null>(null);
  const [hoveredMessageId, setHoveredMessageId] = useState<string | null>(null);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editDraft, setEditDraft] = useState<string>("");
  const [mutating, setMutating] = useState<boolean>(false);

  const scrollBoxRef = useRef<HTMLDivElement>(null);
  const stickToBottomRef = useRef(true);
  const forceScrollRef = useRef(false);
  // Set only by the sort-order toggle: land on whichever message is
  // literally first in the newly chosen order (oldest for ascending,
  // newest for descending) instead of always snapping to the newest one —
  // otherwise both orderings look identical on toggle.
  const jumpToStartRef = useRef(false);
  const prevChannelRef = useRef(activeChannel);
  const [jumpToLatest, setJumpToLatest] = useState(false);
  const activeWorkSession = workSessions.find(session => session.id === activeWorkSessionId) || null;

  useEffect(() => {
    if (!focusSessionId) return;
    setActiveChannel(`session:${focusSessionId}`);
  }, [focusSessionId, focusSessionToken]);

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

  // Fetch memories and channels
  useEffect(() => {
    async function loadHubData() {
      if (!isTauriRuntime()) return;
      try {
        const listed = await invoke<ChannelRecord[]>("hub_list_channels");
        if (listed.length > 0) setChannels(listed);
        const mems = await invoke<MemoryRecord[]>("hub_list_memories", { scope: null, tier: null });
        setMemories(mems);
      } catch (err) {
        console.error("Failed to load hub memories:", err);
      }
    }
    loadHubData();
    const interval = setInterval(loadHubData, 4000);
    return () => clearInterval(interval);
  }, []);

  useEffect(() => {
    if (!isTauriRuntime() || !workspacePath) {
      setAgentPresence(null);
      return;
    }
    let cancelled = false;
    async function loadPresence() {
      try {
        const next = await invoke<WorkspaceAgentPresence>("hub_workspace_agent_presence", {
          workspace: workspacePath,
        });
        if (!cancelled) setAgentPresence(next);
      } catch (err) {
        console.error("Failed to load workspace agent presence:", err);
        if (!cancelled) setAgentPresence(null);
      }
    }
    void loadPresence();
    const interval = setInterval(() => { void loadPresence(); }, 4000);
    return () => {
      cancelled = true;
      clearInterval(interval);
    };
  }, [workspacePath]);

  useEffect(() => {
    const pool = activeChannel.startsWith("dm-")
      ? hubMessages
      : [...hubMessages, ...channelRecords];
    const posts = uniqueChannelPosts(pool, activeChannel);
    const latest = latestCreatedAt(posts);
    if (!latest) return;
    setLastReadAt(prev => {
      if ((prev[activeChannel] || "") >= latest) return prev;
      void invoke("hub_mark_read", { agent: "human", scope: `channel:${activeChannel}` }).catch(() => {});
      return persistLastRead({ ...prev, [activeChannel]: latest });
    });
  }, [activeChannel, hubMessages, channelRecords]);

  // Read-receipt markers for the active scope — who on the team has read
  // up through a given point in time, rendered per-message in MessageStream.
  useEffect(() => {
    if (activeChannel.startsWith("dm-")) {
      setReadMarkers([]);
      return;
    }
    let cancelled = false;
    invoke<{ agent_id: string; scope: string; last_read_at: string }[]>("hub_list_read_markers", { scope: `channel:${activeChannel}` })
      .then(markers => { if (!cancelled) setReadMarkers(markers); })
      .catch(() => { if (!cancelled) setReadMarkers([]); });
    return () => { cancelled = true; };
  }, [activeChannel, hubMessages, channelRecords]);

  useEffect(() => {
    setLastReadAt(prev => {
      let changed = false;
      const next = { ...prev };
      for (const channel of channels) {
        if (next[channel.id]) continue;
        const latest = latestCreatedAt(uniqueChannelPosts(hubMessages, channel.id));
        if (!latest) continue;
        next[channel.id] = latest;
        changed = true;
      }
      return changed ? persistLastRead(next) : prev;
    });
  }, [hubMessages, channels]);

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

  const handleSendMessage = useSendMessage({
    activeChannel, activeWorkSession, hubAgents, workspacePath, messageInput, sending, setSending,
    recipientMode, selectedSubset, singleRecipient, isTaskTag, isWakeTag, wakePolicyGate, replyTo,
    pendingAttachments, setDeliveryNotices, refreshHarnessSessions, setMessageInput, setReplyTo,
    setPendingAttachments, forceScrollRef, stickToBottomRef, onRefresh,
  });

  const startReply = (message: HubMessage) => {
    const rootId = threadRootId(message, activeChannel) || message.id;
    setReplyTo({
      id: rootId,
      fromAgent: message.from_agent,
      preview: message.body,
    });
  };

  const createChannel = async () => {
    if (!newChannelName.trim() || !isTauriRuntime()) return;
    try {
      const created = await invoke<ChannelRecord>("hub_create_channel", {
        name: newChannelName.trim(),
        topic: null,
      });
      setChannels(prev => prev.some(channel => channel.id === created.id)
        ? prev.map(channel => channel.id === created.id ? created : channel)
        : [...prev, created]);
      setNewChannelName("");
      setCreatingChannel(false);
      setChannelActionError("");
      setActiveChannel(created.id);
    } catch (error) {
      setChannelActionError(String(error));
    }
  };

  const deleteChannel = async (channel: ChannelRecord) => {
    if (channel.builtin || !isTauriRuntime()) return;
    if (!window.confirm(`Delete #${channel.id}? Messages stay in the hub but the channel leaves the sidebar.`)) return;
    try {
      await invoke("hub_delete_channel", { id: channel.id });
      setChannels(prev => prev.filter(item => item.id !== channel.id));
      setChannelActionError("");
      if (activeChannel === channel.id) setActiveChannel("general");
    } catch (error) {
      setChannelActionError(String(error));
    }
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

  const getAgentInfo = (agentId: string) => agentInfo(agentId, hubAgents, agentPresence);

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
      return sortByCreatedAt(matches, sortOrder);
    }

    const seen = new Set<string>();
    const deduped = matches.filter(msg => {
      const key = channelDedupeKey(msg, activeChannel);
      if (seen.has(key)) return false;
      seen.add(key);
      return true;
    });
    return sortByCreatedAt(deduped, sortOrder);
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
    if (jumpToStartRef.current) {
      // The array's own first element — oldest message when ascending,
      // newest when descending — is what "start" means for this order.
      el.scrollTop = 0;
      jumpToStartRef.current = false;
      stickToBottomRef.current = sortOrder === "desc";
      forceScrollRef.current = false;
      setJumpToLatest(false);
    } else if (channelChanged || forceScrollRef.current || stickToBottomRef.current) {
      el.scrollTop = newestEdgeScrollTop(el, sortOrder);
      stickToBottomRef.current = true;
      forceScrollRef.current = false;
      setJumpToLatest(false);
    } else {
      setJumpToLatest(true);
    }
  }, [threadKey, activeChannel, sortOrder]);

  const viewProps = { activeChannel, setActiveChannel, channels, creatingChannel, setCreatingChannel, newChannelName, setNewChannelName, channelActionError, createChannel, deleteChannel, channelMessages, unreadPosts, lastReadAt, readMarkers, workSessions, activeWorkSessionId, onSelectWorkSession, hubAgents, rosterAgentIds, getAgentInfo, memories, setShowMemoryDrawer, activeWorkSession, searchTerm, setSearchTerm, sortOrder, setSortOrder, scrollBoxRef, stickToBottomRef, forceScrollRef, jumpToStartRef, setJumpToLatest, jumpToLatest, isNearBottom, hoveredMessageId, setHoveredMessageId, AGENT_COLORS, editingId, editDraft, setEditDraft, saveEdit, cancelEdit, threadRootId, hubMessages, linkedMemories, startReply, openMessageMenu, contextMenu, startEdit, deleteMessage, replyTo, setReplyTo, messageInput, setMessageInput, recipientMode, setRecipientMode, selectedSubset, setSelectedSubset, singleRecipient, setSingleRecipient, teamWakeTargets, isTaskTag, setIsTaskTag, isWakeTag, setIsWakeTag, wakePolicyGate, setWakePolicyGate, handleSendMessage, sending, pendingAttachments, setPendingAttachments, attachmentError, setAttachmentError, showMemoryDrawer, setMemorySearch, memorySearch, selectedTierFilter, setSelectedTierFilter, harnessSessions, workspacePath, deliveryNotices, onRetryDelivery: retryDelivery, onDismissDelivery: dismissDelivery, retryingHarness, onRefresh };

  return (
    <div style={{ display: "grid", gridTemplateColumns: showMemoryDrawer ? "260px 1fr 340px" : "260px 1fr", height: "calc(100vh - 120px)", gap: "1rem", color: "var(--text-main)", fontFamily: "'Inter', sans-serif" }}>
      <MessagerSidebar {...viewProps} />
      <ChatCanvas {...viewProps} />
      <MemoryDrawer {...viewProps} />
      <MessageContextMenu {...viewProps} />
    </div>
  );
}
