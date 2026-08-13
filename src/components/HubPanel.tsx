import { startTransition, useCallback, useEffect, useState } from "react";
import { invoke } from "../lib/tauri";
import TaskTab from "./panels/TaskTab";
import DashboardPanel from "./panels/DashboardPanel";

interface MemoryRecord {
  id: string;
  scope: string;
  workspace_path?: string | null;
  tier: string;
  agent_id?: string | null;
  title?: string | null;
  body: string;
  tags_json: string;
  created_at: string;
  updated_at: string;
  stale: boolean;
}

interface MessageRecord {
  id: string;
  from_agent: string;
  to_agent: string;
  kind: string;
  status: string;
  subject?: string | null;
  body: string;
  created_at: string;
}

interface WakeRecord {
  id: string;
  target_agent: string;
  reason?: string | null;
  status: string;
  requires_human_gate: boolean;
  created_at: string;
}

interface AgentRecord {
  id: string;
  display_name: string;
}

interface AuditEvent {
  id: string;
  root_path: string;
  path: string;
  operation: string;
  observed_at: string;
  process_json: string;
  content_hash?: string | null;
  previous_hash?: string | null;
  event_hash: string;
  status: string;
}

interface WakePolicy {
  default_requires_human_gate: boolean;
  allow_auto_wake: boolean;
}

interface BudgetStatus {
  agent_id: string;
  limit_units: number;
  spent_units: number;
  paused: boolean;
  updated_at: string;
}

interface ProviderQuotaWindow {
  label: string;
  family?: string | null;
  used_percent: number;
  remaining_percent: number;
  resets_at?: number | null;
  window_minutes?: number | null;
}

interface ProviderQuota {
  agent_id: string;
  provider: string;
  harness_title?: string;
  status: string;
  detail?: string | null;
  windows: ProviderQuotaWindow[];
  fetched_at: number;
}

type HubTab = "dashboard" | "memory" | "inbox" | "wakes" | "tasks" | "policy" | "usage" | "journal";

const cardStyle: React.CSSProperties = {
  border: "1px solid var(--border-color)",
  borderRadius: "12px",
  padding: "1.5rem",
  background: "rgba(0, 0, 0, 0.3)",
  boxShadow: "0 4px 6px rgba(0,0,0,0.1)",
  /* Narrow to compositable properties only — avoids layout thrash on hover. */
  transition: "border-color 0.2s ease, box-shadow 0.2s ease"
};

const inputStyle: React.CSSProperties = {
  padding: '0.75rem',
  borderRadius: '8px',
  background: 'rgba(0,0,0,0.4)',
  color: 'white',
  border: '1px solid var(--border-color)',
  outline: 'none',
  transition: 'border-color 0.2s'
};

function WakePolicyCheckbox({
  checked,
  onChange,
  title,
  description,
}: {
  checked: boolean;
  onChange: (checked: boolean) => void;
  title: string;
  description: string;
}) {
  return (
    <label style={{ display: "flex", alignItems: "flex-start", gap: "0.9rem", cursor: "pointer", padding: "0.8rem", borderRadius: "10px", border: checked ? "1px solid #a78bfa" : "1px solid rgba(100, 116, 139, 0.65)", background: checked ? "rgba(124, 58, 237, 0.18)" : "rgba(15, 23, 42, 0.58)", transition: "background 0.15s ease, border-color 0.15s ease" }}>
      <input
        type="checkbox"
        checked={checked}
        onChange={(event) => onChange(event.target.checked)}
        style={{ position: "absolute", opacity: 0, width: 1, height: 1 }}
      />
      <span aria-hidden="true" style={{ display: "grid", placeItems: "center", flex: "0 0 auto", width: "1.35rem", height: "1.35rem", marginTop: "0.1rem", borderRadius: "0.35rem", border: checked ? "2px solid #c4b5fd" : "2px solid #64748b", background: checked ? "#7c3aed" : "#0f172a", color: "#fff", fontSize: "0.95rem", fontWeight: 800, boxShadow: checked ? "0 0 0 3px rgba(167, 139, 250, 0.24)" : "inset 0 0 0 1px rgba(255, 255, 255, 0.04)", transition: "background 0.15s ease, border-color 0.15s ease, box-shadow 0.15s ease" }}>
        {checked ? "✓" : ""}
      </span>
      <span>
        <span style={{ display: "block", fontSize: "1rem", fontWeight: 600, color: checked ? "#ede9fe" : "var(--text-main)", marginBottom: "0.25rem" }}>{title}</span>
        <span style={{ display: "block", fontSize: "0.85rem", color: "var(--text-muted)", lineHeight: 1.45 }}>{description}</span>
      </span>
    </label>
  );
}

function UsageChart({ budgets }: { budgets: BudgetStatus[] }) {
  if (budgets.length === 0) return null;
  const chartWidth = 760;
  const rowHeight = 42;
  const labelWidth = 120;
  const barWidth = chartWidth - labelWidth - 150;
  return (
    <div style={{ ...cardStyle, display: "grid", gap: "0.75rem" }}>
      <div style={{ display: "flex", justifyContent: "space-between", gap: "1rem", flexWrap: "wrap" }}>
        <h3 style={{ margin: 0, color: "var(--text-main)" }}>Budget utilization</h3>
        <div style={{ display: "flex", gap: "1rem", color: "var(--text-muted)", fontSize: "0.8rem" }}>
          <span><i style={{ display: "inline-block", width: 10, height: 10, borderRadius: 2, background: "var(--primary)", marginRight: 5 }} />Used</span>
          <span><i style={{ display: "inline-block", width: 10, height: 10, borderRadius: 2, background: "#334155", marginRight: 5 }} />Available</span>
        </div>
      </div>
      <svg viewBox={`0 0 ${chartWidth} ${budgets.length * rowHeight}`} width="100%" role="img" aria-label="Used and available budget by agent" style={{ minHeight: 120 }}>
        {budgets.map((budget, index) => {
          const used = Math.min(budget.limit_units, Math.max(0, budget.spent_units));
          const usedWidth = budget.limit_units > 0 ? (used / budget.limit_units) * barWidth : 0;
          const y = index * rowHeight + 8;
          return <g key={budget.agent_id}>
            <text x="0" y={y + 17} fill="var(--text-main)" fontSize="13">{budget.agent_id}</text>
            <rect x={labelWidth} y={y} width={barWidth} height="22" rx="5" fill="#334155" />
            <rect x={labelWidth} y={y} width={usedWidth} height="22" rx="5" fill="var(--primary)" />
            <text x={labelWidth + barWidth + 12} y={y + 15} fill="var(--text-muted)" fontSize="12">{budget.spent_units} / {budget.limit_units}</text>
          </g>;
        })}
      </svg>
    </div>
  );
}

function QuotaChart({ quotas }: { quotas: ProviderQuota[] }) {
  const formatReset = (timestamp?: number | null) => timestamp
    ? `resets ${new Date(timestamp * 1000).toLocaleString()}`
    : "reset time unavailable";
  const windowName = (window: ProviderQuotaWindow) => {
    if (!window.window_minutes) return window.label;
    if (window.window_minutes <= 360) return `${window.label} · hourly window`;
    if (window.window_minutes >= 28 * 24 * 60) return `${window.label} · monthly window`;
    if (window.window_minutes >= 6 * 24 * 60) return `${window.label} · weekly window`;
    return `${window.label} · ${window.window_minutes} min`;
  };
  return (
    <div style={{ ...cardStyle, display: "grid", gap: "1rem" }}>
      <div style={{ display: "flex", justifyContent: "space-between", gap: "1rem", flexWrap: "wrap" }}>
        <div>
          <h3 style={{ margin: 0, color: "var(--text-main)" }}>Provider quota remaining</h3>
          <p style={{ margin: "0.35rem 0 0", color: "var(--text-muted)", fontSize: "0.82rem" }}>Account limits reported by each harness provider, separate from local Shared Hub budgets.</p>
        </div>
        <div style={{ display: "flex", gap: "1rem", color: "var(--text-muted)", fontSize: "0.8rem" }}>
          <span><i style={{ display: "inline-block", width: 10, height: 10, borderRadius: 2, background: "var(--primary)", marginRight: 5 }} />Remaining</span>
          <span><i style={{ display: "inline-block", width: 10, height: 10, borderRadius: 2, background: "#334155", marginRight: 5 }} />Used</span>
        </div>
      </div>
      <div style={{ display: "grid", gap: "1.25rem" }}>
        {quotas.map((quota) => {
          const families = Array.from(
            new Set(quota.windows.map((w) => w.family).filter(Boolean))
          ) as string[];

          return (
            <div key={quota.agent_id} style={{ display: "grid", gap: "0.6rem", background: "rgba(0, 0, 0, 0.2)", padding: "0.85rem 1rem", borderRadius: "10px", border: "1px solid var(--border-color)" }}>
              <div style={{ display: "flex", justifyContent: "space-between", gap: "1rem", flexWrap: "wrap", alignItems: "center" }}>
                <strong style={{ color: "var(--primary)", fontSize: "1.02rem" }}>
                  {quota.harness_title || `${quota.agent_id} · ${quota.provider}`}
                </strong>
                <span style={{ color: quota.status === "ok" ? "#22c55e" : "var(--text-muted)", fontSize: "0.82rem", fontWeight: 500 }}>
                  {quota.status === "ok" ? "live quota" : "unavailable"}
                </span>
              </div>
              {quota.windows.length === 0 ? (
                <span style={{ color: "var(--text-muted)", fontSize: "0.82rem" }}>{quota.detail || "No provider quota windows returned."}</span>
              ) : families.length > 0 ? (
                families.map((family) => {
                  const familyWindows = quota.windows.filter((w) => w.family === family);
                  return (
                    <div key={`${quota.agent_id}-${family}`} style={{ display: "grid", gap: "0.5rem", marginTop: "0.25rem" }}>
                      <div style={{ fontSize: "0.86rem", fontWeight: 600, color: "var(--text-main)", opacity: 0.9, letterSpacing: "0.02em" }}>
                        {family}
                      </div>
                      {familyWindows.map((window) => (
                        <div key={`${quota.agent_id}-${family}-${window.label}`} style={{ display: "grid", gap: "0.25rem", paddingLeft: "0.5rem" }}>
                          <div style={{ display: "flex", justifyContent: "space-between", gap: "1rem", color: "var(--text-muted)", fontSize: "0.8rem" }}>
                            <span>{windowName(window)} · {formatReset(window.resets_at)}</span>
                            <strong style={{ color: window.remaining_percent === 0 ? "#ef4444" : "var(--text-main)" }}>
                              {window.remaining_percent}% remaining
                            </strong>
                          </div>
                          <div style={{ height: 12, background: "#334155", borderRadius: 6, overflow: "hidden" }}>
                            <div
                              style={{
                                width: `${window.remaining_percent}%`,
                                height: "100%",
                                background: window.remaining_percent < 10 ? "#ef4444" : window.remaining_percent < 25 ? "#eab308" : "var(--primary)",
                                transition: "width 0.3s ease",
                              }}
                            />
                          </div>
                        </div>
                      ))}
                    </div>
                  );
                })
              ) : (
                quota.windows.map((window) => (
                  <div key={`${quota.agent_id}-${window.label}`} style={{ display: "grid", gap: "0.25rem" }}>
                    <div style={{ display: "flex", justifyContent: "space-between", gap: "1rem", color: "var(--text-muted)", fontSize: "0.8rem" }}>
                      <span>{windowName(window)} · {formatReset(window.resets_at)}</span>
                      <strong style={{ color: window.remaining_percent === 0 ? "#ef4444" : "var(--text-main)" }}>
                        {window.remaining_percent}% remaining
                      </strong>
                    </div>
                    <div style={{ height: 12, background: "#334155", borderRadius: 6, overflow: "hidden" }}>
                      <div
                        style={{
                          width: `${window.remaining_percent}%`,
                          height: "100%",
                          background: window.remaining_percent < 10 ? "#ef4444" : window.remaining_percent < 25 ? "#eab308" : "var(--primary)",
                          transition: "width 0.3s ease",
                        }}
                      />
                    </div>
                  </div>
                ))
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}

export default function HubPanel() {
  const [hubTab, setHubTab] = useState<HubTab>("dashboard");
  const [dataDir, setDataDir] = useState<string>("");
  const [error, setError] = useState<string>("");
  const [status, setStatus] = useState<string>("");

  const [memories, setMemories] = useState<MemoryRecord[]>([]);
  const [messages, setMessages] = useState<MessageRecord[]>([]);
  const [wakes, setWakes] = useState<WakeRecord[]>([]);
  const [agents, setAgents] = useState<AgentRecord[]>([]);
  const [auditEvents, setAuditEvents] = useState<AuditEvent[]>([]);
  const [auditShowAll, setAuditShowAll] = useState(false);

  const [searchQ, setSearchQ] = useState("");
  const [tierFilter, setTierFilter] = useState<string>("");
  const [memTitle, setMemTitle] = useState("");
  const [memBody, setMemBody] = useState("");
  const [memTier, setMemTier] = useState("short_term");
  const [memAgent, setMemAgent] = useState("grok");

  const [msgFrom, setMsgFrom] = useState("human");
  const [msgTo, setMsgTo] = useState("claude");
  const [msgBody, setMsgBody] = useState("");
  const [msgSubject, setMsgSubject] = useState("");
  const [msgKind, setMsgKind] = useState("message");
  const [pollTo, setPollTo] = useState("claude");
  const [inboxConversation, setInboxConversation] = useState("chat");
  const [inboxSearch, setInboxSearch] = useState("");

  const [wakeTarget, setWakeTarget] = useState("claude");
  const [wakeReason, setWakeReason] = useState("");

  const [wakePolicy, setWakePolicy] = useState<WakePolicy | null>(null);
  const [budgets, setBudgets] = useState<BudgetStatus[]>([]);
  const [quotas, setQuotas] = useState<ProviderQuota[]>([]);
  const [budgetAgent, setBudgetAgent] = useState("");
  const [budgetLimit, setBudgetLimit] = useState("100");
  const [budgetSpend, setBudgetSpend] = useState("1");

  const [editingMemory, setEditingMemory] = useState<string | null>(null);
  const [editTitle, setEditTitle] = useState("");
  const [editBody, setEditBody] = useState("");

  const run = useCallback(async <T,>(label: string, fn: () => Promise<T>): Promise<T | null> => {
    setError("");
    try {
      const result = await fn();
      setStatus(label);
      return result;
    } catch (e) {
      setError(String(e));
      return null;
    }
  }, []);

  const refreshMemories = useCallback(async () => {
    const list = await run("memories refreshed", () =>
      invoke<MemoryRecord[]>("hub_list_memories", {
        scope: null,
        tier: tierFilter || null,
        workspace: null,
        includeStale: false,
      })
    );
    if (list) setMemories(list);
  }, [run, tierFilter]);

  const refreshMessages = useCallback(async () => {
    const list = await run("inbox refreshed", () => invoke<MessageRecord[]>("hub_list_messages"));
    if (list) setMessages(list);
  }, [run]);

  const refreshWakes = useCallback(async () => {
    const list = await run("wakes refreshed", () => invoke<WakeRecord[]>("hub_list_wakes"));
    if (list) setWakes(list);
  }, [run]);

  const refreshPolicy = useCallback(async () => {
    const policy = await run("policy refreshed", () => invoke<WakePolicy>("hub_get_wake_policy"));
    if (policy) setWakePolicy(policy);
  }, [run]);

  const refreshBudgets = useCallback(async () => {
    const statuses = await Promise.all(
      agents.map(async (agent) =>
        invoke<BudgetStatus | null>("hub_get_budget", { agent: agent.id })
      ),
    );
    setBudgets(statuses.filter((status): status is BudgetStatus => status !== null));
  }, [agents]);

  const refreshQuotas = useCallback(async () => {
    const statuses = await run("provider quotas refreshed", () =>
      invoke<ProviderQuota[]>("hub_get_provider_quotas")
    );
    if (statuses) setQuotas(statuses);
  }, [run]);

  const refreshAuditEvents = useCallback(async () => {
    const list = await run("audit events refreshed", () =>
      invoke<AuditEvent[]>("hub_list_audit_events", { pendingOnly: !auditShowAll })
    );
    if (list) setAuditEvents(list);
  }, [run, auditShowAll]);

  const approveAudit = async (id: string) => {
    await run("audit event approved", () => invoke("hub_approve_audit", { id }));
    await refreshAuditEvents();
  };

  const quarantineAudit = async (id: string) => {
    await run("audit event quarantined", () => invoke("hub_quarantine_audit", { id }));
    await refreshAuditEvents();
  };

  useEffect(() => {
    invoke<string>("hub_get_data_dir").then(setDataDir).catch((e) => setError(String(e)));
    invoke<AgentRecord[]>("hub_list_agents").then((list) => {
      setAgents(list);
      if (list.length > 0) {
        setMemAgent(list[0].id);
        setMsgFrom(list[0].id);
        const firstAgent = list.find((agent) => agent.id !== "human") || list[0];
        setMsgTo(firstAgent.id);
        setPollTo(firstAgent.id);
        setInboxConversation(firstAgent.id);
        setWakeTarget(firstAgent.id);
      }
    }).catch((e) => setError(String(e)));
    invoke<AuditEvent[]>("hub_list_audit_events", { pendingOnly: true })
      .then(setAuditEvents)
      .catch((e) => console.error("Failed to load pending audit events:", e));
  }, []);

  useEffect(() => {
    if (hubTab === "memory") refreshMemories();
    else if (hubTab === "inbox") refreshMessages();
    else if (hubTab === "wakes") refreshWakes();
    else if (hubTab === "policy") refreshPolicy();
    else if (hubTab === "usage") {
      refreshBudgets();
      refreshQuotas();
    }
    else if (hubTab === "journal") refreshAuditEvents();
  }, [hubTab, refreshMemories, refreshMessages, refreshWakes, refreshPolicy, refreshBudgets, refreshQuotas, refreshAuditEvents]);

  useEffect(() => {
    if (hubTab !== "inbox") return;
    const interval = window.setInterval(() => {
      void refreshMessages();
    }, 3000);
    return () => window.clearInterval(interval);
  }, [hubTab, refreshMessages]);

  const writeMemory = async () => {
    if (!memBody.trim()) return;
    await run("memory written", () =>
      invoke("hub_write_memory", {
        tier: memTier,
        agentId: memAgent,
        title: memTitle || null,
        body: memBody,
        tags: [],
      })
    );
    setMemBody("");
    setMemTitle("");
    await refreshMemories();
  };

  const saveEditedMemory = async (id: string) => {
    if (!editBody.trim()) return;
    await run("memory updated", () =>
      invoke("hub_update_memory", {
        id,
        title: editTitle || null,
        body: editBody,
        tags: null, // Keep existing tags
      })
    );
    setEditingMemory(null);
    await refreshMemories();
  };

  const searchMemories = async () => {
    if (!searchQ.trim()) {
      await refreshMemories();
      return;
    }
    const list = await run("search done", () =>
      invoke<MemoryRecord[]>("hub_search_memories", { query: searchQ })
    );
    if (list) setMemories(list);
  };

  const sendMessage = async () => {
    if (!msgBody.trim()) return;
    await run("message sent", () =>
      invoke("hub_send_message", {
        args: {
          from: msgFrom,
          to: msgTo,
        kind: msgKind,
          subject: msgSubject.trim() || null,
          workspace: null,
          task: null,
          body: msgBody,
        },
      })
    );
    setMsgBody("");
    setMsgSubject("");
    await refreshMessages();
  };

  const requestWake = async () => {
    await run("wake requested", () =>
      invoke("hub_request_wake", {
        target: wakeTarget,
        reason: wakeReason || null,
        messageId: null,
        humanGate: true,
      })
    );
    setWakeReason("");
    await refreshWakes();
  };

  const updatePolicy = async (updates: Partial<WakePolicy>) => {
    if (!wakePolicy) return;
    const previousPolicy = wakePolicy;
    const newPolicy = { ...previousPolicy, ...updates };
    // Update immediately so this controlled input never snaps back while IPC
    // persists the choice. Roll back only when persistence actually fails.
    setWakePolicy(newPolicy);
    const savedPolicy = await run("policy updated", () =>
      invoke<WakePolicy>("hub_set_wake_policy", { policy: newPolicy })
    );
    setWakePolicy(savedPolicy ?? previousPolicy);
  };

  const setBudget = async () => {
    if (!budgetAgent || !Number.isFinite(Number(budgetLimit))) return;
    await run("budget saved", () => invoke("hub_set_agent_budget", {
      agent: budgetAgent,
      limit: Number(budgetLimit),
    }));
    await refreshBudgets();
  };

  const recordSpend = async (agent: string) => {
    const amount = Number(budgetSpend);
    if (!Number.isFinite(amount) || amount < 0) return;
    await run("budget usage recorded", () => invoke("hub_record_budget_usage", { agent, amount }));
    await refreshBudgets();
  };

  const resumeBudget = async (agent: string) => {
    await run("agent resumed", () => invoke("hub_resume_agent", { agent }));
    await refreshBudgets();
  };

  const tabBtn = (id: HubTab, label: string, badge?: number) => (
    <button
      key={id}
      className={hubTab === id ? "btn-primary" : "btn-secondary"}
      style={{ padding: "0.5rem 1rem", fontSize: "0.9rem", borderRadius: "8px", transition: "opacity 0.15s ease, transform 0.15s ease" }}
      onClick={() => startTransition(() => setHubTab(id))}
    >
      {label}
      {!!badge && (
        <span style={{ marginLeft: "0.4rem", fontSize: "0.7rem", padding: "0.05rem 0.4rem", borderRadius: "20px", background: "#eab308", color: "#1a1a1a", fontWeight: 700 }}>
          {badge}
        </span>
      )}
    </button>
  );

  const inboxMessages = messages.filter((message) => {
    const inConversation = inboxConversation === "all"
      || message.from_agent === inboxConversation
      || message.to_agent === inboxConversation;
    const query = inboxSearch.trim().toLowerCase();
    return inConversation && (!query
      || message.body.toLowerCase().includes(query)
      || (message.subject || "").toLowerCase().includes(query));
  });

  const unreadFor = (agent: string) => messages.filter((message) =>
    message.status === "pending" && (agent === "all"
      || message.from_agent === agent
      || message.to_agent === agent)
  ).length;
  const markConversationRead = async () => {
    const target = inboxConversation === "all" ? pollTo : inboxConversation;
    const list = await run(`read ${target}`, () =>
      invoke<MessageRecord[]>("hub_poll_messages", { to: target, markAcked: true })
    );
    if (list) {
      await refreshMessages();
      setStatus(`${list.length} new message${list.length === 1 ? "" : "s"} read`);
    }
  };


  return (
    <div className="glass-card fade-in" style={{ animationDelay: '0.1s' }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", gap: "1rem", flexWrap: "wrap", marginBottom: "1rem" }}>
        <h2 style={{ margin: 0, fontSize: "1.5rem", background: "linear-gradient(to right, #fff, var(--primary))", WebkitBackgroundClip: "text", WebkitTextFillColor: "transparent" }}>
          Shared Hub
        </h2>
        <div style={{ display: "flex", gap: "0.5rem", background: "rgba(0,0,0,0.2)", padding: "0.25rem", borderRadius: "10px" }}>
          {tabBtn("dashboard", "Dashboard")}
          {tabBtn("tasks", "Tasks")}
          {tabBtn("policy", "Policy")}
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
      {hubTab === "policy" && wakePolicy && (
        <div className="fade-in" style={{ display: "flex", flexDirection: "column", gap: "1.5rem" }}>
          <div style={{ ...cardStyle, display: "grid", gap: "1.5rem" }}>
            <h3 style={{ margin: 0, fontSize: "1.2rem", fontWeight: 600, color: "var(--text-main)" }}>Wake Policy Controls</h3>
            <p style={{ margin: 0, fontSize: "0.9rem", color: "var(--text-muted)", lineHeight: 1.5 }}>
              Configure standing policies for agent-to-agent wakeups. This policy applies to all agents operating within the local hub.
            </p>

            <div style={{ display: "flex", flexDirection: "column", gap: "0.75rem", marginTop: "0.5rem" }}>
              <WakePolicyCheckbox
                checked={wakePolicy.default_requires_human_gate}
                onChange={(checked) => updatePolicy({ default_requires_human_gate: checked })}
                title="Require Human Gate by Default"
                description="If enabled, all incoming wake requests must be manually approved by the human owner before the target agent is launched."
              />
              <WakePolicyCheckbox
                checked={wakePolicy.allow_auto_wake}
                onChange={(checked) => updatePolicy({ allow_auto_wake: checked })}
                title="Allow Auto-Wake Requests"
                description="If disabled, any attempt to bypass the human gate (auto-wake) will be outright rejected. Overrides agent-specific delegations."
              />
            </div>
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
            </div>
          </div>
          <UsageChart budgets={budgets} />
          <QuotaChart quotas={quotas} />
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
