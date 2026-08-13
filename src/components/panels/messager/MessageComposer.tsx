// @ts-nocheck
export default function MessageComposer(props: any) {
  const { activeChannel, activeWorkSession, searchTerm, setSearchTerm, scrollBoxRef, stickToBottomRef, forceScrollRef, setJumpToLatest, jumpToLatest, isNearBottom, filteredMessages, hoveredMessageId, setHoveredMessageId, getAgentInfo, AGENT_COLORS, editingId, editDraft, setEditDraft, saveEdit, cancelEdit, threadRootId, hubMessages, linkedMemories, setShowMemoryDrawer, setMemorySearch, startReply, openMessageMenu, replyTo, setReplyTo, messageInput, setMessageInput, recipientMode, setRecipientMode, selectedSubset, setSelectedSubset, singleRecipient, setSingleRecipient, rosterAgentIds, hubAgents, isTaskTag, setIsTaskTag, isWakeTag, setIsWakeTag, wakePolicyGate, setWakePolicyGate, handleSendMessage, sending } = props;
  return (
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
                        {(activeWorkSession && activeChannel === `session:${activeWorkSession.id}` ? activeWorkSession.member_ids : rosterAgentIds(hubAgents)).filter(id => id !== "human" && id !== "system").map(agentId => (
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
                        {(activeWorkSession && activeChannel === `session:${activeWorkSession.id}` ? activeWorkSession.member_ids : rosterAgentIds(hubAgents)).filter(id => id !== "human" && id !== "system").map(agentId => (
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
  );
}
