import type { EffectiveSettings } from "../types";
import { FieldRow, inputStyle } from "./shared";
import { nocCardDenseStyle } from "../../common/telemetry/panel";

export interface GeneralTabProps {
  effective: EffectiveSettings;
  defaultWorkspaceDraft: string;
  setDefaultWorkspaceDraft: (value: string) => void;
  workspaceRoot: string | null;
  busy: boolean;
  saveDefaultWorkspace: () => void;
  clearDefaultWorkspace: () => void;
}

export default function GeneralTab({
  effective,
  defaultWorkspaceDraft,
  setDefaultWorkspaceDraft,
  workspaceRoot,
  busy,
  saveDefaultWorkspace,
  clearDefaultWorkspace,
}: GeneralTabProps) {
  return (
    <div style={{ ...nocCardDenseStyle, padding: "1.25rem" }}>
      <FieldRow
        label="Default workspace"
        hint="Which workspace opens on launch when Orchestrate isn't restoring the last session. Global only — there is no per-workspace override of this."
      >
        <input
          style={inputStyle}
          className="noc-focusable"
          value={defaultWorkspaceDraft}
          onChange={(event) => setDefaultWorkspaceDraft(event.target.value)}
          placeholder="/path/to/workspace"
          disabled={busy}
        />
        <button
          type="button"
          className="btn-secondary noc-focusable"
          style={{ marginTop: 0 }}
          disabled={busy || !workspaceRoot}
          onClick={() => setDefaultWorkspaceDraft(workspaceRoot ?? "")}
        >
          Use current workspace
        </button>
        <button
          type="button"
          className="btn-primary noc-focusable"
          style={{ marginTop: 0 }}
          disabled={busy}
          onClick={saveDefaultWorkspace}
        >
          Save
        </button>
        <button
          type="button"
          className="btn-secondary noc-focusable"
          style={{ marginTop: 0 }}
          disabled={busy || !effective.default_workspace}
          onClick={clearDefaultWorkspace}
        >
          Clear
        </button>
      </FieldRow>
    </div>
  );
}
