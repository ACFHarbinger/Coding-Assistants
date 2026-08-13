import { startTransition, useCallback, useEffect, useState } from "react";
import { invoke } from "../../lib/tauri";
import { LIVE_QUOTA_AGENT_IDS } from "./hub/HubCharts";
import HubPanelView from "./hub/HubPanelView";
import type { AgentRecord, AuditEvent, BudgetStatus, HubTab, MemoryRecord, MessageRecord, ProviderQuota, WakePolicy, WakeRecord } from "./hub/types";

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
  const [refreshingQuotaIds, setRefreshingQuotaIds] = useState<Set<string>>(new Set());
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

  const refreshSingleQuota = useCallback(async (agentId: string) => {
    setRefreshingQuotaIds((prev) => new Set(prev).add(agentId));
    try {
      const updated = await run(`${agentId} quota refreshed`, () =>
        invoke<ProviderQuota>("hub_refresh_provider_quota", { agentId })
      );
      if (updated) {
        setQuotas((prev) => prev.map((q) => (q.agent_id === agentId ? updated : q)));
      }
    } finally {
      setRefreshingQuotaIds((prev) => {
        const next = new Set(prev);
        next.delete(agentId);
        return next;
      });
    }
  }, [run]);

  const refreshStaleQuotas = useCallback(async () => {
    const staleIds = quotas
      .map((q) => q.agent_id)
      .filter((id) => !LIVE_QUOTA_AGENT_IDS.has(id));
    await Promise.all(staleIds.map((id) => refreshSingleQuota(id)));
  }, [quotas, refreshSingleQuota]);

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

  return <HubPanelView {...{ hubTab, dataDir, error, status, setStatus, tabBtn, auditEvents, setAuditShowAll, auditShowAll, refreshAuditEvents, approveAudit, quarantineAudit, memories, searchQ, setSearchQ, searchMemories, refreshMemories, tierFilter, setTierFilter, memTier, setMemTier, memAgent, setMemAgent, memTitle, setMemTitle, memBody, setMemBody, writeMemory, editingMemory, setEditingMemory, editTitle, setEditTitle, editBody, setEditBody, saveEditedMemory, run, invoke, agents, inboxConversation, setInboxConversation, setMsgTo, setPollTo, unreadFor, msgFrom, setMsgFrom, msgTo, msgKind, setMsgKind, msgSubject, setMsgSubject, msgBody, setMsgBody, sendMessage, pollTo, markConversationRead, refreshMessages, inboxSearch, setInboxSearch, inboxMessages, wakeTarget, setWakeTarget, wakeReason, setWakeReason, requestWake, refreshWakes, wakes, wakePolicy, updatePolicy, budgetAgent, setBudgetAgent, budgetLimit, setBudgetLimit, setBudget, refreshBudgets, refreshQuotas, refreshStaleQuotas, budgets, quotas, refreshingQuotaIds, refreshSingleQuota, budgetSpend, setBudgetSpend, recordSpend, resumeBudget }} />;
}
