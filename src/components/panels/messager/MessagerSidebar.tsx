// @ts-nocheck
import { AgentAvatar } from "./AgentAvatar";
export default function MessagerSidebar(props: any) {
  const { activeChannel, setActiveChannel, channels, creatingChannel, setCreatingChannel, newChannelName, setNewChannelName, channelActionError, createChannel, deleteChannel, channelMessages, unreadPosts, lastReadAt, workSessions, activeWorkSessionId, onSelectWorkSession, hubAgents, rosterAgentIds, getAgentInfo, memories, setShowMemoryDrawer, activeWorkSession, searchTerm, setSearchTerm, scrollBoxRef, stickToBottomRef, forceScrollRef, setJumpToLatest, jumpToLatest, isNearBottom, filteredMessages, hoveredMessageId, setHoveredMessageId, AGENT_COLORS, editingId, editDraft, setEditDraft, saveEdit, cancelEdit, threadRootId, hubMessages, linkedMemories, startReply, openMessageMenu, contextMenu, startEdit, deleteMessage, replyTo, setReplyTo, messageInput, setMessageInput, recipientMode, setRecipientMode, selectedSubset, setSelectedSubset, singleRecipient, setSingleRecipient, teamWakeTargets, isTaskTag, setIsTaskTag, isWakeTag, setIsWakeTag, wakePolicyGate, setWakePolicyGate, handleSendMessage, sending, showMemoryDrawer, setMemorySearch, memorySearch, selectedTierFilter, setSelectedTierFilter, filteredMemories, insertMemoryLink, onRefresh } = props;
  return (
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
          <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: "0.5rem", paddingLeft: "0.5rem" }}>
            <div style={{ fontSize: "0.75rem", fontWeight: 700, color: "var(--text-muted)", letterSpacing: "0.05em", textTransform: "uppercase" }}>
              Channels
            </div>
            <button
              className="btn-secondary"
              onClick={() => { setCreatingChannel(open => !open); setChannelActionError(""); }}
              style={{ padding: "0.15rem 0.45rem", fontSize: "0.75rem" }}
              title="Create a channel"
            >
              +
            </button>
          </div>
          {creatingChannel && (
            <div style={{ display: "flex", gap: "0.35rem", marginBottom: "0.45rem" }}>
              <input
                value={newChannelName}
                onChange={event => setNewChannelName(event.target.value)}
                onKeyDown={event => { if (event.key === "Enter") void createChannel(); }}
                placeholder="new-channel"
                style={{ flex: 1, padding: "0.4rem 0.55rem", borderRadius: "8px", background: "rgba(0,0,0,0.35)", color: "white", border: "1px solid var(--border-color)", outline: "none", fontSize: "0.8rem" }}
              />
              <button className="btn-primary" onClick={() => void createChannel()} disabled={!newChannelName.trim()} style={{ padding: "0.4rem 0.6rem", fontSize: "0.75rem" }}>
                Add
              </button>
            </div>
          )}
          {channelActionError && (
            <div style={{ color: "#fca5a5", fontSize: "0.72rem", marginBottom: "0.4rem", paddingLeft: "0.5rem" }}>{channelActionError}</div>
          )}
          <div style={{ display: "flex", flexDirection: "column", gap: "0.25rem" }}>
            {channels.map(ch => {
              const isActive = activeChannel === ch.id;
              const unreadCount = isActive
                ? 0
                : unreadPosts(
                    ch.id === activeChannel ? [...hubMessages, ...channelRecords] : hubMessages,
                    ch.id,
                    lastReadAt[ch.id]
                  ).length;
              return (
                <div key={ch.id} style={{ display: "flex", alignItems: "center", gap: "0.2rem" }}>
                  <button
                    onClick={() => setActiveChannel(ch.id)}
                    style={{
                      flex: 1,
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
                  {!ch.builtin && (
                    <button
                      className="btn-secondary"
                      title={`Delete ${ch.name}`}
                      onClick={() => void deleteChannel(ch)}
                      style={{ padding: "0.25rem 0.4rem", fontSize: "0.7rem" }}
                    >
                      ×
                    </button>
                  )}
                </div>
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
                <div
                  key={agentId}
                  style={{
                    display: "flex",
                    alignItems: "center",
                    gap: "0.35rem",
                    borderRadius: "8px",
                    background: isActive ? "rgba(168, 85, 247, 0.2)" : "transparent",
                    paddingRight: "0.35rem"
                  }}
                >
                  <AgentAvatar
                    agentId={agentId}
                    displayName={info.displayName}
                    avatarAttachmentId={info.avatarAttachmentId}
                    background={info.bg}
                    size={24}
                    editable
                    onChanged={() => { void onRefresh?.(); }}
                  />
                  <button
                    onClick={() => setActiveChannel(dmId)}
                    style={{
                      flex: 1,
                      display: "flex",
                      alignItems: "center",
                      gap: "0.6rem",
                      padding: "0.5rem 0.75rem 0.5rem 0.25rem",
                      borderRadius: "8px",
                      border: "none",
                      background: "transparent",
                      color: isActive ? "#fff" : "var(--text-muted)",
                      fontWeight: isActive ? 600 : 400,
                      fontSize: "0.85rem",
                      cursor: "pointer",
                      textAlign: "left",
                      minWidth: 0
                    }}
                  >
                    <span style={{
                      width: "8px", height: "8px", borderRadius: "50%",
                      background: agentId === "human" ? "#3b82f6" : info.isRunning ? "#10b981" : "#64748b",
                      flexShrink: 0
                    }} />
                    <div style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", flex: 1 }}>
                      {info.displayName}
                    </div>
                  </button>
                </div>
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
  );
}
