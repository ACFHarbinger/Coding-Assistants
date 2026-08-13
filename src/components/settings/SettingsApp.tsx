import { useCallback, useEffect, useMemo, useState } from "react";
import { isTauriRuntime } from "../../lib/tauri";
import {
  getEffectiveSettings,
  getSettingsLoadStatus,
  listSettingsAuditEvents,
  resetSettingsField,
  setDefaultSession,
  setDefaultWorkspace,
  updateSettings,
} from "./api";
import type { EffectiveSettings, SettingsAuditEvent, SettingsField, SettingsFieldStatus, SettingsLoadStatus } from "./types";

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
  { id: "orchestration", label: "Orchestration", summary: "Task/wake confirmation, auto-enrollment, budgets, tool/sandbox policy.", implemented: false },
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

  const targetWorkspace = scope === "workspace" ? workspaceRoot : null;
  const activeTab = TABS.find((tab) => tab.id === activeTabId) ?? TABS[0];

  const applySnapshot = useCallback((snapshot: EffectiveSettings) => {
    setEffective(snapshot);
    setDefaultWorkspaceDraft(snapshot.default_workspace ?? "");
    setDefaultSessionDraft(snapshot.default_session ?? "");
    setBackupRetentionDraft(snapshot.backup_retention);
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

  useEffect(() => {
    void refresh();
  }, [refresh]);

  useEffect(() => {
    void refreshAudit();
  }, [refreshAudit]);

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
