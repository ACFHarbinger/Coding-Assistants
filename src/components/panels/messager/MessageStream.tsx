// @ts-nocheck
import { isNearNewestEdge, newestEdgeScrollTop } from "./utils";
export default function MessageStream(props: any) {
  const { activeChannel, channelMessages, contextMenu, mutating, scrollBoxRef, stickToBottomRef, forceScrollRef, setJumpToLatest, jumpToLatest, sortOrder, hoveredMessageId, setHoveredMessageId, getAgentInfo, editingId, editDraft, setEditDraft, saveEdit, cancelEdit, threadRootId, linkedMemories, setShowMemoryDrawer, setMemorySearch, startReply, openMessageMenu, readMarkers } = props;
  return (
        <div style={{ flex: 1, position: "relative", minHeight: 0 }}>
        <div
          ref={scrollBoxRef}
          onScroll={() => {
            const el = scrollBoxRef.current;
            if (!el) return;
            const near = isNearNewestEdge(el, sortOrder);
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
                          To: {msg.to_agent === "team"
                            ? "All Team"
                            : (msg.recipient_agents?.length
                              ? msg.recipient_agents.join(", ")
                              : msg.to_agent)}
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
                        {(() => {
                          const readers = (readMarkers || [])
                            .filter(marker => marker.agent_id !== msg.from_agent && marker.last_read_at >= msg.created_at)
                            .map(marker => getAgentInfo(marker.agent_id).displayName);
                          if (readers.length === 0) return null;
                          return (
                            <div
                              style={{
                                justifySelf: "start",
                                fontSize: "0.72rem",
                                color: "var(--text-muted)",
                                display: "flex",
                                alignItems: "center",
                                gap: "0.3rem",
                              }}
                              title={`Read by: ${readers.join(", ")}`}
                            >
                              <span style={{ color: "#6ee7b7" }}>✓✓</span> Read by {readers.join(", ")}
                            </div>
                          );
                        })()}
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
              if (el) el.scrollTop = newestEdgeScrollTop(el, sortOrder);
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
  );
}
