// @ts-nocheck
export default function MessageComposer(props: any) {
  const { activeChannel, activeWorkSession, searchTerm, setSearchTerm, scrollBoxRef, stickToBottomRef, forceScrollRef, setJumpToLatest, jumpToLatest, isNearBottom, filteredMessages, hoveredMessageId, setHoveredMessageId, getAgentInfo, AGENT_COLORS, editingId, editDraft, setEditDraft, saveEdit, cancelEdit, threadRootId, hubMessages, linkedMemories, setShowMemoryDrawer, setMemorySearch, startReply, openMessageMenu, replyTo, setReplyTo, messageInput, setMessageInput, recipientMode, setRecipientMode, selectedSubset, setSelectedSubset, singleRecipient, setSingleRecipient, rosterAgentIds, hubAgents, isTaskTag, setIsTaskTag, isWakeTag, setIsWakeTag, wakePolicyGate, setWakePolicyGate, handleSendMessage, sending } = props;
  const recipients = (activeWorkSession && activeChannel === `session:${activeWorkSession.id}`
    ? activeWorkSession.member_ids
    : rosterAgentIds(hubAgents)
  ).filter((id: string) => id !== "human" && id !== "system");
  const selectionStyle = (active: boolean, accent: "violet" | "teal" | "amber" = "violet"): React.CSSProperties => {
    const colors = accent === "amber"
      ? { border: "#facc15", background: "#854d0e", text: "#fefce8" }
      : accent === "teal"
        ? { border: "#5eead4", background: "#065f46", text: "#ecfdf5" }
        : { border: "#c4b5fd", background: "#6d28d9", text: "#fff" };
    return {
      padding: "0.4rem 0.75rem", borderRadius: "7px",
      border: active ? `2px solid ${colors.border}` : "1px solid #475569",
      background: active ? colors.background : "#111827", color: active ? colors.text : "#cbd5e1",
      boxShadow: active ? `0 0 0 2px ${colors.border}33` : "none",
      fontSize: "0.8rem", cursor: "pointer", fontWeight: active ? 800 : 600,
    };
  };
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
                      aria-pressed={recipientMode === "all"}
                      style={selectionStyle(recipientMode === "all")}
                    >
                      🌐 All Team
                    </button>
                    <button
                      type="button"
                      onClick={() => setRecipientMode("subset")}
                      aria-pressed={recipientMode === "subset"}
                      style={selectionStyle(recipientMode === "subset")}
                    >
                      👥 Subset
                    </button>
                    <button
                      type="button"
                      onClick={() => setRecipientMode("single")}
                      aria-pressed={recipientMode === "single"}
                      style={selectionStyle(recipientMode === "single")}
                    >
                      🎯 Single Agent
                    </button>

                    {recipientMode === "subset" && (
                      <div aria-label="Subset recipients" style={{ display: "flex", gap: "0.45rem", flexWrap: "wrap", background: "#020617", padding: "0.45rem", borderRadius: "8px", border: "1px solid #334155" }}>
                        {recipients.map((agentId: string) => {
                          const selected = selectedSubset[agentId] !== false;
                          return <button key={agentId} type="button" aria-pressed={selected}
                            onClick={() => setSelectedSubset((prev: Record<string, boolean>) => ({ ...prev, [agentId]: !selected }))}
                            style={selectionStyle(selected, "teal")}
                          >{selected ? "✓ " : "○ "}{getAgentInfo(agentId).displayName}</button>;
                        })}
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
                        {recipients.map((agentId: string) => (
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
                  aria-pressed={isTaskTag}
                  style={selectionStyle(isTaskTag, "amber")}
                  title="Mark as Task execution request (targets existing team members only)"
                >
                  ⚡ [TASK]
                </button>
                <button
                  type="button"
                  onClick={() => setIsWakeTag(prev => !prev)}
                  aria-pressed={isWakeTag}
                  style={selectionStyle(isWakeTag, "teal")}
                  title="Mark as Wake request (can wake/spawn team instances)"
                >
                  🔔 [WAKE]
                </button>

                <button type="button" aria-pressed={wakePolicyGate}
                  onClick={() => setWakePolicyGate((previous: boolean) => !previous)}
                  style={{ ...selectionStyle(wakePolicyGate, "amber"), marginLeft: "auto" }}
                >{wakePolicyGate ? "✓ " : "○ "}Require Human Approval Gate</button>
              </div>
              {!isTaskTag && !isWakeTag && (
                <p style={{ margin: 0, color: "#94a3b8", fontSize: "0.76rem", lineHeight: 1.4 }}>
                  Plain messages are recorded in this chat only. Select <strong style={{ color: "#fef08a" }}>TASK</strong> to request delivery to an active existing harness, or <strong style={{ color: "#a7f3d0" }}>WAKE</strong> to request an eligible wake.
                </p>
              )}
            </div>
          </div>
        </div>
  );
}
