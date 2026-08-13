// @ts-nocheck -- the view consumes the HubPanel's local interaction contract.
import TaskTab from "../../TaskTab";
import DashboardPanel from "../DashboardPanel";
import { UsageChart, QuotaChart, cardStyle, inputStyle } from "./HubCharts";

export default function HubPanelView(props: any) {
  const { hubTab, dataDir, error, status, tabBtn, auditEvents, setAuditShowAll, auditShowAll, refreshAuditEvents, approveAudit, quarantineAudit, memories, searchQ, setSearchQ, searchMemories, refreshMemories, memTier, setMemTier, memAgent, setMemAgent, memTitle, setMemTitle, memBody, setMemBody, writeMemory, editingMemory, setEditingMemory, editTitle, setEditTitle, editBody, setEditBody, saveEditedMemory, run, invoke, agents, inboxConversation, setInboxConversation, setMsgTo, setPollTo, unreadFor, msgFrom, setMsgFrom, msgTo, msgKind, setMsgKind, msgSubject, setMsgSubject, msgBody, setMsgBody, sendMessage, pollTo, markConversationRead, refreshMessages, inboxSearch, setInboxSearch, inboxMessages, wakeTarget, setWakeTarget, wakeReason, setWakeReason, requestWake, refreshWakes, wakes, budgetAgent, setBudgetAgent, budgetLimit, setBudgetLimit, setBudget, refreshBudgets, refreshQuotas, refreshStaleQuotas, budgets, quotas, refreshingQuotaIds, refreshSingleQuota, budgetSpend, setBudgetSpend, recordSpend, resumeBudget } = props;


  return (
    <div className="glass-card fade-in" style={{ animationDelay: '0.1s' }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", gap: "1rem", flexWrap: "wrap", marginBottom: "1rem" }}>
        <h2 style={{ margin: 0, fontSize: "1.5rem", background: "linear-gradient(to right, #fff, var(--primary))", WebkitBackgroundClip: "text", WebkitTextFillColor: "transparent" }}>
          Shared Hub
        </h2>
        <div style={{ display: "flex", gap: "0.5rem", background: "rgba(0,0,0,0.2)", padding: "0.25rem", borderRadius: "10px" }}>
          {tabBtn("dashboard", "Dashboard")}
          {tabBtn("tasks", "Tasks")}
          {tabBtn("usage", "Usage")}
          {tabBtn("journal", "Journal", auditEvents.filter((e) => e.status === "pending").length)}
        </div>
      </div>

      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "1.5rem", paddingBottom: "1rem", borderBottom: "1px solid var(--border-color)" }}>
        <p style={{ fontSize: "0.85rem", color: "var(--text-muted)", margin: 0 }}>
          Data dir: <code style={{ background: "rgba(0,0,0,0.3)", padding: "0.2rem 0.5rem", borderRadius: "4px" }}>{dataDir || "…"}</code>
        </p>
        <div>
          {error && <span style={{ color: "#ef4444", fontSize: "0.85rem", background: "rgba(239, 68, 68, 0.1)", padding: "0.2rem 0.5rem", borderRadius: "4px" }}>Error: {error}</span>}
          {status && !error && <span style={{ color: "#22c55e", fontSize: "0.85rem", background: "rgba(34, 197, 94, 0.1)", padding: "0.2rem 0.5rem", borderRadius: "4px", display: "inline-flex", alignItems: "center", gap: "0.25rem" }}>
            <div style={{ width: "6px", height: "6px", borderRadius: "50%", background: "#22c55e" }} /> {status}
          </span>}
        </div>
      </div>

      {hubTab === "dashboard" && <DashboardPanel agents={agents} />}

      {hubTab === "memory" && (
        <div className="fade-in" style={{ display: "flex", flexDirection: "column", gap: "1.5rem" }}>
          <div style={{ display: "flex", gap: "0.75rem", flexWrap: "wrap", alignItems: "center", background: "rgba(0,0,0,0.2)", padding: "1rem", borderRadius: "12px", border: "1px solid var(--border-color)" }}>
            <input
              placeholder="Search memories…"
              value={searchQ}
              onChange={(e) => setSearchQ(e.target.value)}
              style={{ ...inputStyle, flex: 1, minWidth: 200 }}
              onFocus={e => e.target.style.borderColor = 'var(--primary)'}
              onBlur={e => e.target.style.borderColor = 'var(--border-color)'}
            />
            <button className="btn-secondary" onClick={searchMemories}>Search</button>
            <select value={tierFilter} onChange={(e) => setTierFilter(e.target.value)} style={inputStyle}>
              <option value="">All tiers</option>
              <option value="short_term">short_term</option>
              <option value="episodic">episodic</option>
              <option value="semantic">semantic</option>
            </select>
            <button className="btn-secondary" onClick={refreshMemories}>Refresh</button>
            <button
              className="btn-secondary"
              onClick={async () => {
                await run("compacted", () => invoke("hub_compact_short_term", { keepNewest: 20 }));
                await refreshMemories();
              }}
            >
              Compact ST
            </button>
            <button
              className="btn-secondary"
              onClick={async () => {
                const outcome = await run("export_committed", () =>
                  invoke<{ path: string; committed: boolean; detail: string }>(
                    "hub_export_markdown_git",
                    { message: null },
                  ),
                );
                if (outcome) {
                  setStatus(
                    outcome.committed
                      ? `exported + committed → ${outcome.path}`
                      : `exported → ${outcome.path} (${outcome.detail})`,
                  );
                }
              }}
            >
              Export MD + Commit
            </button>
          </div>

          <div style={{ ...cardStyle, display: "grid", gap: "1rem" }}>
            <h3 style={{ margin: 0, fontSize: "1rem", fontWeight: 600, color: "var(--text-main)" }}>Write Memory</h3>
            <div style={{ display: "flex", gap: "0.75rem", flexWrap: "wrap" }}>
              <select value={memTier} onChange={(e) => setMemTier(e.target.value)} style={inputStyle}>
                <option value="short_term">short_term</option>
                <option value="episodic">episodic</option>
                <option value="semantic">semantic</option>
              </select>
              <select value={memAgent} onChange={(e) => setMemAgent(e.target.value)} style={inputStyle}>
                {agents.map((a) => (
                  <option key={a.id} value={a.id}>{a.display_name}</option>
                ))}
              </select>
              <input
                placeholder="Title (optional)"
                value={memTitle}
                onChange={(e) => setMemTitle(e.target.value)}
                style={{ ...inputStyle, flex: 1, minWidth: 150 }}
              />
            </div>
            <textarea
              rows={3}
              placeholder="Memory body…"
              value={memBody}
              onChange={(e) => setMemBody(e.target.value)}
              style={{ ...inputStyle, resize: "vertical", fontFamily: "var(--font-sans)" }}
            />
            <div style={{ display: "flex", justifyContent: "flex-end" }}>
              <button className="btn-primary" onClick={writeMemory} disabled={!memBody.trim()}>
                Save to hub
              </button>
            </div>
          </div>

          <div style={{ display: "flex", flexDirection: "column", gap: "1rem", maxHeight: 500, overflowY: "auto", paddingRight: "0.5rem" }}>
            {memories.length === 0 && (
              <div style={{ padding: "3rem", textAlign: "center", background: "rgba(0,0,0,0.2)", borderRadius: "12px", border: "1px dashed var(--border-color)" }}>
                <p style={{ color: "var(--text-muted)", fontSize: "0.95rem", margin: 0 }}>No memories found in the hub.</p>
              </div>
            )}
            {memories.map((m) => (
              <div key={m.id} style={{ ...cardStyle, position: "relative", overflow: "hidden" }} onMouseEnter={e => e.currentTarget.style.borderColor = 'var(--primary)'} onMouseLeave={e => e.currentTarget.style.borderColor = 'var(--border-color)'}>
                <div style={{ display: "flex", justifyContent: "space-between", gap: "1rem", flexWrap: "wrap", marginBottom: "0.75rem", paddingBottom: "0.75rem", borderBottom: "1px solid var(--border-color)" }}>
                  <div style={{ flex: 1 }}>
                    {editingMemory === m.id ? (
                      <input
                        value={editTitle}
                        onChange={(e) => setEditTitle(e.target.value)}
                        placeholder="Memory title (optional)"
                        style={{ ...inputStyle, width: "100%", marginBottom: "0.5rem" }}
                      />
                    ) : (
                      <strong style={{ fontSize: "1.1rem", color: m.stale ? "var(--text-muted)" : "var(--primary)", textDecoration: m.stale ? "line-through" : "none" }}>{m.title || "(untitled)"}</strong>
                    )}
                    <div style={{ display: "flex", gap: "0.5rem", marginTop: "0.35rem" }}>
                      <span style={{ fontSize: "0.7rem", padding: "0.1rem 0.4rem", background: "rgba(255,255,255,0.1)", borderRadius: "4px", color: "var(--text-main)" }}>{m.tier}</span>
                      <span style={{ fontSize: "0.7rem", padding: "0.1rem 0.4rem", background: m.scope === 'global' ? "rgba(16, 185, 129, 0.1)" : "rgba(56, 189, 248, 0.1)", borderRadius: "4px", color: m.scope === 'global' ? "#10b981" : "#38bdf8" }}>{m.scope}</span>
                      <span style={{ fontSize: "0.7rem", padding: "0.1rem 0.4rem", background: "rgba(168, 85, 247, 0.1)", borderRadius: "4px", color: "#a855f7" }}>{m.agent_id || "global"}</span>
                      {m.stale && <span style={{ fontSize: "0.7rem", padding: "0.1rem 0.4rem", background: "rgba(239, 68, 68, 0.1)", borderRadius: "4px", color: "#ef4444" }}>STALE</span>}
                    </div>
                  </div>
                  <div style={{ display: "flex", gap: "0.4rem", alignItems: "flex-start" }}>
                    {editingMemory === m.id ? (
                      <>
                        <button className="btn-primary" style={{ fontSize: "0.75rem", padding: "0.2rem 0.5rem" }} onClick={() => saveEditedMemory(m.id)}>Save</button>
                        <button className="btn-secondary" style={{ fontSize: "0.75rem", padding: "0.2rem 0.5rem" }} onClick={() => setEditingMemory(null)}>Cancel</button>
                      </>
                    ) : (
                      <>
                        <button className="btn-secondary" style={{ fontSize: "0.75rem", padding: "0.2rem 0.5rem" }} onClick={() => {
                          setEditingMemory(m.id);
                          setEditTitle(m.title || "");
                          setEditBody(m.body);
                        }}>Edit</button>
                        {m.tier === "short_term" && (
                          <button className="btn-secondary" style={{ fontSize: "0.75rem", padding: "0.2rem 0.5rem" }} onClick={async () => {
                            await run("promoted", () => invoke("hub_promote_memory", { id: m.id, toTier: "episodic" }));
                            await refreshMemories();
                          }}>→ episodic</button>
                        )}
                        {m.tier === "episodic" && (
                          <button className="btn-secondary" style={{ fontSize: "0.75rem", padding: "0.2rem 0.5rem" }} onClick={async () => {
                            await run("promoted", () => invoke("hub_promote_memory", { id: m.id, toTier: "semantic" }));
                            await refreshMemories();
                          }}>→ semantic</button>
                        )}
                        <button className="btn-secondary" style={{ fontSize: "0.75rem", padding: "0.2rem 0.5rem" }} onClick={async () => {
                          await run("stale", () => invoke("hub_mark_memory_stale", { id: m.id, stale: true }));
                          await refreshMemories();
                        }}>Stale</button>
                        <button className="btn-secondary" style={{ fontSize: "0.75rem", padding: "0.2rem 0.5rem", borderColor: "rgba(239, 68, 68, 0.3)", color: "#ef4444" }} onClick={async () => {
                          await run("deleted", () => invoke("hub_delete_memory", { id: m.id }));
                          await refreshMemories();
                        }}>Delete</button>
                      </>
                    )}
                  </div>
                </div>
                {editingMemory === m.id ? (
                  <textarea
                    rows={4}
                    value={editBody}
                    onChange={(e) => setEditBody(e.target.value)}
                    style={{ ...inputStyle, width: "100%", resize: "vertical", fontFamily: "var(--font-sans)" }}
                  />
                ) : (
                  <pre style={{ margin: "0", whiteSpace: "pre-wrap", fontSize: "0.9rem", color: m.stale ? "var(--text-muted)" : "var(--text-main)", fontFamily: "var(--font-sans)", lineHeight: 1.5 }}>
                    {m.body}
                  </pre>
                )}
                <div style={{ fontSize: "0.7rem", color: "var(--text-muted)", marginTop: "1rem", textAlign: "right" }}>
                  {m.created_at} · <span style={{ fontFamily: "var(--font-mono)" }}>{m.id.slice(0, 8)}</span>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {hubTab === "inbox" && (
        <div className="fade-in" style={{ display: "grid", gridTemplateColumns: "minmax(170px, 0.3fr) minmax(0, 1fr)", gap: "1rem", alignItems: "start" }}>
          <aside style={{ ...cardStyle, padding: "0.75rem", display: "grid", gap: "0.35rem" }} aria-label="Conversations">
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", padding: "0.35rem 0.5rem 0.65rem" }}><strong style={{ color: "var(--text-main)" }}>Conversations</strong><span style={{ color: "var(--text-muted)", fontSize: "0.75rem" }}>{unreadFor("all")} unread</span></div>
            {[{ id: "all", label: "All messages" }, ...agents.filter((agent) => agent.id !== "human").map((agent) => ({ id: agent.id, label: agent.display_name }))].map((conversation) => (
              <button key={conversation.id} className={inboxConversation === conversation.id ? "btn-primary" : "btn-secondary"} onClick={() => { setInboxConversation(conversation.id); if (conversation.id !== "all") { setMsgTo(conversation.id); setPollTo(conversation.id); } }} style={{ display: "flex", justifyContent: "space-between", gap: "0.5rem", textAlign: "left", padding: "0.6rem 0.7rem" }}><span>{conversation.label}</span>{unreadFor(conversation.id) > 0 && <span style={{ minWidth: 20, textAlign: "center", borderRadius: 10, background: "#ef4444", color: "white", fontSize: "0.7rem", padding: "0.1rem 0.35rem" }}>{unreadFor(conversation.id)}</span>}</button>
            ))}
          </aside>
          <section style={{ display: "flex", flexDirection: "column", gap: "1.5rem", minWidth: 0 }} aria-label="Message thread">
          <div style={{ ...cardStyle, display: "grid", gap: "1rem" }}>
            <h3 style={{ margin: 0, fontSize: "1rem", fontWeight: 600, color: "var(--text-main)" }}>Send Message / Handoff</h3>
            <div style={{ display: "flex", gap: "0.75rem", flexWrap: "wrap", alignItems: "center" }}>
              <select value={msgFrom} onChange={(e) => setMsgFrom(e.target.value)} style={inputStyle}>
                {agents.map((a) => <option key={a.id} value={a.id}>{a.display_name}</option>)}
              </select>
              <span style={{ color: "var(--text-muted)" }}>→</span>
              <select value={msgTo} onChange={(e) => { setMsgTo(e.target.value); setInboxConversation(e.target.value); }} style={inputStyle}>
                <option value="team">Team</option>
                {agents.filter((a) => a.id !== msgFrom).map((a) => <option key={a.id} value={a.id}>{a.display_name}</option>)}
              </select>
              <select value={msgKind} onChange={(e) => setMsgKind(e.target.value)} style={{ ...inputStyle, marginLeft: "auto" }}>
                <option value="message">message</option>
                <option value="handoff">handoff</option>
                <option value="system">system</option>
              </select>
            </div>
            <input value={msgSubject} onChange={(e) => setMsgSubject(e.target.value)} placeholder="Subject (optional)" style={inputStyle} />
            <textarea rows={4} value={msgBody} onChange={(e) => setMsgBody(e.target.value)} placeholder="Message body…" style={{ ...inputStyle, resize: "vertical" }} />
            <div style={{ display: "flex", justifyContent: "flex-end" }}>
              <button className="btn-primary" onClick={sendMessage} disabled={!msgBody.trim()}>Send Message</button>
            </div>
          </div>

          <div style={{ display: "flex", gap: "0.75rem", alignItems: "center", background: "rgba(0,0,0,0.2)", padding: "1rem", borderRadius: "12px", border: "1px solid var(--border-color)", flexWrap: "wrap" }}>
            <span style={{ fontSize: "0.9rem", color: "var(--text-main)" }}>Poll inbox for:</span>
            <select value={pollTo} onChange={(e) => setPollTo(e.target.value)} style={inputStyle}>
              {agents.map((a) => <option key={a.id} value={a.id}>{a.display_name}</option>)}
            </select>
            <button className="btn-secondary" onClick={markConversationRead}>Mark unread as read</button>
            <button className="btn-secondary" onClick={refreshMessages}>Refresh List</button>
            <input
              type="text"
              placeholder="Search inbox..."
              value={inboxSearch}
              onChange={(e) => setInboxSearch(e.target.value)}
              style={{ ...inputStyle, marginLeft: "auto", minWidth: "160px" }}
            />
          </div>

          <div style={{ display: "flex", flexDirection: "column", gap: "1rem", maxHeight: 400, overflowY: "auto", paddingRight: "0.5rem" }}>
            {inboxMessages.length === 0 && (
              <div style={{ padding: "3rem", textAlign: "center", background: "rgba(0,0,0,0.2)", borderRadius: "12px", border: "1px dashed var(--border-color)" }}>
                <p style={{ color: "var(--text-muted)", fontSize: "0.95rem", margin: 0 }}>Inbox is empty or no messages match filter.</p>
              </div>
            )}
            {inboxMessages.map((m) => (
              <div key={m.id} style={cardStyle}>
                <div style={{ display: "flex", justifyContent: "space-between", marginBottom: "0.75rem", paddingBottom: "0.75rem", borderBottom: "1px solid var(--border-color)" }}>
                  <div style={{ fontSize: "0.95rem" }}>
                    <strong style={{ color: "var(--primary)" }}>{m.from_agent}</strong> <span style={{ color: "var(--text-muted)" }}>→</span> <strong>{m.to_agent}</strong>
                  </div>
                  <div style={{ display: "flex", gap: "0.5rem" }}>
                    <span style={{ fontSize: "0.7rem", padding: "0.1rem 0.4rem", background: "rgba(255,255,255,0.1)", borderRadius: "4px" }}>{m.kind}</span>
                    <span style={{ fontSize: "0.7rem", padding: "0.1rem 0.4rem", background: m.status === 'read' ? "rgba(34, 197, 94, 0.1)" : "rgba(234, 179, 8, 0.1)", color: m.status === 'read' ? "#22c55e" : "#eab308", borderRadius: "4px" }}>{m.status}</span>
                  </div>
                </div>
                {m.subject && <div style={{ fontWeight: 600, marginBottom: "0.5rem", color: "var(--text-main)" }}>{m.subject}</div>}
                <pre style={{ margin: 0, whiteSpace: "pre-wrap", fontSize: "0.9rem", color: "var(--text-main)", fontFamily: "var(--font-sans)", lineHeight: 1.5 }}>
                  {m.body}
                </pre>
              </div>
            ))}
          </div>
          </section>
        </div>
      )}

      {hubTab === "wakes" && (
        <div className="fade-in" style={{ display: "flex", flexDirection: "column", gap: "1.5rem" }}>
          <div style={{ ...cardStyle, display: "grid", gap: "1rem" }}>
            <h3 style={{ margin: 0, fontSize: "1rem", fontWeight: 600, color: "var(--text-main)" }}>Request Wake</h3>
            <div style={{ display: "flex", gap: "0.75rem", flexWrap: "wrap" }}>
              <select value={wakeTarget} onChange={(e) => setWakeTarget(e.target.value)} style={inputStyle}>
                {agents.map((a) => <option key={a.id} value={a.id}>{a.display_name}</option>)}
              </select>
              <input
                style={{ ...inputStyle, flex: 1, minWidth: 200 }}
                placeholder="Reason for waking..."
                value={wakeReason}
                onChange={(e) => setWakeReason(e.target.value)}
              />
              <button className="btn-primary" onClick={requestWake}>Wake (human gate)</button>
              <button className="btn-secondary" onClick={refreshWakes}>Refresh List</button>
            </div>
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: "1rem", maxHeight: 400, overflowY: "auto", paddingRight: "0.5rem" }}>
            {wakes.length === 0 && (
              <div style={{ padding: "3rem", textAlign: "center", background: "rgba(0,0,0,0.2)", borderRadius: "12px", border: "1px dashed var(--border-color)" }}>
                <p style={{ color: "var(--text-muted)", fontSize: "0.95rem", margin: 0 }}>No wakes recorded.</p>
              </div>
            )}
            {wakes.map((w) => (
              <div key={w.id} style={{ ...cardStyle, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <div>
                  <div style={{ marginBottom: "0.25rem", fontSize: "1.05rem", fontWeight: 600, color: "var(--primary)" }}>
                    {w.target_agent}
                  </div>
                  <div style={{ fontSize: "0.9rem", color: "var(--text-main)" }}>
                    {w.reason || <span style={{ color: "var(--text-muted)", fontStyle: "italic" }}>No reason provided</span>}
                  </div>
                  <div style={{ fontSize: "0.75rem", color: "var(--text-muted)", marginTop: "0.5rem" }}>
                    {w.created_at}
                  </div>
                </div>
                <div style={{ display: "flex", flexDirection: "column", alignItems: "flex-end", gap: "0.5rem" }}>
                  <span style={{ fontSize: "0.75rem", padding: "0.2rem 0.6rem", background: "rgba(255,255,255,0.1)", borderRadius: "20px" }}>
                    {w.status}
                  </span>
                  {w.requires_human_gate && (
                    <span style={{ fontSize: "0.7rem", color: "#eab308", display: "flex", alignItems: "center", gap: "0.25rem" }}>
                      <span style={{ display: "inline-block", width: "6px", height: "6px", background: "#eab308", borderRadius: "50%" }} />
                      Human Gate Required
                    </span>
                  )}
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
      {hubTab === "journal" && (
        <div className="fade-in" style={{ display: "flex", flexDirection: "column", gap: "1.5rem" }}>
          <div style={{ ...cardStyle, display: "flex", alignItems: "center", justifyContent: "space-between", gap: "1rem", flexWrap: "wrap" }}>
            <div>
              <h3 style={{ margin: 0, fontSize: "1rem", fontWeight: 600, color: "var(--text-main)" }}>Pending Audit Events</h3>
              <p style={{ margin: "0.35rem 0 0 0", fontSize: "0.8rem", color: "var(--text-muted)" }}>
                Filesystem changes observed by <code>ca audit watch</code>, awaiting owner review.
              </p>
            </div>
            <div style={{ display: "flex", gap: "0.75rem", alignItems: "center" }}>
              <label style={{ display: "flex", alignItems: "center", gap: "0.4rem", fontSize: "0.85rem", color: "var(--text-muted)", cursor: "pointer" }}>
                <input
                  type="checkbox"
                  checked={auditShowAll}
                  onChange={(e) => setAuditShowAll(e.target.checked)}
                />
                Show all (not just pending)
              </label>
              <button className="btn-secondary" onClick={refreshAuditEvents}>Refresh List</button>
            </div>
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: "1rem", maxHeight: 400, overflowY: "auto", paddingRight: "0.5rem" }}>
            {auditEvents.length === 0 && (
              <div style={{ padding: "3rem", textAlign: "center", background: "rgba(0,0,0,0.2)", borderRadius: "12px", border: "1px dashed var(--border-color)" }}>
                <p style={{ color: "var(--text-muted)", fontSize: "0.95rem", margin: 0 }}>
                  {auditShowAll ? "No audit events recorded." : "No pending audit events."}
                </p>
              </div>
            )}
            {auditEvents.map((event) => (
              <div key={event.id} style={{ ...cardStyle, display: "flex", justifyContent: "space-between", alignItems: "center", gap: "1rem", flexWrap: "wrap" }}>
                <div style={{ minWidth: 0 }}>
                  <div style={{ marginBottom: "0.25rem", fontSize: "0.95rem", fontWeight: 600, color: "var(--primary)", fontFamily: "var(--font-mono)", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                    {event.path}
                  </div>
                  <div style={{ fontSize: "0.85rem", color: "var(--text-main)" }}>
                    {event.operation} · {event.observed_at}
                  </div>
                  <div style={{ fontSize: "0.75rem", color: "var(--text-muted)", marginTop: "0.35rem" }}>
                    root: {event.root_path}
                  </div>
                </div>
                <div style={{ display: "flex", alignItems: "center", gap: "0.5rem", flexShrink: 0 }}>
                  <span style={{
                    fontSize: "0.75rem",
                    padding: "0.2rem 0.6rem",
                    borderRadius: "20px",
                    background: event.status === "pending" ? "rgba(234, 179, 8, 0.15)" : event.status === "quarantined" ? "rgba(239, 68, 68, 0.15)" : "rgba(255,255,255,0.1)",
                    color: event.status === "pending" ? "#eab308" : event.status === "quarantined" ? "#ef4444" : "var(--text-muted)"
                  }}>
                    {event.status}
                  </span>
                  {event.status === "pending" && (
                    <>
                      <button className="btn-primary" onClick={() => approveAudit(event.id)}>Approve</button>
                      <button className="btn-secondary" onClick={() => quarantineAudit(event.id)}>Quarantine</button>
                    </>
                  )}
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
      {hubTab === "usage" && (
        <div className="fade-in" style={{ display: "flex", flexDirection: "column", gap: "1.5rem" }}>
          <div style={{ ...cardStyle, display: "grid", gap: "1rem" }}>
            <h3 style={{ margin: 0, fontSize: "1.2rem", color: "var(--text-main)" }}>Agent Usage</h3>
            <p style={{ margin: 0, color: "var(--text-muted)", fontSize: "0.9rem" }}>
              Configure caller-defined units such as provider calls, tokens, or spend. Reaching a limit blocks new wakes.
            </p>
            <div style={{ display: "flex", gap: "0.75rem", flexWrap: "wrap" }}>
              <select value={budgetAgent} onChange={(e) => setBudgetAgent(e.target.value)} style={inputStyle}>
                <option value="">Select agent</option>
                {agents.map((agent) => <option key={agent.id} value={agent.id}>{agent.display_name}</option>)}
              </select>
              <input type="number" min="0" step="1" value={budgetLimit} onChange={(e) => setBudgetLimit(e.target.value)} style={{ ...inputStyle, width: 130 }} placeholder="Limit" />
              <button className="btn-primary" onClick={setBudget} disabled={!budgetAgent}>Set / reset budget</button>
              <button className="btn-secondary" onClick={refreshBudgets}>Refresh</button>
              <button className="btn-secondary" onClick={refreshQuotas}>Refresh provider quotas</button>
              <button className="btn-secondary" onClick={refreshStaleQuotas}>Refresh all stale quotas</button>
            </div>
          </div>
          <UsageChart budgets={budgets} />
          <QuotaChart quotas={quotas} refreshingIds={refreshingQuotaIds} onRefreshOne={refreshSingleQuota} />
          <div style={{ display: "flex", flexDirection: "column", gap: "1rem" }}>
            {budgets.length === 0 && <p style={{ color: "var(--text-muted)" }}>No budgets configured.</p>}
            {budgets.map((budget) => (
              <div key={budget.agent_id} style={{ ...cardStyle, display: "flex", justifyContent: "space-between", gap: "1rem", alignItems: "center", flexWrap: "wrap" }}>
                <div>
                  <strong style={{ color: "var(--primary)" }}>{budget.agent_id}</strong>
                  <div style={{ color: "var(--text-muted)", fontSize: "0.85rem" }}>
                    {budget.spent_units} / {budget.limit_units} units · {budget.paused ? "paused" : "active"}
                  </div>
                </div>
                <div style={{ display: "flex", gap: "0.5rem", alignItems: "center" }}>
                  <input type="number" min="0" step="1" value={budgetSpend} onChange={(e) => setBudgetSpend(e.target.value)} style={{ ...inputStyle, width: 90 }} />
                  <button className="btn-secondary" onClick={() => recordSpend(budget.agent_id)}>Record usage</button>
                  {budget.paused && <button className="btn-primary" onClick={() => resumeBudget(budget.agent_id)}>Resume</button>}
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {hubTab === "tasks" && <TaskTab />}
    </div>
  );
}
