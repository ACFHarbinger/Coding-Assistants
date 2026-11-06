// @ts-nocheck
import { AgentAvatar } from "./AgentAvatar";
import { TeamRoleBadge } from "../../common/TeamRoleBadge";
import { NOC_COLORS, NOC_SURFACES } from "../../common/telemetry/tokens";
export default function ChatHeader(props: any) {
  const { activeChannel, setActiveChannel, channels, creatingChannel, setCreatingChannel, newChannelName, setNewChannelName, channelActionError, createChannel, deleteChannel, channelMessages, unreadPosts, lastReadAt, workSessions, activeWorkSessionId, onSelectWorkSession, hubAgents, rosterAgentIds, getAgentInfo, memories, setShowMemoryDrawer, activeWorkSession, searchTerm, setSearchTerm, sortOrder, setSortOrder, scrollBoxRef, jumpToStartRef, stickToBottomRef, setJumpToLatest, jumpToLatest, isNearBottom, filteredMessages, hoveredMessageId, setHoveredMessageId, AGENT_COLORS, editingId, editDraft, setEditDraft, saveEdit, cancelEdit, threadRootId, hubMessages, linkedMemories, startReply, openMessageMenu, contextMenu, startEdit, deleteMessage, replyTo, setReplyTo, messageInput, setMessageInput, recipientMode, setRecipientMode, selectedSubset, setSelectedSubset, singleRecipient, setSingleRecipient, teamWakeTargets, isTaskTag, setIsTaskTag, isWakeTag, setIsWakeTag, wakePolicyGate, setWakePolicyGate, handleSendMessage, sending, showMemoryDrawer, setMemorySearch, memorySearch, selectedTierFilter, setSelectedTierFilter, filteredMemories, insertMemoryLink } = props;
  return (
        <div style={{
          padding: "0.9rem 1.35rem",
          borderBottom: "1px solid rgba(148, 163, 184, 0.16)",
          background: "linear-gradient(180deg, rgba(15, 23, 42, 0.9) 0%, rgba(9, 14, 26, 0.94) 100%)",
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          gap: "0.75rem",
          flexWrap: "wrap",
        }}>
          <div>
            <h2 style={{ margin: 0, fontSize: "1.05rem", fontWeight: 700, color: NOC_SURFACES.textPrimary, display: "flex", alignItems: "center", gap: "0.5rem" }}>
              {activeChannel.startsWith("dm-") ? (() => {
                const peer = getAgentInfo(activeChannel.replace("dm-", ""));
                return (
                  <>
                    <AgentAvatar
                      agentId={activeChannel.replace("dm-", "")}
                      displayName={peer.displayName}
                      avatarAttachmentId={peer.avatarAttachmentId}
                      background={peer.bg}
                      size={28}
                    />
                    <span>Direct Message: {peer.displayName}</span>
                    {peer.assignedRole && (
                      <TeamRoleBadge role={peer.assignedRole} size="normal" />
                    )}
                  </>
                );
              })() : (
                <span style={{ color: activeWorkSession && activeChannel === `session:${activeWorkSession.id}` ? NOC_COLORS.cyan : NOC_SURFACES.textPrimary }}>
                  {activeWorkSession && activeChannel === `session:${activeWorkSession.id}` ? `◈ Work session: ${activeWorkSession.name}` : `#${activeChannel}`}
                </span>
              )}
            </h2>
            <p style={{ fontSize: "0.78rem", color: NOC_SURFACES.textMuted, margin: "0.2rem 0 0 0" }}>
              {activeWorkSession && activeChannel === `session:${activeWorkSession.id}` ? `${activeWorkSession.member_ids.length} members · messages from the human and agent harnesses` : channels.find(c => c.id === activeChannel)?.topic || "Agent interaction stream"}
            </p>
          </div>

          {/* Search Input + sort order toggle in Header */}
          <div style={{ display: "flex", alignItems: "center", gap: "0.65rem" }}>
            <button
              type="button"
              className="noc-focusable"
              title={sortOrder === "asc" ? "Oldest first — click for newest first" : "Newest first — click for oldest first"}
              onClick={() => {
                if (jumpToStartRef) jumpToStartRef.current = true;
                setSortOrder(sortOrder === "asc" ? "desc" : "asc");
              }}
              style={{
                display: "flex",
                alignItems: "center",
                gap: "0.35rem",
                padding: "0.4rem 0.7rem",
                borderRadius: "8px",
                background: "rgba(9, 14, 26, 0.75)",
                border: "1px solid rgba(148, 163, 184, 0.22)",
                color: NOC_SURFACES.textPrimary,
                fontSize: "0.8rem",
                fontWeight: 600,
                cursor: "pointer"
              }}
            >
              <span style={{ color: NOC_COLORS.cyan }}>{sortOrder === "asc" ? "↑" : "↓"}</span>
              <span>{sortOrder === "asc" ? "Oldest first" : "Newest first"}</span>
            </button>
            <input
              type="text"
              className="noc-focusable"
              placeholder="Search chat..."
              value={searchTerm}
              onChange={e => setSearchTerm(e.target.value)}
              style={{
                padding: "0.4rem 0.75rem",
                borderRadius: "8px",
                background: "rgba(9, 14, 26, 0.75)",
                border: "1px solid rgba(148, 163, 184, 0.22)",
                color: NOC_SURFACES.textPrimary,
                fontSize: "0.82rem",
                outline: "none"
              }}
            />
          </div>
        </div>
  );
}
