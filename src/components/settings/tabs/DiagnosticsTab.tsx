import { useState } from "react";
import type { EffectiveSettings, SettingsLoadStatus } from "../types";
import { FieldRow, shortenPath } from "./shared";

export interface DiagnosticsTabProps {
  effective: EffectiveSettings;
  loadStatus: SettingsLoadStatus | null;
  workspaceRoot: string | null;
  busy: boolean;
}

export default function DiagnosticsTab({
  effective,
  loadStatus,
  workspaceRoot,
  busy,
}: DiagnosticsTabProps) {
  const [logLevel, setLogLevel] = useState<string>("info");
  const [copied, setCopied] = useState(false);
  const [copyError, setCopyError] = useState<string | null>(null);

  // An export must never contain the workspace's raw absolute path. The
  // final path segment is enough to distinguish the active project without
  // revealing the user's home directory or directory structure.
  const workspaceLabel = workspaceRoot
    ? workspaceRoot.split(/[\\/]/).filter(Boolean).pop() ?? "workspace"
    : null;

  const getDiagnosticsPayload = () => {
    return JSON.stringify(
      {
        timestamp: new Date().toISOString(),
        schema_version: effective.schema_version,
        load_status: loadStatus?.status ?? "unknown",
        active_workspace_label: workspaceLabel,
        diagnostic_log_level: logLevel,
        profiles_count: effective.profiles.length,
        harnesses: effective.harnesses.map((h) => ({
          harness: h.harness,
          capture_polling: h.capture_polling,
          inject_permission: h.inject_permission,
          has_default_profile: Boolean(h.default_profile),
        })),
        orchestration_policy: {
          confirm_new_enrollment: effective.orchestration.confirm_new_enrollment,
          confirm_broadcast: effective.orchestration.confirm_broadcast,
          auto_enrollment_allowed: effective.orchestration.auto_enrollment_allowed,
          sandbox_strictness: effective.orchestration.sandbox_strictness,
          export_enabled: effective.orchestration.export_enabled,
        },
        storage: {
          backup_retention: effective.backup_retention,
          retention_days: effective.orchestration.retention_days,
        },
      },
      null,
      2,
    );
  };

  const handleCopyDiagnostics = async () => {
    setCopyError(null);
    try {
      await navigator.clipboard.writeText(getDiagnosticsPayload());
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      setCopyError("Clipboard access is unavailable. Select the redacted snapshot below to copy it manually.");
    }
  };

  return (
    <div style={{ display: "grid", gap: "1.5rem" }}>
      <section>
        <h3 style={{ margin: "0 0 0.5rem", fontSize: "0.95rem", fontWeight: 700 }}>Configuration Health</h3>
        <div
          style={{
            padding: "0.85rem 1rem",
            borderRadius: "9px",
            border: "1px solid var(--border-color)",
            background: "rgba(0,0,0,0.22)",
            display: "grid",
            gap: "0.5rem",
            fontSize: "0.85rem",
          }}
        >
          <div style={{ display: "flex", justifyContent: "space-between" }}>
            <span style={{ color: "var(--text-muted)" }}>Settings Store Status:</span>
            <span style={{ fontWeight: 600, color: loadStatus?.status === "invalid" || loadStatus?.status === "unreadable" ? "#fca5a5" : "#6ee7b7" }}>
              {loadStatus ? loadStatus.status.toUpperCase() : "LOADED"}
            </span>
          </div>
          <div style={{ display: "flex", justifyContent: "space-between" }}>
            <span style={{ color: "var(--text-muted)" }}>Schema Version:</span>
            <span style={{ color: "var(--text-main)" }}>v{effective.schema_version}</span>
          </div>
          <div style={{ display: "flex", justifyContent: "space-between" }}>
            <span style={{ color: "var(--text-muted)" }}>Active Workspace:</span>
            <span style={{ color: "var(--text-main)" }}>{workspaceRoot ? shortenPath(workspaceRoot, 32) : "(Global scope)"}</span>
          </div>
          <div style={{ display: "flex", justifyContent: "space-between" }}>
            <span style={{ color: "var(--text-muted)" }}>Registered Provider Profiles:</span>
            <span style={{ color: "var(--text-main)" }}>{effective.profiles.length} profiles</span>
          </div>
        </div>
      </section>

      <section>
        <FieldRow
          label="Diagnostics Log Level"
          hint="Included in the exported snapshot as the requested diagnostic verbosity. Runtime logging configuration is not persisted yet."
        >
          <select
            value={logLevel}
            onChange={(e) => setLogLevel(e.target.value)}
            disabled={busy}
            style={{
              padding: "0.45rem 0.7rem",
              borderRadius: "8px",
              border: "1px solid var(--border-color)",
              background: "rgba(255,255,255,0.03)",
              color: "var(--text-main)",
              fontSize: "0.85rem",
              minWidth: "160px",
            }}
          >
            <option value="error">Error only</option>
            <option value="warn">Warn + Error</option>
            <option value="info">Info (Default)</option>
            <option value="debug">Debug (Verbose)</option>
          </select>
        </FieldRow>
      </section>

      <section>
        <h3 style={{ margin: "0 0 0.4rem", fontSize: "0.95rem", fontWeight: 700 }}>Redacted Diagnostics Export</h3>
        <p style={{ color: "var(--text-muted)", fontSize: "0.8rem", margin: "0 0 0.75rem" }}>
          Export a sanitized snapshot of current settings and health state. Secrets, credentials, and raw absolute filesystem paths are strictly excluded.
        </p>

        <div style={{ display: "flex", gap: "0.6rem", alignItems: "center", flexWrap: "wrap", marginBottom: "0.75rem" }}>
          <button
            type="button"
            className="btn-primary"
            style={{ marginTop: 0, padding: "0.4rem 0.8rem", fontSize: "0.8rem" }}
            onClick={() => void handleCopyDiagnostics()}
          >
            {copied ? "Copied to Clipboard!" : "Copy Redacted Diagnostics"}
          </button>
        </div>
        {copyError && <p style={{ color: "#fca5a5", fontSize: "0.8rem", margin: "-0.25rem 0 0.75rem" }}>{copyError}</p>}

        <pre
          style={{
            maxHeight: "180px",
            overflowY: "auto",
            padding: "0.75rem",
            borderRadius: "8px",
            background: "rgba(0,0,0,0.45)",
            border: "1px solid var(--border-color)",
            color: "var(--text-muted)",
            fontSize: "0.75rem",
            lineHeight: 1.45,
            margin: 0,
          }}
        >
          {getDiagnosticsPayload()}
        </pre>
      </section>
    </div>
  );
}
