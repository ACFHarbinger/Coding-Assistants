import { useCallback, useEffect, useMemo, useState } from "react";
import { isTauriRuntime } from "../../lib/tauri";
import {
  getEffectiveSettings,
  getSettingsLoadStatus,
  getStandingPolicy,
  listAgentBudgets,
  listSettingsAuditEvents,
  resetWorkspaceDefaultProfile,
  resetSettingsField,
  setAgentBudget,
  setAllowAutoWake,
  setConfirmWakes,
  setDefaultSession,
  setDefaultWorkspace,
  setRetentionDays,
  updateOrchestrationPolicy,
  updateSettings,
} from "./api";
import type {
  BudgetStatus,
  EffectiveSettings,
  SandboxStrictness,
  SettingsAuditEvent,
  SettingsField,
  SettingsLoadStatus,
} from "./types";
import GeneralTab from "./tabs/GeneralTab";
import WorkspaceTab from "./tabs/WorkspaceTab";
import MemoryTab from "./tabs/MemoryTab";
import OrchestrationTab from "./tabs/OrchestrationTab";
import AgentsTab from "./tabs/AgentsTab";
import CreativeToolsTab from "./tabs/CreativeToolsTab";
import DiagnosticsTab from "./tabs/DiagnosticsTab";
import DangerTab from "./tabs/DangerTab";
import SettingsAuditDrawer from "./tabs/SettingsAuditDrawer";
import { closeSettingsWindow, readWorkspaceRoot } from "./tabs/shared";

type TabId = "general" | "workspace" | "agents" | "creative" | "orchestration" | "memory" | "diagnostics" | "danger";

interface TabDef {
  id: TabId;
  label: string;
  summary: string;
  dangerous?: boolean;
  implemented: boolean;
}

const TABS: TabDef[] = [
  { id: "general", label: "General", summary: "App-wide defaults, such as which workspace opens on launch.", implemented: true },
  { id: "workspace", label: "Workspace & sessions", summary: "Global default vs. this workspace's default chat session.", implemented: true },
  { id: "agents", label: "Agents & harnesses", summary: "Named provider profiles and per-harness settings.", implemented: true },
  { id: "creative", label: "Creative Tools", summary: "Expose local creative app MCP bridges (Blender, Krita, Godot, etc.) to coding agents.", implemented: true },
  { id: "orchestration", label: "Orchestration", summary: "Task/wake confirmation, auto-enrollment, budgets, tool/sandbox policy.", implemented: true },
  { id: "memory", label: "Memory & storage", summary: "Retention, export, and settings-backup policy.", implemented: true },
  { id: "diagnostics", label: "Diagnostics", summary: "Log level, configuration health, redacted diagnostics export.", implemented: true },
  { id: "danger", label: "Danger zone", summary: "Confirmed reset, removal, and purge operations.", dangerous: true, implemented: true },
];

export default function SettingsApp() {
  const workspaceRoot = useMemo(readWorkspaceRoot, []);
  const [scope, setScope] = useState<"global" | "workspace">(workspaceRoot ? "workspace" : "global");
  const [activeTabId, setActiveTabId] = useState<TabId>("general");
  const [effective, setEffective] = useState<EffectiveSettings | null>(null);
  const [loadStatus, setLoadStatus] = useState<SettingsLoadStatus | null>(null);
  const [auditEvents, setAuditEvents] = useState<SettingsAuditEvent[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const [defaultWorkspaceDraft, setDefaultWorkspaceDraft] = useState("");
  const [defaultSessionDraft, setDefaultSessionDraft] = useState("");
  const [backupRetentionDraft, setBackupRetentionDraft] = useState(3);

  const [confirmWakes, setConfirmWakesLocal] = useState(true);
  const [allowAutoWake, setAllowAutoWakeLocal] = useState(true);
  const [retentionDaysDraft, setRetentionDaysDraft] = useState("");
  const [budgets, setBudgets] = useState<BudgetStatus[]>([]);
  const [budgetAgentIdDraft, setBudgetAgentIdDraft] = useState("");
  const [budgetLimitDraft, setBudgetLimitDraft] = useState("");

  const targetWorkspace = scope === "workspace" ? workspaceRoot : null;
  const activeTab = TABS.find((tab) => tab.id === activeTabId) ?? TABS[0];

  const applySnapshot = useCallback((snapshot: EffectiveSettings) => {
    setEffective(snapshot);
    setDefaultWorkspaceDraft(snapshot.default_workspace ?? "");
    setDefaultSessionDraft(snapshot.default_session ?? "");
    setBackupRetentionDraft(snapshot.backup_retention);
    setRetentionDaysDraft(snapshot.orchestration.retention_days?.toString() ?? "");
  }, []);

  const refresh = useCallback(async () => {
    if (!isTauriRuntime()) return;
    try {
      const [snapshot, status] = await Promise.all([getEffectiveSettings(targetWorkspace), getSettingsLoadStatus()]);
      applySnapshot(snapshot);
      setLoadStatus(status);
      setError(null);
    } catch (err) {
      setError(String(err));
    }
  }, [targetWorkspace, applySnapshot]);

  const refreshAudit = useCallback(async () => {
    if (!isTauriRuntime()) return;
    try {
      setAuditEvents(await listSettingsAuditEvents());
    } catch {
      // Non-critical: the audit panel is a bonus view, not the source of truth.
    }
  }, []);

  // Wake confirmation is global-only (Hub `WakePolicy` has no per-workspace
  // concept today), so it doesn't need to re-fetch on scope changes.
  const refreshStandingPolicy = useCallback(async () => {
    if (!isTauriRuntime()) return;
    try {
      const snapshot = await getStandingPolicy(null);
      setConfirmWakesLocal(snapshot.confirm_wakes);
      setAllowAutoWakeLocal(snapshot.allow_auto_wake);
    } catch (err) {
      setError(String(err));
    }
  }, []);

  const refreshBudgets = useCallback(async () => {
    if (!isTauriRuntime()) return;
    try {
      setBudgets(await listAgentBudgets());
    } catch {
      // Non-critical: budgets are a bonus view on this tab.
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  useEffect(() => {
    void refreshAudit();
  }, [refreshAudit]);

  useEffect(() => {
    void refreshStandingPolicy();
    void refreshBudgets();
  }, [refreshStandingPolicy, refreshBudgets]);

  useEffect(() => {
    document.title = "Settings";
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") void closeSettingsWindow();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  const runMutation = useCallback(
    async (mutate: () => Promise<EffectiveSettings>) => {
      setBusy(true);
      setError(null);
      try {
        applySnapshot(await mutate());
        void refreshAudit();
      } catch (err) {
        setError(String(err));
      } finally {
        setBusy(false);
      }
    },
    [applySnapshot, refreshAudit],
  );

  const moveTabFocus = (event: React.KeyboardEvent<HTMLDivElement>) => {
    const currentIndex = TABS.findIndex((tab) => tab.id === activeTabId);
    let nextIndex: number | null = null;
    if (event.key === "ArrowRight") nextIndex = (currentIndex + 1) % TABS.length;
    else if (event.key === "ArrowLeft") nextIndex = (currentIndex - 1 + TABS.length) % TABS.length;
    else if (event.key === "Home") nextIndex = 0;
    else if (event.key === "End") nextIndex = TABS.length - 1;
    if (nextIndex === null) return;
    event.preventDefault();
    const next = TABS[nextIndex];
    setActiveTabId(next.id);
    document.getElementById(`settings-tab-${next.id}`)?.focus();
  };

  const resetField = (field: SettingsField) => {
    if (!targetWorkspace) return;
    void runMutation(() => resetSettingsField(targetWorkspace, field));
  };

  const toggleConfirmWakes = async () => {
    setBusy(true);
    setError(null);
    try {
      const snapshot = await setConfirmWakes(!confirmWakes);
      setConfirmWakesLocal(snapshot.confirm_wakes);
      void refreshAudit();
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  };

  const toggleAllowAutoWake = async () => {
    setBusy(true);
    setError(null);
    try {
      const snapshot = await setAllowAutoWake(!allowAutoWake);
      setAllowAutoWakeLocal(snapshot.allow_auto_wake);
      void refreshAudit();
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  };

  const toggleOrchestrationField = (field: "confirm_new_enrollment" | "confirm_broadcast" | "auto_enrollment_allowed" | "export_enabled", current: boolean) => {
    void runMutation(() => updateOrchestrationPolicy(targetWorkspace, { [field]: !current }));
  };

  const setSandboxStrictness = (level: SandboxStrictness) => {
    void runMutation(() => updateOrchestrationPolicy(targetWorkspace, { sandbox_strictness: level }));
  };

  const saveRetentionDays = () => {
    const trimmed = retentionDaysDraft.trim();
    const days = trimmed === "" ? null : Number(trimmed);
    if (days !== null && (!Number.isInteger(days) || days <= 0)) {
      setError("Retention days must be empty (indefinite) or a positive whole number.");
      return;
    }
    if (scope === "workspace" && days === null) {
      setError("A workspace override needs a concrete day count — use Reset to Global to clear it.");
      return;
    }
    void runMutation(() => setRetentionDays(targetWorkspace, days));
  };

  const saveBudget = async () => {
    const agentId = budgetAgentIdDraft.trim();
    const limit = Number(budgetLimitDraft);
    if (!agentId || !Number.isFinite(limit) || limit <= 0) {
      setError("Budget needs an agent id and a limit greater than 0.");
      return;
    }
    setBusy(true);
    setError(null);
    try {
      await setAgentBudget(agentId, limit);
      setBudgetAgentIdDraft("");
      setBudgetLimitDraft("");
      await refreshBudgets();
      void refreshAudit();
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  };

  const handleResetWorkspaceOverrides = async () => {
    if (!workspaceRoot) return;
    const fieldsToReset: SettingsField[] = [
      "backup_retention",
      "default_session",
      "confirm_new_enrollment",
      "confirm_broadcast",
      "auto_enrollment_allowed",
      "sandbox_strictness",
      "retention_days",
      "export_enabled",
      "link_suggestion_mode",
    ];
    setBusy(true);
    setError(null);
    try {
      // Read the workspace-scoped snapshot first: profile selection is not a
      // SettingsField, so it has its own reset command.
      const workspaceSettings = await getEffectiveSettings(workspaceRoot);
      for (const field of fieldsToReset) {
        await resetSettingsField(workspaceRoot, field);
      }
      for (const harness of workspaceSettings.harnesses) {
        if (harness.default_profile_status === "override") {
          await resetWorkspaceDefaultProfile(workspaceRoot, harness.harness);
        }
      }
      await refresh();
      void refreshAudit();
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div style={{ height: "100%", display: "flex", flexDirection: "column", padding: "1.5rem", gap: "1.25rem" }}>
      <header style={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: "1rem" }}>
        <div>
          <h1 style={{ fontSize: "1.4rem", fontWeight: 800 }}>Settings</h1>
          <p style={{ color: "var(--text-muted)", fontSize: "0.85rem", marginTop: "0.2rem" }}>
            Global defaults with workspace-specific overrides, persisted under <code>~/.coding-assistants</code>.
          </p>
        </div>
        <button type="button" className="btn-secondary" style={{ marginTop: 0 }} onClick={() => void closeSettingsWindow()}>
          Close
        </button>
      </header>

      {!isTauriRuntime() && (
        <div style={{ padding: "0.75rem 1rem", borderRadius: "10px", border: "1px solid rgba(250, 204, 21, 0.4)", background: "rgba(250, 204, 21, 0.08)", color: "#fde68a", fontSize: "0.85rem" }}>
          Settings requires the desktop app; this browser preview cannot read or save configuration.
        </div>
      )}

      {loadStatus && (loadStatus.status === "invalid" || loadStatus.status === "unreadable") && (
        <div style={{ padding: "0.75rem 1rem", borderRadius: "10px", border: "1px solid rgba(248, 113, 113, 0.45)", background: "rgba(239, 68, 68, 0.10)", color: "#fca5a5", fontSize: "0.85rem" }}>
          <strong>Settings file could not be read — using safe defaults.</strong>{" "}
          {loadStatus.reason} A one-click restore from the last-known-good backup lands in a follow-up slice.
        </div>
      )}

      {error && (
        <div style={{ padding: "0.6rem 1rem", borderRadius: "10px", border: "1px solid rgba(248, 113, 113, 0.45)", background: "rgba(239, 68, 68, 0.10)", color: "#fca5a5", fontSize: "0.82rem" }}>
          {error}
        </div>
      )}

      <div style={{ display: "grid", gridTemplateColumns: "minmax(170px, 0.28fr) minmax(0, 1fr)", gap: "1.25rem", flex: 1, minHeight: 0 }}>
        <div role="tablist" aria-label="Settings sections" onKeyDown={moveTabFocus} style={{ display: "grid", alignContent: "start", gap: "0.35rem" }}>
          {TABS.map((tab) => {
            const selected = tab.id === activeTabId;
            return (
              <button
                key={tab.id}
                id={`settings-tab-${tab.id}`}
                type="button"
                role="tab"
                aria-selected={selected}
                aria-controls={`settings-panel-${tab.id}`}
                tabIndex={selected ? 0 : -1}
                onClick={() => setActiveTabId(tab.id)}
                className="btn-secondary"
                style={{
                  marginTop: 0,
                  padding: "0.55rem 0.7rem",
                  textAlign: "left",
                  borderColor: selected ? "var(--primary)" : undefined,
                  background: selected ? "rgba(99, 102, 241, 0.14)" : undefined,
                  color: tab.dangerous ? "#fca5a5" : undefined,
                }}
              >
                {tab.dangerous ? "⚠ " : ""}
                {tab.label}
              </button>
            );
          })}
        </div>

        <div
          id={`settings-panel-${activeTab.id}`}
          role="tabpanel"
          aria-labelledby={`settings-tab-${activeTab.id}`}
          className="glass-card"
          style={{ overflowY: "auto", minHeight: 0 }}
        >
          <h2 style={{ margin: 0, fontSize: "1.1rem", color: activeTab.dangerous ? "#fca5a5" : undefined }}>
            {activeTab.dangerous ? "⚠ " : ""}
            {activeTab.label}
          </h2>
          <p style={{ color: "var(--text-muted)", fontSize: "0.85rem", margin: "0.3rem 0 1.25rem" }}>{activeTab.summary}</p>

          {!activeTab.implemented && (
            <div style={{ padding: "0.85rem", borderRadius: "9px", background: activeTab.dangerous ? "rgba(239, 68, 68, 0.10)" : "rgba(255,255,255,0.03)", border: activeTab.dangerous ? "1px solid rgba(248, 113, 113, 0.45)" : "1px solid var(--border-color)" }}>
              <strong style={{ display: "block", marginBottom: "0.4rem" }}>
                {activeTab.dangerous ? "Confirmation required" : "Coming in a later Settings slice"}
              </strong>
              <p style={{ margin: 0, color: "var(--text-muted)", fontSize: "0.85rem", lineHeight: 1.55 }}>
                {activeTab.dangerous
                  ? "Reset, removal, and purge controls will always show an explicit warning and confirmation before changing local data."
                  : "This tab's fields land in a follow-up Settings slice (S4/S5)."}
              </p>
            </div>
          )}

          {activeTab.id === "general" && effective && (
            <GeneralTab
              effective={effective}
              defaultWorkspaceDraft={defaultWorkspaceDraft}
              setDefaultWorkspaceDraft={setDefaultWorkspaceDraft}
              workspaceRoot={workspaceRoot}
              busy={busy}
              saveDefaultWorkspace={() => void runMutation(() => setDefaultWorkspace(defaultWorkspaceDraft.trim() || null))}
              clearDefaultWorkspace={() => void runMutation(() => setDefaultWorkspace(null))}
            />
          )}

          {activeTab.id === "workspace" && effective && (
            <WorkspaceTab
              effective={effective}
              scope={scope}
              setScope={setScope}
              workspaceRoot={workspaceRoot}
              defaultSessionDraft={defaultSessionDraft}
              setDefaultSessionDraft={setDefaultSessionDraft}
              busy={busy}
              saveDefaultSession={() => void runMutation(() => setDefaultSession(targetWorkspace, defaultSessionDraft.trim() || null))}
              clearDefaultSession={() => void runMutation(() => setDefaultSession(targetWorkspace, null))}
              resetField={resetField}
            />
          )}

          {activeTab.id === "agents" && effective && (
            <AgentsTab
              effective={effective}
              scope={scope}
              setScope={setScope}
              workspaceRoot={workspaceRoot}
              busy={busy}
              onChanged={() => void refresh()}
            />
          )}

          {activeTab.id === "creative" && (
            <CreativeToolsTab
              workspaceRoot={workspaceRoot}
              busy={busy}
            />
          )}

          {activeTab.id === "memory" && effective && (
            <MemoryTab
              effective={effective}
              scope={scope}
              setScope={setScope}
              workspaceRoot={workspaceRoot}
              backupRetentionDraft={backupRetentionDraft}
              setBackupRetentionDraft={setBackupRetentionDraft}
              busy={busy}
              saveBackupRetention={() => void runMutation(() => updateSettings(targetWorkspace, { backup_retention: backupRetentionDraft }))}
              resetField={resetField}
            />
          )}

          {activeTab.id === "orchestration" && effective && (
            <OrchestrationTab
              effective={effective}
              scope={scope}
              setScope={setScope}
              workspaceRoot={workspaceRoot}
              busy={busy}
              confirmWakes={confirmWakes}
              toggleConfirmWakes={() => void toggleConfirmWakes()}
              allowAutoWake={allowAutoWake}
              toggleAllowAutoWake={() => void toggleAllowAutoWake()}
              toggleOrchestrationField={toggleOrchestrationField}
              setSandboxStrictness={setSandboxStrictness}
              retentionDaysDraft={retentionDaysDraft}
              setRetentionDaysDraft={setRetentionDaysDraft}
              saveRetentionDays={saveRetentionDays}
              resetField={resetField}
              budgets={budgets}
              budgetAgentIdDraft={budgetAgentIdDraft}
              setBudgetAgentIdDraft={setBudgetAgentIdDraft}
              budgetLimitDraft={budgetLimitDraft}
              setBudgetLimitDraft={setBudgetLimitDraft}
              saveBudget={() => void saveBudget()}
            />
          )}

          {activeTab.id === "diagnostics" && effective && (
            <DiagnosticsTab
              effective={effective}
              loadStatus={loadStatus}
              workspaceRoot={workspaceRoot}
              busy={busy}
            />
          )}

          {activeTab.id === "danger" && effective && (
            <DangerTab
              workspaceRoot={workspaceRoot}
              busy={busy}
              onResetWorkspaceOverrides={handleResetWorkspaceOverrides}
              onChanged={() => void refresh()}
            />
          )}
        </div>
      </div>

      <SettingsAuditDrawer auditEvents={auditEvents} />
    </div>
  );
}
