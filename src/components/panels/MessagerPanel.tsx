import { useState, useEffect, useLayoutEffect, useRef } from "react";
import { invoke, isTauriRuntime } from "../../lib/tauri";
import type { ChannelRecord, ContextMenuState, DetectedProcess, HarnessInjectResult, HubMessage, MemoryRecord, ReplyTarget, MessagerPanelProps, TaggedSendOutcome } from "./messager/types";
import type { HarnessDeliveryNotice, HarnessSessionRegistration } from "./harness/types";
import { injectNotice, isSuccessfulInject } from "./harness/types";
import { AGENT_COLORS, agentInfo, DEFAULT_CHANNELS, channelDedupeKey, isNearBottom, latestCreatedAt, loadLastRead, persistLastRead, rosterAgentIds, teamWakeTargets, threadRootId, uniqueChannelPosts, unreadPosts } from "./messager/utils";
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
  const [lastReadAt, setLastReadAt] = useState<Record<string, string>>(loadLastRead);
  const [readMarkers, setReadMarkers] = useState<{ agent_id: string; scope: string; last_read_at: string }[]>([]);
  const [channelRecords, setChannelRecords] = useState<HubMessage[]>([]);
  const [linkedMemories, setLinkedMemories] = useState<Record<string, MemoryRecord[]>>({});
  const [replyTo, setReplyTo] = useState<ReplyTarget | null>(null);
  // Canonical U12 / C10 recipient selection & intent tag state
  const [recipientMode, setRecipientMode] = useState<"all" | "subset" | "single">("all");
  const [selectedSubset, setSelectedSubset] = useState<Record<string, boolean>>({});
  const [singleRecipient, setSingleRecipient] = useState<string>("grok");
  const [harnessSessions, setHarnessSessions] = useState<HarnessSessionRegistration[]>([]);
  const [deliveryNotices, setDeliveryNotices] = useState<HarnessDeliveryNotice[]>([]);
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

  // Fetch memories and process presence
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

  const refreshHarnessSessions = async () => {
    if (!isTauriRuntime()) return;
    try {
      const listed = await invoke<HarnessSessionRegistration[]>("hub_list_harness_sessions");
      setHarnessSessions(listed);
    } catch (error) {
      console.error("Failed to list harness sessions:", error);
    }
  };

  useEffect(() => {
    void refreshHarnessSessions();
    const interval = setInterval(() => void refreshHarnessSessions(), 5000);
    return () => clearInterval(interval);
  }, [workspacePath]);

  const retryDelivery = async (notice: HarnessDeliveryNotice) => {
    if (!notice.messageId || !notice.body) return;
    try {
      const result = await invoke<HarnessInjectResult>("hub_inject_harness", {
        harness: notice.harness,
        workspace: workspacePath,
        sessionId: activeWorkSessionId,
        messageId: notice.messageId,
        body: notice.body,
        isTask: notice.isTask ?? false,
        isWake: notice.isWake ?? false,
      });
      const next = injectNotice(result.status, result.detail);
      setDeliveryNotices(current => current.map(item => item.harness === notice.harness
        ? { ...item, status: result.status, detail: result.detail, retryable: next.retryable }
        : item));
      await refreshHarnessSessions();
    } catch (error) {
      setDeliveryNotices(current => current.map(item => item.harness === notice.harness
        ? { ...item, status: "unavailable", detail: String(error), retryable: true }
        : item));
    }
  };

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

      const eligibleRecipients = sessionChannel && activeWorkSession
        ? activeWorkSession.member_ids.filter(id => id !== "human" && id !== "system")
        : enrolledRoster;
      let targetAgents: string[] = [];
      if (dmTarget) {
        targetAgents = [dmTarget];
      } else if (recipientMode === "single") {
        targetAgents = [singleRecipient];
      } else if (recipientMode === "subset") {
        // A subset starts with every eligible member selected. The old UI
        // rendered an absent key as checked but only sent explicitly present
        // keys, so an untouched subset could accidentally address nobody.
        targetAgents = eligibleRecipients.filter(id => selectedSubset[id] !== false);
        if (targetAgents.length === 0) {
          alert("Please select at least one recipient agent for subset messaging.");
          setSending(false);
          return;
        }
      } else {
        targetAgents = eligibleRecipients;
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

      // hub's MessageKind enum only knows message/handoff/wake/system — task
      // intent rides in the `task` field, subject suffix, and [TASK] body
      // prefix instead of a "task" kind, which the backend would reject.
      const messageKind = isWakeTag ? "wake" : "message";
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
        if (isTaskTag || isWakeTag) {
          if (!workspacePath.startsWith("/")) {
            throw new Error("Tagged delivery requires an absolute Workspace Root in Orchestrate");
          }
          const outcomes = await invoke<TaggedSendOutcome[]>("hub_send_tagged_message", {
            args: { from: "human", to: targetAgents, isTask: isTaskTag, isWake: isWakeTag, subject, workspace: null, task: isTaskTag ? bodyText : null, sessionId: activeWorkSession.id, body: bodyText }
          });
          const accepted = outcomes.filter(outcome => outcome.accepted && outcome.message_id);
          const injections = await Promise.allSettled(
            accepted.map(outcome => invoke<HarnessInjectResult>("hub_inject_harness", {
              harness: outcome.to_agent,
              workspace: workspacePath,
              sessionId: activeWorkSession.id,
              messageId: outcome.message_id,
              body: bodyText,
              isTask: isTaskTag,
              isWake: isWakeTag,
            }))
          );
          const notices: HarnessDeliveryNotice[] = [
            ...outcomes.filter(outcome => !outcome.accepted).map(outcome => ({
              harness: outcome.to_agent,
              status: "unavailable",
              detail: outcome.reason || "rejected",
              retryable: false,
            })),
            ...injections.map((result, index) => {
              const target = accepted[index];
              if (result.status === "rejected") {
                return {
                  harness: target?.to_agent ?? "harness",
                  status: "unavailable",
                  detail: String(result.reason),
                  retryable: true,
                  messageId: target?.message_id,
                  body: bodyText,
                  isTask: isTaskTag,
                  isWake: isWakeTag,
                };
              }
              const notice = injectNotice(result.value.status, result.value.detail);
              return {
                harness: result.value.harness,
                status: result.value.status,
                detail: result.value.detail,
                retryable: notice.retryable,
                messageId: target?.message_id,
                body: bodyText,
                isTask: isTaskTag,
                isWake: isWakeTag,
              };
            }),
          ];
          setDeliveryNotices(notices.filter(notice => !isSuccessfulInject(notice.status)));
          void refreshHarnessSessions();
        } else {
          await invoke("hub_send_session_message", {
            args: { from: "human", sessionId: activeWorkSession.id, to: targetAgents, subject, workspace: null, task: null, body: bodyText }
          });
        }
      } else if (isTaskTag || isWakeTag) {
        await invoke("hub_send_tagged_message", {
          args: {
            from: "human",
            to: targetAgents,
            isTask: isTaskTag,
            isWake: isWakeTag,
            subject,
            workspace: null,
            task: isTaskTag ? bodyText : null,
            sessionId: null,
            body: bodyText
          }
        });
      } else {
        const sentMsg = await invoke<{ id: string }>("hub_send_message", {
          args: { from: "human", to: toField, kind: messageKind, subject, workspace: null, task: isTaskTag ? bodyText : null, body: bodyText }
        });
        const wakeTargets = toField === "team" ? teamWakeTargets(hubAgents) : targetAgents;
        if (wakePolicyGate) {
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

  const getAgentInfo = (agentId: string) => agentInfo(agentId, hubAgents, runningProcesses);

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

  const viewProps = { activeChannel, setActiveChannel, channels, creatingChannel, setCreatingChannel, newChannelName, setNewChannelName, channelActionError, createChannel, deleteChannel, channelMessages, unreadPosts, lastReadAt, readMarkers, workSessions, activeWorkSessionId, onSelectWorkSession, hubAgents, rosterAgentIds, getAgentInfo, memories, setShowMemoryDrawer, activeWorkSession, searchTerm, setSearchTerm, scrollBoxRef, stickToBottomRef, forceScrollRef, setJumpToLatest, jumpToLatest, isNearBottom, hoveredMessageId, setHoveredMessageId, AGENT_COLORS, editingId, editDraft, setEditDraft, saveEdit, cancelEdit, threadRootId, hubMessages, linkedMemories, startReply, openMessageMenu, contextMenu, startEdit, deleteMessage, replyTo, setReplyTo, messageInput, setMessageInput, recipientMode, setRecipientMode, selectedSubset, setSelectedSubset, singleRecipient, setSingleRecipient, teamWakeTargets, isTaskTag, setIsTaskTag, isWakeTag, setIsWakeTag, wakePolicyGate, setWakePolicyGate, handleSendMessage, sending, showMemoryDrawer, setMemorySearch, memorySearch, selectedTierFilter, setSelectedTierFilter, harnessSessions, workspacePath, deliveryNotices, onRetryDelivery: retryDelivery, onDismissDelivery: (harness: string) => setDeliveryNotices(current => current.filter(item => item.harness !== harness)) };

  return (
    <div style={{ display: "grid", gridTemplateColumns: showMemoryDrawer ? "260px 1fr 340px" : "260px 1fr", height: "calc(100vh - 120px)", gap: "1rem", color: "var(--text-main)", fontFamily: "'Inter', sans-serif" }}>
      <MessagerSidebar {...viewProps} />
      <ChatCanvas {...viewProps} />
      <MemoryDrawer {...viewProps} />
      <MessageContextMenu {...viewProps} />
    </div>
  );
}
