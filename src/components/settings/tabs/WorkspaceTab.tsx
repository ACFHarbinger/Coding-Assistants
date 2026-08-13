import type { EffectiveSettings, SettingsField } from "../types";
import { FieldRow, StatusPill, inputStyle, shortenPath } from "./shared";

export interface WorkspaceTabProps {
  effective: EffectiveSettings;
  scope: "global" | "workspace";
  setScope: (scope: "global" | "workspace") => void;
  workspaceRoot: string | null;
  defaultSessionDraft: string;
  setDefaultSessionDraft: (value: string) => void;
  busy: boolean;
  saveDefaultSession: () => void;
  clearDefaultSession: () => void;
  resetField: (field: SettingsField) => void;
}

export default function WorkspaceTab({
  effective,
  scope,
  setScope,
  workspaceRoot,
  defaultSessionDraft,
  setDefaultSessionDraft,
  busy,
  saveDefaultSession,
  clearDefaultSession,
  resetField,
}: WorkspaceTabProps) {
  return (
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
        <button type="button" className="btn-primary" style={{ marginTop: 0 }} disabled={busy} onClick={saveDefaultSession}>
          Save
        </button>
        <button type="button" className="btn-secondary" style={{ marginTop: 0 }} disabled={busy || !effective.default_session} onClick={clearDefaultSession}>
          Clear
        </button>
        {scope === "workspace" && effective.default_session_status === "override" && (
          <button type="button" className="btn-secondary" style={{ marginTop: 0 }} disabled={busy} onClick={() => resetField("default_session")}>
            Reset to Global
          </button>
        )}
      </FieldRow>
    </>
  );
}
