import { useState } from "react";
import { shortenPath } from "./shared";

export interface DangerTabProps {
  workspaceRoot: string | null;
  busy: boolean;
  onResetWorkspaceOverrides: () => Promise<void>;
  onChanged: () => void;
}

export default function DangerTab({
  workspaceRoot,
  busy,
  onResetWorkspaceOverrides,
  onChanged,
}: DangerTabProps) {
  const [confirmingAction, setConfirmingAction] = useState<string | null>(null);
  const [typedConfirmation, setTypedConfirmation] = useState("");
  const [dangerError, setDangerError] = useState<string | null>(null);
  const [actionSuccess, setActionSuccess] = useState<string | null>(null);

  const workspaceBasename = workspaceRoot
    ? workspaceRoot.split(/[\\/]/).filter(Boolean).pop() ?? "workspace"
    : "";

  const handleExecuteResetOverrides = async () => {
    if (!workspaceRoot) return;
    if (typedConfirmation.trim() !== workspaceBasename) {
      setDangerError(`Type "${workspaceBasename}" to confirm workspace override reset.`);
      return;
    }
    setDangerError(null);
    try {
      await onResetWorkspaceOverrides();
      setActionSuccess("All workspace overrides reset to global defaults.");
      setConfirmingAction(null);
      setTypedConfirmation("");
      onChanged();
    } catch (err) {
      setDangerError(String(err));
    }
  };

  return (
    <div style={{ display: "grid", gap: "1.5rem" }}>
      <div
        style={{
          padding: "0.85rem 1rem",
          borderRadius: "9px",
          background: "rgba(239, 68, 68, 0.10)",
          border: "1px solid rgba(248, 113, 113, 0.45)",
          color: "#fca5a5",
          fontSize: "0.85rem",
          lineHeight: 1.5,
        }}
      >
        <strong>Caution: Destructive Operations</strong>
        <p style={{ margin: "0.35rem 0 0", color: "#fecaca", fontSize: "0.8rem" }}>
          Irreversible transcript, memory, profile, and data-purge actions remain unavailable until their backing behavior is implemented and reviewed. Every action shown here requires explicit confirmation before execution.
        </p>
      </div>

      {dangerError && (
        <div
          style={{
            padding: "0.6rem 0.85rem",
            borderRadius: "8px",
            background: "rgba(239, 68, 68, 0.15)",
            border: "1px solid rgba(248, 113, 113, 0.65)",
            color: "#fca5a5",
            fontSize: "0.82rem",
          }}
        >
          {dangerError}
        </div>
      )}

      {actionSuccess && (
        <div
          style={{
            padding: "0.6rem 0.85rem",
            borderRadius: "8px",
            background: "rgba(16, 185, 129, 0.12)",
            border: "1px solid rgba(16, 185, 129, 0.35)",
            color: "#6ee7b7",
            fontSize: "0.82rem",
          }}
        >
          {actionSuccess}
        </div>
      )}

      <section
        style={{
          padding: "1rem",
          borderRadius: "10px",
          border: "1px solid rgba(248, 113, 113, 0.35)",
          background: "rgba(0,0,0,0.25)",
        }}
      >
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start", gap: "1rem", flexWrap: "wrap" }}>
          <div>
            <strong style={{ color: "var(--text-main)", fontSize: "0.95rem" }}>Reset Workspace Overrides</strong>
            <p style={{ color: "var(--text-muted)", fontSize: "0.8rem", margin: "0.3rem 0 0", lineHeight: 1.45 }}>
              Clears configuration-policy, retention, export/linking, default-session, and harness-profile overrides for the active workspace ({workspaceRoot ? shortenPath(workspaceRoot, 32) : "no workspace active"}), reverting them to global defaults. This is recoverable: no profiles, transcripts, messages, or memories are deleted.
            </p>
          </div>
          {!confirmingAction && (
            <button
              type="button"
              className="btn-secondary"
              style={{
                marginTop: 0,
                padding: "0.4rem 0.8rem",
                fontSize: "0.78rem",
                color: "#fca5a5",
                borderColor: "rgba(248, 113, 113, 0.45)",
                background: "rgba(239, 68, 68, 0.12)",
              }}
              disabled={busy || !workspaceRoot}
              onClick={() => {
                setConfirmingAction("reset_workspace");
                setTypedConfirmation("");
                setDangerError(null);
              }}
            >
              Reset Overrides
            </button>
          )}
        </div>

        {confirmingAction === "reset_workspace" && (
          <div
            style={{
              marginTop: "1rem",
              padding: "0.85rem",
              borderRadius: "8px",
              background: "rgba(239, 68, 68, 0.08)",
              border: "1px solid rgba(248, 113, 113, 0.5)",
              display: "grid",
              gap: "0.6rem",
            }}
          >
            <div style={{ fontSize: "0.82rem", color: "#fecaca" }}>
              To confirm resetting overrides for <strong>{workspaceBasename}</strong>, type the workspace name below:
            </div>
            <input
              style={{
                padding: "0.45rem 0.7rem",
                borderRadius: "8px",
                border: "1px solid rgba(248, 113, 113, 0.6)",
                background: "rgba(0,0,0,0.4)",
                color: "white",
                fontSize: "0.85rem",
              }}
              placeholder={workspaceBasename}
              value={typedConfirmation}
              onChange={(e) => setTypedConfirmation(e.target.value)}
            />
            <div style={{ display: "flex", gap: "0.5rem", justifyContent: "flex-end", marginTop: "0.25rem" }}>
              <button
                type="button"
                autoFocus
                className="btn-secondary"
                style={{ marginTop: 0, padding: "0.35rem 0.8rem", fontSize: "0.78rem" }}
                onClick={() => {
                  setConfirmingAction(null);
                  setTypedConfirmation("");
                }}
              >
                Cancel (Keep Overrides)
              </button>
              <button
                type="button"
                className="btn-primary"
                style={{
                  marginTop: 0,
                  padding: "0.35rem 0.8rem",
                  fontSize: "0.78rem",
                  background: "#dc2626",
                  borderColor: "#ef4444",
                }}
                disabled={busy || typedConfirmation.trim() !== workspaceBasename}
                onClick={() => void handleExecuteResetOverrides()}
              >
                Confirm Reset
              </button>
            </div>
          </div>
        )}
      </section>
    </div>
  );
}
