import { useCallback, useEffect, useMemo, useState } from "react";
import { isTauriRuntime } from "../../lib/tauri";
import {
  getEffectiveSettings,
  getSettingsLoadStatus,
  getStandingPolicy,
  listAgentBudgets,
  listSettingsAuditEvents,
  resetSettingsField,
  setAgentBudget,
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
  SettingsFieldStatus,
  SettingsLoadStatus,
} from "./types";

type TabId = "general" | "workspace" | "agents" | "orchestration" | "memory" | "diagnostics" | "danger";

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
  { id: "agents", label: "Agents & harnesses", summary: "Named provider profiles and per-harness settings.", implemented: false },
  { id: "orchestration", label: "Orchestration", summary: "Task/wake confirmation, auto-enrollment, budgets, tool/sandbox policy.", implemented: true },
  { id: "memory", label: "Memory & storage", summary: "Retention, export, and settings-backup policy.", implemented: true },
  { id: "diagnostics", label: "Diagnostics", summary: "Log level, configuration health, redacted diagnostics export.", implemented: false },
  { id: "danger", label: "Danger zone", summary: "Confirmed reset, removal, and purge operations.", dangerous: true, implemented: false },
];

const MIN_BACKUP_RETENTION = 1;
const MAX_BACKUP_RETENTION = 20;

function readWorkspaceRoot(): string | null {
  try {
    return localStorage.getItem("ca.workspaceRoot");
  } catch {
    return null;
  }
}

function shortenPath(path: string, max = 42): string {
  if (path.length <= max) return path;
  return `…${path.slice(path.length - max + 1)}`;
}

async function closeSettingsWindow(): Promise<void> {
  if (!isTauriRuntime()) {
    window.close();
    return;
  }
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  await getCurrentWindow().close();
}

function StatusPill({ status }: { status: SettingsFieldStatus }) {
  const isOverride = status === "override";
  return (
    <span
      style={{
        fontSize: "0.7rem",
        fontWeight: 600,
        padding: "0.15rem 0.55rem",
        borderRadius: "999px",
        border: `1px solid ${isOverride ? "rgba(129, 140, 248, 0.5)" : "var(--border-color)"}`,
        color: isOverride ? "#a5b4fc" : "var(--text-muted)",
        background: isOverride ? "rgba(99, 102, 241, 0.15)" : "rgba(255,255,255,0.03)",
        whiteSpace: "nowrap",
      }}
    >
      {isOverride ? "Workspace Override" : "Inherited"}
    </span>
  );
}

function FieldRow({ label, hint, pill, children }: { label: string; hint?: string; pill?: React.ReactNode; children: React.ReactNode }) {
  return (
    <div style={{ marginBottom: "1.25rem" }}>
      <div style={{ display: "flex", alignItems: "center", gap: "0.6rem", marginBottom: "0.35rem" }}>
        <label style={{ fontWeight: 600, fontSize: "0.9rem" }}>{label}</label>
        {pill}
      </div>
      {hint && <p style={{ margin: "0 0 0.5rem", color: "var(--text-muted)", fontSize: "0.8rem", lineHeight: 1.5 }}>{hint}</p>}
      <div style={{ display: "flex", flexWrap: "wrap", gap: "0.5rem", alignItems: "center" }}>{children}</div>
    </div>
  );
}

function ToggleRow({
  label,
  hint,
  checked,
  onToggle,
  disabled,
  pill,
  resetButton,
}: {
  label: string;
  hint?: string;
  checked: boolean;
  onToggle: () => void;
  disabled?: boolean;
  pill?: React.ReactNode;
  resetButton?: React.ReactNode;
}) {
  return (
    <div style={{ marginBottom: "1.1rem" }}>
      <div style={{ display: "flex", alignItems: "center", gap: "0.6rem", flexWrap: "wrap" }}>
        <button
          type="button"
          role="switch"
          aria-checked={checked}
          onClick={onToggle}
          disabled={disabled}
          className={checked ? "btn-primary" : "btn-secondary"}
          style={{ marginTop: 0, padding: "0.3rem 0.7rem", fontSize: "0.78rem", minWidth: "3.2rem" }}
        >
          {checked ? "On" : "Off"}
        </button>
        <label style={{ fontWeight: 600, fontSize: "0.9rem" }}>{label}</label>
        {pill}
        {resetButton}
      </div>
      {hint && <p style={{ margin: "0.3rem 0 0", color: "var(--text-muted)", fontSize: "0.8rem", lineHeight: 1.5 }}>{hint}</p>}
    </div>
  );
}

const SANDBOX_LEVELS: SandboxStrictness[] = ["strict", "standard", "permissive"];

const inputStyle: React.CSSProperties = {
  flex: "1 1 260px",
  minWidth: "200px",
  padding: "0.5rem 0.7rem",
  borderRadius: "8px",
  border: "1px solid var(--border-color)",
  background: "rgba(255,255,255,0.03)",
  color: "var(--text-main)",
  fontSize: "0.85rem",
};

export default function SettingsApp() {
  const workspaceRoot = useMemo(readWorkspaceRoot, []);
  const [scope, setScope] = useState<"global" | "workspace">(workspaceRoot ? "workspace" : "global");
  const [activeTabId, setActiveTabId] = useState<TabId>("general");
  const [effective, setEffective] = useState<EffectiveSettings | null>(null);
  const [loadStatus, setLoadStatus] = useState<SettingsLoadStatus | null>(null);
  const [auditEvents, setAuditEvents] = useState<SettingsAuditEvent[]>([]);
  const [showAudit, setShowAudit] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const [defaultWorkspaceDraft, setDefaultWorkspaceDraft] = useState("");
  const [defaultSessionDraft, setDefaultSessionDraft] = useState("");
  const [backupRetentionDraft, setBackupRetentionDraft] = useState(3);

  const [confirmWakes, setConfirmWakesLocal] = useState(true);
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
            <FieldRow
              label="Default workspace"
              hint="Which workspace opens on launch when Orchestrate isn't restoring the last session. Global only — there is no per-workspace override of this."
            >
              <input
                style={inputStyle}
                value={defaultWorkspaceDraft}
                onChange={(event) => setDefaultWorkspaceDraft(event.target.value)}
                placeholder="/path/to/workspace"
                disabled={busy}
              />
              <button type="button" className="btn-secondary" style={{ marginTop: 0 }} disabled={busy || !workspaceRoot} onClick={() => setDefaultWorkspaceDraft(workspaceRoot ?? "")}>
                Use current workspace
              </button>
              <button
                type="button"
                className="btn-primary"
                style={{ marginTop: 0 }}
                disabled={busy}
                onClick={() => void runMutation(() => setDefaultWorkspace(defaultWorkspaceDraft.trim() || null))}
              >
                Save
              </button>
              <button
                type="button"
                className="btn-secondary"
                style={{ marginTop: 0 }}
                disabled={busy || !effective.default_workspace}
                onClick={() => void runMutation(() => setDefaultWorkspace(null))}
              >
                Clear
              </button>
            </FieldRow>
          )}

          {activeTab.id === "workspace" && effective && (
            <>
              <div style={{ display: "flex", gap: "0.5rem", marginBottom: "1.5rem" }}>
                <button type="button" className={scope === "global" ? "btn-primary" : "btn-secondary"} style={{ marginTop: 0 }} onClick={() => setScope("global")}>
                  Global defaults
                </button>
                <button
                  type="button"
                  className={scope === "workspace" ? "btn-primary" : "btn-secondary"}
                  style={{ marginTop: 0 }}
                  disabled={!workspaceRoot}
                  title={workspaceRoot ?? "No workspace root is set in the main window"}
                  onClick={() => setScope("workspace")}
                >
                  This workspace{workspaceRoot ? ` (${shortenPath(workspaceRoot)})` : ""}
                </button>
              </div>

              <FieldRow
                label="Default chat / session"
                hint="Which Chat & Memory session this scope opens by default."
                pill={scope === "workspace" ? <StatusPill status={effective.default_session_status} /> : undefined}
              >
                <input
                  style={inputStyle}
                  value={defaultSessionDraft}
                  onChange={(event) => setDefaultSessionDraft(event.target.value)}
                  placeholder="session id"
                  disabled={busy}
                />
                <button
                  type="button"
                  className="btn-primary"
                  style={{ marginTop: 0 }}
                  disabled={busy}
                  onClick={() => void runMutation(() => setDefaultSession(targetWorkspace, defaultSessionDraft.trim() || null))}
                >
                  Save
                </button>
                <button
                  type="button"
                  className="btn-secondary"
                  style={{ marginTop: 0 }}
                  disabled={busy || !effective.default_session}
                  onClick={() => void runMutation(() => setDefaultSession(targetWorkspace, null))}
                >
                  Clear
                </button>
                {scope === "workspace" && effective.default_session_status === "override" && (
                  <button type="button" className="btn-secondary" style={{ marginTop: 0 }} disabled={busy} onClick={() => resetField("default_session")}>
                    Reset to Global
                  </button>
                )}
              </FieldRow>
            </>
          )}

          {activeTab.id === "memory" && effective && (
            <>
              <div style={{ display: "flex", gap: "0.5rem", marginBottom: "1.5rem" }}>
                <button type="button" className={scope === "global" ? "btn-primary" : "btn-secondary"} style={{ marginTop: 0 }} onClick={() => setScope("global")}>
                  Global defaults
                </button>
                <button
                  type="button"
                  className={scope === "workspace" ? "btn-primary" : "btn-secondary"}
                  style={{ marginTop: 0 }}
                  disabled={!workspaceRoot}
                  title={workspaceRoot ?? "No workspace root is set in the main window"}
                  onClick={() => setScope("workspace")}
                >
                  This workspace{workspaceRoot ? ` (${shortenPath(workspaceRoot)})` : ""}
                </button>
              </div>

              <FieldRow
                label={`Retained settings backups (${MIN_BACKUP_RETENTION}–${MAX_BACKUP_RETENTION})`}
                hint="How many timestamped last-known-good copies of settings.toml to keep."
                pill={scope === "workspace" ? <StatusPill status={effective.backup_retention_status} /> : undefined}
              >
                <input
                  type="number"
                  min={MIN_BACKUP_RETENTION}
                  max={MAX_BACKUP_RETENTION}
                  style={{ ...inputStyle, flex: "0 0 100px" }}
                  value={backupRetentionDraft}
                  onChange={(event) => setBackupRetentionDraft(Number(event.target.value))}
                  disabled={busy}
                />
                <button
                  type="button"
                  className="btn-primary"
                  style={{ marginTop: 0 }}
                  disabled={busy}
                  onClick={() => void runMutation(() => updateSettings(targetWorkspace, { backup_retention: backupRetentionDraft }))}
                >
                  Save
                </button>
                {scope === "workspace" && effective.backup_retention_status === "override" && (
                  <button type="button" className="btn-secondary" style={{ marginTop: 0 }} disabled={busy} onClick={() => resetField("backup_retention")}>
                    Reset to Global
                  </button>
                )}
              </FieldRow>
            </>
          )}

          {activeTab.id === "orchestration" && effective && (
            <>
              <ToggleRow
                label="Confirm before wakes"
                hint="Standing wake human-gate. Global only — Hub's WakePolicy has no per-workspace scope today."
                checked={confirmWakes}
                onToggle={() => void toggleConfirmWakes()}
                disabled={busy}
              />

              <div style={{ display: "flex", gap: "0.5rem", margin: "1.4rem 0 1.1rem" }}>
                <button type="button" className={scope === "global" ? "btn-primary" : "btn-secondary"} style={{ marginTop: 0 }} onClick={() => setScope("global")}>
                  Global defaults
                </button>
                <button
                  type="button"
                  className={scope === "workspace" ? "btn-primary" : "btn-secondary"}
                  style={{ marginTop: 0 }}
                  disabled={!workspaceRoot}
                  title={workspaceRoot ?? "No workspace root is set in the main window"}
                  onClick={() => setScope("workspace")}
                >
                  This workspace{workspaceRoot ? ` (${shortenPath(workspaceRoot)})` : ""}
                </button>
              </div>

              <ToggleRow
                label="Confirm new enrollment"
                hint="Confirm before a wake enrolls a not-yet-team-member identity."
                checked={effective.orchestration.confirm_new_enrollment}
                onToggle={() => toggleOrchestrationField("confirm_new_enrollment", effective.orchestration.confirm_new_enrollment)}
                disabled={busy}
                pill={scope === "workspace" ? <StatusPill status={effective.orchestration.confirm_new_enrollment_status} /> : undefined}
                resetButton={
                  scope === "workspace" && effective.orchestration.confirm_new_enrollment_status === "override" ? (
                    <button type="button" className="btn-secondary" style={{ marginTop: 0, padding: "0.25rem 0.6rem", fontSize: "0.72rem" }} disabled={busy} onClick={() => resetField("confirm_new_enrollment")}>
                      Reset to Global
                    </button>
                  ) : undefined
                }
              />

              <ToggleRow
                label="Confirm broadcasts"
                hint="Confirm before an all/team broadcast send."
                checked={effective.orchestration.confirm_broadcast}
                onToggle={() => toggleOrchestrationField("confirm_broadcast", effective.orchestration.confirm_broadcast)}
                disabled={busy}
                pill={scope === "workspace" ? <StatusPill status={effective.orchestration.confirm_broadcast_status} /> : undefined}
                resetButton={
                  scope === "workspace" && effective.orchestration.confirm_broadcast_status === "override" ? (
                    <button type="button" className="btn-secondary" style={{ marginTop: 0, padding: "0.25rem 0.6rem", fontSize: "0.72rem" }} disabled={busy} onClick={() => resetField("confirm_broadcast")}>
                      Reset to Global
                    </button>
                  ) : undefined
                }
              />

              <ToggleRow
                label="Allow auto-enrollment"
                hint="Whether a wake may enroll a brand-new harness identity at all. When off, a wake to an unknown identity is refused rather than silently enrolling it."
                checked={effective.orchestration.auto_enrollment_allowed}
                onToggle={() => toggleOrchestrationField("auto_enrollment_allowed", effective.orchestration.auto_enrollment_allowed)}
                disabled={busy}
                pill={scope === "workspace" ? <StatusPill status={effective.orchestration.auto_enrollment_allowed_status} /> : undefined}
                resetButton={
                  scope === "workspace" && effective.orchestration.auto_enrollment_allowed_status === "override" ? (
                    <button type="button" className="btn-secondary" style={{ marginTop: 0, padding: "0.25rem 0.6rem", fontSize: "0.72rem" }} disabled={busy} onClick={() => resetField("auto_enrollment_allowed")}>
                      Reset to Global
                    </button>
                  ) : undefined
                }
              />

              <ToggleRow
                label="Allow non-destructive export"
                hint="Whether Markdown export actions are available."
                checked={effective.orchestration.export_enabled}
                onToggle={() => toggleOrchestrationField("export_enabled", effective.orchestration.export_enabled)}
                disabled={busy}
                pill={scope === "workspace" ? <StatusPill status={effective.orchestration.export_enabled_status} /> : undefined}
                resetButton={
                  scope === "workspace" && effective.orchestration.export_enabled_status === "override" ? (
                    <button type="button" className="btn-secondary" style={{ marginTop: 0, padding: "0.25rem 0.6rem", fontSize: "0.72rem" }} disabled={busy} onClick={() => resetField("export_enabled")}>
                      Reset to Global
                    </button>
                  ) : undefined
                }
              />

              <FieldRow
                label="Sandbox strictness"
                hint="Strict refuses to start or inject a harness that can't run without bypassing approval (currently: vibe)."
                pill={scope === "workspace" ? <StatusPill status={effective.orchestration.sandbox_strictness_status} /> : undefined}
              >
                {SANDBOX_LEVELS.map((level) => (
                  <button
                    key={level}
                    type="button"
                    className={effective.orchestration.sandbox_strictness === level ? "btn-primary" : "btn-secondary"}
                    style={{ marginTop: 0, textTransform: "capitalize" }}
                    disabled={busy}
                    onClick={() => setSandboxStrictness(level)}
                  >
                    {level}
                  </button>
                ))}
                {scope === "workspace" && effective.orchestration.sandbox_strictness_status === "override" && (
                  <button type="button" className="btn-secondary" style={{ marginTop: 0 }} disabled={busy} onClick={() => resetField("sandbox_strictness")}>
                    Reset to Global
                  </button>
                )}
              </FieldRow>

              <FieldRow
                label="Transcript/memory retention (days)"
                hint="Empty means indefinite. A workspace override always names a concrete day count — use Reset to Global to clear it."
                pill={scope === "workspace" ? <StatusPill status={effective.orchestration.retention_days_status} /> : undefined}
              >
                <input
                  type="number"
                  min={1}
                  placeholder="indefinite"
                  style={{ ...inputStyle, flex: "0 0 140px" }}
                  value={retentionDaysDraft}
                  onChange={(event) => setRetentionDaysDraft(event.target.value)}
                  disabled={busy}
                />
                <button type="button" className="btn-primary" style={{ marginTop: 0 }} disabled={busy} onClick={saveRetentionDays}>
                  Save
                </button>
                {scope === "workspace" && effective.orchestration.retention_days_status === "override" && (
                  <button type="button" className="btn-secondary" style={{ marginTop: 0 }} disabled={busy} onClick={() => resetField("retention_days")}>
                    Reset to Global
                  </button>
                )}
              </FieldRow>

              <div style={{ marginTop: "1.5rem", paddingTop: "1.1rem", borderTop: "1px solid var(--border-color)" }}>
                <label style={{ fontWeight: 600, fontSize: "0.9rem", display: "block", marginBottom: "0.5rem" }}>Per-agent budgets</label>
                <p style={{ margin: "0 0 0.6rem", color: "var(--text-muted)", fontSize: "0.8rem" }}>
                  Global only. Stored in the same Hub budget table every C6 flow already reads.
                </p>
                {budgets.length === 0 && <p style={{ color: "var(--text-muted)", fontSize: "0.8rem" }}>No agent budgets configured yet.</p>}
                {budgets.map((budget) => (
                  <div key={budget.agent_id} style={{ display: "flex", gap: "0.6rem", alignItems: "center", fontSize: "0.82rem", padding: "0.3rem 0" }}>
                    <strong style={{ minWidth: "90px" }}>{budget.agent_id}</strong>
                    <span style={{ color: "var(--text-muted)" }}>
                      {budget.spent_units} / {budget.limit_units} units
                    </span>
                    {budget.paused && (
                      <span style={{ color: "#fca5a5", fontSize: "0.72rem", fontWeight: 600 }}>Paused</span>
                    )}
                  </div>
                ))}
                <div style={{ display: "flex", gap: "0.5rem", marginTop: "0.6rem", flexWrap: "wrap" }}>
                  <input
                    style={{ ...inputStyle, flex: "0 0 140px" }}
                    placeholder="agent id"
                    value={budgetAgentIdDraft}
                    onChange={(event) => setBudgetAgentIdDraft(event.target.value)}
                    disabled={busy}
                  />
                  <input
                    type="number"
                    min={0}
                    style={{ ...inputStyle, flex: "0 0 120px" }}
                    placeholder="limit units"
                    value={budgetLimitDraft}
                    onChange={(event) => setBudgetLimitDraft(event.target.value)}
                    disabled={busy}
                  />
                  <button type="button" className="btn-primary" style={{ marginTop: 0 }} disabled={busy} onClick={() => void saveBudget()}>
                    Set budget
                  </button>
                </div>
              </div>
            </>
          )}
        </div>
      </div>

      <div>
        <button type="button" className="btn-secondary" style={{ marginTop: 0, padding: "0.4rem 0.8rem", fontSize: "0.78rem" }} onClick={() => setShowAudit((value) => !value)}>
          {showAudit ? "Hide" : "Show"} recent settings changes ({auditEvents.length})
        </button>
        {showAudit && (
          <div style={{ marginTop: "0.6rem", maxHeight: "160px", overflowY: "auto", border: "1px solid var(--border-color)", borderRadius: "8px", padding: "0.5rem 0.75rem" }}>
            {auditEvents.length === 0 && <p style={{ color: "var(--text-muted)", fontSize: "0.8rem" }}>No settings changes recorded yet.</p>}
            {auditEvents
              .slice()
              .reverse()
              .map((event) => (
                <div key={event.id} style={{ fontSize: "0.78rem", color: "var(--text-muted)", padding: "0.25rem 0", borderBottom: "1px solid var(--border-color)" }}>
                  <span style={{ color: "var(--text-main)" }}>{event.operation}</span> {event.path} — {event.observed_at}
                </div>
              ))}
          </div>
        )}
      </div>
    </div>
  );
}
