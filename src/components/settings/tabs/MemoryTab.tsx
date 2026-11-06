import type { EffectiveSettings, SettingsField } from "../types";
import { FieldRow, MAX_BACKUP_RETENTION, MIN_BACKUP_RETENTION, StatusPill, inputStyle, shortenPath } from "./shared";
import { nocCardDenseStyle } from "../../common/telemetry/panel";

export interface MemoryTabProps {
  effective: EffectiveSettings;
  scope: "global" | "workspace";
  setScope: (scope: "global" | "workspace") => void;
  workspaceRoot: string | null;
  backupRetentionDraft: number;
  setBackupRetentionDraft: (value: number) => void;
  busy: boolean;
  saveBackupRetention: () => void;
  resetField: (field: SettingsField) => void;
}

export default function MemoryTab({
  effective,
  scope,
  setScope,
  workspaceRoot,
  backupRetentionDraft,
  setBackupRetentionDraft,
  busy,
  saveBackupRetention,
  resetField,
}: MemoryTabProps) {
  return (
    <div style={{ display: "grid", gap: "1rem" }}>
      <div style={{ display: "flex", gap: "0.5rem" }}>
        <button
          type="button"
          className={`${scope === "global" ? "btn-primary" : "btn-secondary"} noc-focusable`}
          style={{ marginTop: 0 }}
          onClick={() => setScope("global")}
        >
          Global defaults
        </button>
        <button
          type="button"
          className={`${scope === "workspace" ? "btn-primary" : "btn-secondary"} noc-focusable`}
          style={{ marginTop: 0 }}
          disabled={!workspaceRoot}
          title={workspaceRoot ?? "No workspace root is set in the main window"}
          onClick={() => setScope("workspace")}
        >
          This workspace{workspaceRoot ? ` (${shortenPath(workspaceRoot)})` : ""}
        </button>
      </div>

      <div style={{ ...nocCardDenseStyle, padding: "1.25rem" }}>
        <FieldRow
          label={`Retained settings backups (${MIN_BACKUP_RETENTION}–${MAX_BACKUP_RETENTION})`}
          hint="How many timestamped last-known-good copies of settings.toml to keep."
          pill={scope === "workspace" ? <StatusPill status={effective.backup_retention_status} /> : undefined}
        >
          <input
            type="number"
            min={MIN_BACKUP_RETENTION}
            max={MAX_BACKUP_RETENTION}
            className="noc-focusable tabular-nums"
            style={{ ...inputStyle, flex: "0 0 100px" }}
            value={backupRetentionDraft}
            onChange={(event) => setBackupRetentionDraft(Number(event.target.value))}
            disabled={busy}
          />
          <button
            type="button"
            className="btn-primary noc-focusable"
            style={{ marginTop: 0 }}
            disabled={busy}
            onClick={saveBackupRetention}
          >
            Save
          </button>
          {scope === "workspace" && effective.backup_retention_status === "override" && (
            <button
              type="button"
              className="btn-secondary noc-focusable"
              style={{ marginTop: 0 }}
              disabled={busy}
              onClick={() => resetField("backup_retention")}
            >
              Reset to Global
            </button>
          )}
        </FieldRow>
      </div>
    </div>
  );
}
