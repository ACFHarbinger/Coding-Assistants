// @ts-nocheck
export default function ChatHeader(props: any) {
  const { activeChannel, setActiveChannel, channels, creatingChannel, setCreatingChannel, newChannelName, setNewChannelName, channelActionError, createChannel, deleteChannel, channelMessages, unreadPosts, lastReadAt, workSessions, activeWorkSessionId, onSelectWorkSession, hubAgents, rosterAgentIds, getAgentInfo, memories, setShowMemoryDrawer, activeWorkSession, searchTerm, setSearchTerm, sortOrder, setSortOrder, scrollBoxRef, stickToBottomRef, forceScrollRef, setJumpToLatest, jumpToLatest, isNearBottom, filteredMessages, hoveredMessageId, setHoveredMessageId, AGENT_COLORS, editingId, editDraft, setEditDraft, saveEdit, cancelEdit, threadRootId, hubMessages, linkedMemories, startReply, openMessageMenu, contextMenu, startEdit, deleteMessage, replyTo, setReplyTo, messageInput, setMessageInput, recipientMode, setRecipientMode, selectedSubset, setSelectedSubset, singleRecipient, setSingleRecipient, teamWakeTargets, isTaskTag, setIsTaskTag, isWakeTag, setIsWakeTag, wakePolicyGate, setWakePolicyGate, handleSendMessage, sending, showMemoryDrawer, setMemorySearch, memorySearch, selectedTierFilter, setSelectedTierFilter, filteredMemories, insertMemoryLink } = props;
  return (
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
              {activeWorkSession && activeChannel === `session:${activeWorkSession.id}` ? `${activeWorkSession.member_ids.length} members · messages from the human and agent harnesses` : channels.find(c => c.id === activeChannel)?.topic || "Agent interaction stream"}
            </p>
          </div>

          {/* Search Input + sort order toggle in Header */}
          <div style={{ display: "flex", alignItems: "center", gap: "0.75rem" }}>
            <button
              type="button"
              title={sortOrder === "asc" ? "Oldest first — click for newest first" : "Newest first — click for oldest first"}
              onClick={() => setSortOrder(sortOrder === "asc" ? "desc" : "asc")}
              style={{
                display: "flex",
                alignItems: "center",
                gap: "0.35rem",
                padding: "0.45rem 0.7rem",
                borderRadius: "8px",
                background: "rgba(0,0,0,0.4)",
                border: "1px solid var(--border-color)",
                color: "#fff",
                fontSize: "0.85rem",
                cursor: "pointer"
              }}
            >
              <span>{sortOrder === "asc" ? "↑" : "↓"}</span>
              <span>{sortOrder === "asc" ? "Oldest first" : "Newest first"}</span>
            </button>
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
  );
}
