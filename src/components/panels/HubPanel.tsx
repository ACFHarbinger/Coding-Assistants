import { startTransition, useCallback, useEffect, useState } from "react";
import { invoke } from "../../lib/tauri";
import HubPanelView from "./hub/HubPanelView";
import type { AgentRecord, AuditEvent, BudgetStatus, ChannelWorkspace, HubTab, MemoryRecord, MessageRecord, ProviderQuota, ScoredMemoryRecord, WakeRecord } from "./hub/types";

export default function HubPanel() {
  const [hubTab, setHubTab] = useState<HubTab>("dashboard");
  const [dataDir, setDataDir] = useState<string>("");
  const [error, setError] = useState<string>("");
  const [status, setStatus] = useState<string>("");

  const [memories, setMemories] = useState<(MemoryRecord | ScoredMemoryRecord)[]>([]);
  const [messages, setMessages] = useState<MessageRecord[]>([]);
  const [wakes, setWakes] = useState<WakeRecord[]>([]);
  const [agents, setAgents] = useState<AgentRecord[]>([]);
  const [auditEvents, setAuditEvents] = useState<AuditEvent[]>([]);
  const [auditShowAll, setAuditShowAll] = useState(false);

  const [searchQ, setSearchQ] = useState("");
  const [searchMode, setSearchMode] = useState<"smart" | "exact">("smart");
  const [tierFilter, setTierFilter] = useState<string>("");
  const [scopeFilter, setScopeFilter] = useState<string>("");
  const [memTitle, setMemTitle] = useState("");
  const [memBody, setMemBody] = useState("");
  const [memTier, setMemTier] = useState("short_term");
  const [memScope, setMemScope] = useState("workspace");
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

  const [budgets, setBudgets] = useState<BudgetStatus[]>([]);
  const [quotas, setQuotas] = useState<ProviderQuota[]>([]);
  const [refreshingQuotaIds, setRefreshingQuotaIds] = useState<Set<string>>(new Set());
  const [budgetAgent, setBudgetAgent] = useState("");
  const [budgetLimit, setBudgetLimit] = useState("100");
  const [budgetSpend, setBudgetSpend] = useState("1");

  const [editingMemory, setEditingMemory] = useState<string | null>(null);
  const [editTitle, setEditTitle] = useState("");
  const [editBody, setEditBody] = useState("");

  const [channelWorkspaces, setChannelWorkspaces] = useState<ChannelWorkspace[]>([]);
  const [channelRenameDrafts, setChannelRenameDrafts] = useState<Record<string, string>>({});
  const [channelConnected, setChannelConnected] = useState<Record<string, boolean>>({});
  const [channelConnecting, setChannelConnecting] = useState<Record<string, boolean>>({});

  const run = useCallback(async <T,>(label: string, fn: () => Promise<T>, pendingMessage?: string): Promise<T | null> => {
    setError("");
    if (pendingMessage) {
      setStatus(pendingMessage);
    }
    try {
      const result = await fn();
      setStatus(label);
      return result;
    } catch (e) {
      setError(String(e));
      setStatus("");
      return null;
    }
  }, []);

  const refreshMemories = useCallback(async () => {
    const list = await run("memories refreshed", () =>
      invoke<MemoryRecord[]>("hub_list_memories", {
        scope: scopeFilter || null,
        tier: tierFilter || null,
        workspace: null,
        includeStale: false,
      })
    );
    if (list) setMemories(list);
  }, [run, tierFilter, scopeFilter]);

  const refreshMessages = useCallback(async () => {
    const list = await run("inbox refreshed", () => invoke<MessageRecord[]>("hub_list_messages"));
    if (list) setMessages(list);
  }, [run]);

  const refreshWakes = useCallback(async () => {
    const list = await run("wakes refreshed", () => invoke<WakeRecord[]>("hub_list_wakes"));
    if (list) setWakes(list);
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
    const list = await run("quotas refreshed", () => invoke<ProviderQuota[]>("hub_get_provider_quotas", { forceRefresh: false }));
    if (list) setQuotas(list);
  }, [run]);

  const refreshStaleQuotas = useCallback(async () => {
    const list = await run("stale quotas refreshed", () => invoke<ProviderQuota[]>("hub_get_provider_quotas", { forceRefresh: true }), "Probing provider quotas…");
    if (list) setQuotas(list);
  }, [run]);

  const refreshSingleQuota = useCallback(async (agentId: string) => {
    setRefreshingQuotaIds((prev) => new Set(prev).add(agentId));
    try {
      const updated = await invoke<ProviderQuota>("hub_refresh_provider_quota", { agentId });
      setQuotas((prev) => prev.map((q) => (q.agent_id === agentId ? updated : q)));
    } catch (e) {
      setError(String(e));
    } finally {
      setRefreshingQuotaIds((prev) => {
        const next = new Set(prev);
        next.delete(agentId);
        return next;
      });
    }
  }, []);

  const refreshAuditEvents = useCallback(async () => {
    const list = await run("audit events refreshed", () =>
      invoke<AuditEvent[]>("hub_list_audit_events", { pendingOnly: !auditShowAll })
    );
    if (list) setAuditEvents(list);
  }, [run, auditShowAll]);

  const approveAudit = async (eventId: string) => {
    await run("event approved", () =>
      invoke("hub_approve_audit_event", { eventId, approvedBy: "human" })
    );
    await refreshAuditEvents();
  };

  const quarantineAudit = async (eventId: string) => {
    await run("event quarantined", () =>
      invoke("hub_quarantine_audit_event", { eventId, quarantinedBy: "human", reason: "Rejected via desktop journal" })
    );
    await refreshAuditEvents();
  };

  const refreshChannelWorkspaces = useCallback(async () => {
    const list = await run("channels refreshed", () =>
      invoke<ChannelWorkspace[]>("hub_list_channel_workspaces")
    );
    if (list) {
      setChannelWorkspaces(list);
      for (const cw of list) {
        invoke<boolean>("hub_is_channel_workspace_connected", { workspace: cw.workspace })
          .then((connected) => {
            setChannelConnected((prev) => ({ ...prev, [cw.workspace]: connected }));
          })
          .catch(() => {});
      }
    }
  }, [run]);

  const connectChannelWorkspace = useCallback(async (workspace: string) => {
    setChannelConnecting((prev) => ({ ...prev, [workspace]: true }));
    try {
      const outcome = await run(
        "channel connection requested",
        () => invoke<{ ok: boolean; pid?: number; detail: string }>("hub_connect_channel_workspace", { workspace }),
        "Starting Claude Code channel…",
      );
      if (outcome?.ok) {
        setChannelConnected((prev) => ({ ...prev, [workspace]: true }));
        setStatus(`Claude Channel connected (pid ${outcome.pid ?? "unknown"})`);
      } else if (outcome) {
        setError(outcome.detail);
      }
    } finally {
      setChannelConnecting((prev) => ({ ...prev, [workspace]: false }));
    }
  }, [run]);

  const renameChannelWorkspace = async (workspace: string) => {
    const draft = channelRenameDrafts[workspace]?.trim();
    if (!draft) return;
    await run("channel renamed", () =>
      invoke("hub_rename_channel_workspace", { workspace, displayName: draft })
    );
    await refreshChannelWorkspaces();
  };

  const deleteChannelWorkspace = async (workspace: string) => {
    await run("channel removed", () =>
      invoke("hub_delete_channel_workspace", { workspace })
    );
    await refreshChannelWorkspaces();
  };

  useEffect(() => {
    invoke<string>("get_hub_data_dir").then(setDataDir).catch((e) => setError(String(e)));
    invoke<AgentRecord[]>("hub_list_agents").then((list) => {
      setAgents(list);
      const firstAgent = list.find((a) => a.id !== "human");
      if (firstAgent) {
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
    else if (hubTab === "usage") {
      refreshBudgets();
      refreshQuotas();
    }
    else if (hubTab === "journal") refreshAuditEvents();
    else if (hubTab === "channels") refreshChannelWorkspaces();
  }, [hubTab, refreshMemories, refreshMessages, refreshWakes, refreshBudgets, refreshQuotas, refreshAuditEvents, refreshChannelWorkspaces]);

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
        args: {
          tier: memTier,
          scope: memScope,
          agent: memAgent || null,
          workspace: null,
          title: memTitle || null,
          body: memBody,
          tags: [],
        },
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
        args: {
          id,
          title: editTitle || null,
          body: editBody,
          tags: null,
        },
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
    if (searchMode === "smart") {
      const list = await run("smart search done", () =>
        invoke<ScoredMemoryRecord[]>("hub_search_memories_hybrid", {
          query: searchQ,
          limit: 30,
          scope: scopeFilter || null,
          tier: tierFilter || null,
          workspace: null,
        }),
        "Searching memories (similarity)…"
      );
      if (list) setMemories(list);
    } else {
      const list = await run("exact search done", () =>
        invoke<MemoryRecord[]>("hub_search_memories", { query: searchQ })
      );
      if (list) {
        let filtered = list;
        if (tierFilter) filtered = filtered.filter((m) => m.tier === tierFilter);
        if (scopeFilter) filtered = filtered.filter((m) => m.scope === scopeFilter);
        setMemories(filtered);
      }
    }
  };

  const reindexVectors = async () => {
    const count = await run(
      "vectors reindexed",
      () => invoke<number>("hub_reindex_memory_vectors"),
      "Re-indexing memory vector embeddings…"
    );
    if (count !== null) {
      setStatus(`Re-indexed ${count} memory vector embedding(s)`);
    }
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

  const grokWorkspace = typeof localStorage !== "undefined" ? (localStorage.getItem("ca.workspaceRoot") || "") : "";

  return <HubPanelView {...{ hubTab, dataDir, error, status, setStatus, tabBtn, auditEvents, setAuditShowAll, auditShowAll, refreshAuditEvents, approveAudit, quarantineAudit, memories, searchQ, setSearchQ, searchMode, setSearchMode, searchMemories, refreshMemories, tierFilter, setTierFilter, scopeFilter, setScopeFilter, memTier, setMemTier, memScope, setMemScope, memAgent, setMemAgent, memTitle, setMemTitle, memBody, setMemBody, writeMemory, editingMemory, setEditingMemory, editTitle, setEditTitle, editBody, setEditBody, saveEditedMemory, run, invoke, agents, inboxConversation, setInboxConversation, setMsgTo, setPollTo, unreadFor, msgFrom, setMsgFrom, msgTo, msgKind, setMsgKind, msgSubject, setMsgSubject, msgBody, setMsgBody, sendMessage, pollTo, markConversationRead, refreshMessages, inboxSearch, setInboxSearch, inboxMessages, wakeTarget, setWakeTarget, wakeReason, setWakeReason, requestWake, refreshWakes, wakes, budgetAgent, setBudgetAgent, budgetLimit, setBudgetLimit, setBudget, refreshBudgets, refreshQuotas, refreshStaleQuotas, budgets, quotas, refreshingQuotaIds, refreshSingleQuota, budgetSpend, setBudgetSpend, recordSpend, resumeBudget, channelWorkspaces, channelRenameDrafts, setChannelRenameDrafts, renameChannelWorkspace, deleteChannelWorkspace, refreshChannelWorkspaces, channelConnected, channelConnecting, connectChannelWorkspace, grokWorkspace, reindexVectors }} />;
}
