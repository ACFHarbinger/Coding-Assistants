import { useEffect, useState } from "react";
import {
  listSettingsProfiles,
  purgeWorkspaceData,
  purgeWorkspaceMemories,
  purgeWorkspaceTranscript,
  removeSettingsProfile,
} from "../api";
import type { ProfileSnapshot } from "../types";
import DangerConfirmBox, { type DangerTone } from "./DangerConfirmBox";
import { shortenPath } from "./shared";

export interface DangerTabProps {
  workspaceRoot: string | null;
  busy: boolean;
  onResetWorkspaceOverrides: () => Promise<void>;
  onChanged: () => void;
}

function SectionShell({
  tone,
  title,
  description,
  actionLabel,
  canAct,
  confirming,
  onStart,
  children,
  busy,
}: {
  tone: DangerTone;
  title: string;
  description: string;
  actionLabel: string;
  canAct: boolean;
  confirming: boolean;
  onStart: () => void;
  children?: React.ReactNode;
  busy: boolean;
}) {
  const accent = tone === "red" ? "248, 113, 113" : "251, 191, 36";
  return (
    <section
      style={{
        padding: "1rem",
        borderRadius: "10px",
        border: `1px solid rgba(${accent}, 0.35)`,
        background: "rgba(0,0,0,0.25)",
      }}
    >
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start", gap: "1rem", flexWrap: "wrap" }}>
        <div>
          <strong style={{ color: "var(--text-main)", fontSize: "0.95rem" }}>{title}</strong>
          <p style={{ color: "var(--text-muted)", fontSize: "0.8rem", margin: "0.3rem 0 0", lineHeight: 1.45 }}>
            {description}
          </p>
        </div>
        {!confirming && (
          <button
            type="button"
            className="btn-secondary"
            style={{
              marginTop: 0,
              padding: "0.4rem 0.8rem",
              fontSize: "0.78rem",
              color: tone === "red" ? "#fca5a5" : "#fde68a",
              borderColor: `rgba(${accent}, 0.45)`,
              background: tone === "red" ? "rgba(239, 68, 68, 0.12)" : "rgba(245, 158, 11, 0.12)",
            }}
            disabled={busy || !canAct}
            onClick={onStart}
          >
            {actionLabel}
          </button>
        )}
      </div>
      {children}
    </section>
  );
}

export default function DangerTab({
  workspaceRoot,
  busy,
  onResetWorkspaceOverrides,
  onChanged,
}: DangerTabProps) {
  const [confirmingAction, setConfirmingAction] = useState<string | null>(null);
  const [dangerError, setDangerError] = useState<string | null>(null);
  const [actionSuccess, setActionSuccess] = useState<string | null>(null);
  const [executing, setExecuting] = useState(false);
  const [profiles, setProfiles] = useState<ProfileSnapshot[]>([]);
  const [profileToDelete, setProfileToDelete] = useState("");

  const workspaceBasename = workspaceRoot
    ? workspaceRoot.split(/[\\/]/).filter(Boolean).pop() ?? "workspace"
    : "";
  const workspaceLabel = workspaceRoot ? shortenPath(workspaceRoot, 32) : "no workspace active";

  useEffect(() => {
    let cancelled = false;
    void listSettingsProfiles()
      .then((list) => {
        if (!cancelled) {
          setProfiles(list);
          setProfileToDelete((current) =>
            current && list.some((p) => p.name === current) ? current : "",
          );
        }
      })
      .catch((err: unknown) => {
        if (!cancelled) setDangerError(String(err));
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const startConfirm = (action: string) => {
    setConfirmingAction(action);
    setDangerError(null);
  };
  const cancelConfirm = () => setConfirmingAction(null);

  const finishSuccess = (message: string) => {
    setActionSuccess(message);
    setConfirmingAction(null);
    setDangerError(null);
    onChanged();
  };
  const finishError = (err: unknown) => setDangerError(String(err));

  const runGuarded = async (action: () => Promise<string>) => {
    setExecuting(true);
    try {
      finishSuccess(await action());
    } catch (err) {
      finishError(err);
    } finally {
      setExecuting(false);
    }
  };

  const refreshProfiles = async () => {
    const list = await listSettingsProfiles();
    setProfiles(list);
    setProfileToDelete((current) =>
      current && list.some((p) => p.name === current) ? current : "",
    );
  };

  const blocked = busy || executing;

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
          Red sections are irreversible: purged transcripts and memories are hard-deleted and cannot
          be recovered. Amber sections are recoverable. Every action requires typing its target name
          before execution, writes an audit event on completion, and cancelling never changes data.
          Wakes and harness registrations are never touched by workspace purges.
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

      <SectionShell
        tone="amber"
        title="Reset Workspace Overrides"
        description={`Clears configuration-policy, retention, export/linking, default-session, and harness-profile overrides for the active workspace (${workspaceLabel}), reverting them to global defaults. This is recoverable: no profiles, transcripts, messages, or memories are deleted.`}
        actionLabel="Reset Overrides"
        canAct={!!workspaceRoot}
        confirming={confirmingAction === "reset_workspace"}
        onStart={() => startConfirm("reset_workspace")}
        busy={blocked}
      >
        {confirmingAction === "reset_workspace" && (
          <DangerConfirmBox
            tone="amber"
            targetName={workspaceBasename}
            prompt={`To confirm resetting overrides for ${workspaceBasename}, type the workspace name below:`}
            mismatchError={`Type "${workspaceBasename}" to confirm workspace override reset.`}
            confirmLabel="Confirm Reset"
            cancelLabel="Cancel (Keep Overrides)"
            busy={blocked}
            onCancel={cancelConfirm}
            onError={setDangerError}
            onConfirm={() =>
              void runGuarded(async () => {
                await onResetWorkspaceOverrides();
                return "All workspace overrides reset to global defaults.";
              })
            }
          />
        )}
      </SectionShell>

      <SectionShell
        tone="red"
        title="Purge Workspace Transcript"
        description={`Permanently deletes every message recorded against ${workspaceLabel} — all kinds and statuses. Irreversible: rows are hard-deleted, not cancelled. Memories, profiles, wakes, and other workspaces are untouched.`}
        actionLabel="Purge Transcript"
        canAct={!!workspaceRoot}
        confirming={confirmingAction === "purge_transcript"}
        onStart={() => startConfirm("purge_transcript")}
        busy={blocked}
      >
        {confirmingAction === "purge_transcript" && workspaceRoot && (
          <DangerConfirmBox
            targetName={workspaceBasename}
            prompt={`To confirm permanently deleting the transcript for ${workspaceBasename}, type the workspace name below:`}
            mismatchError={`Type "${workspaceBasename}" to confirm transcript purge.`}
            confirmLabel="Confirm Purge"
            cancelLabel="Cancel (Keep Transcript)"
            busy={blocked}
            onCancel={cancelConfirm}
            onError={setDangerError}
            onConfirm={() =>
              void runGuarded(async () => {
                const deleted = await purgeWorkspaceTranscript(workspaceRoot);
                return `Purged ${deleted} transcript message${deleted === 1 ? "" : "s"} for ${workspaceBasename}.`;
              })
            }
          />
        )}
      </SectionShell>

      <SectionShell
        tone="red"
        title="Purge Workspace Memories"
        description={`Permanently deletes every memory recorded against ${workspaceLabel} — all tiers, including non-stale ones (unlike retention purge, which only takes stale rows). Irreversible. Transcripts, profiles, wakes, and other workspaces are untouched.`}
        actionLabel="Purge Memories"
        canAct={!!workspaceRoot}
        confirming={confirmingAction === "purge_memories"}
        onStart={() => startConfirm("purge_memories")}
        busy={blocked}
      >
        {confirmingAction === "purge_memories" && workspaceRoot && (
          <DangerConfirmBox
            targetName={workspaceBasename}
            prompt={`To confirm permanently deleting memories for ${workspaceBasename}, type the workspace name below:`}
            mismatchError={`Type "${workspaceBasename}" to confirm memory purge.`}
            confirmLabel="Confirm Purge"
            cancelLabel="Cancel (Keep Memories)"
            busy={blocked}
            onCancel={cancelConfirm}
            onError={setDangerError}
            onConfirm={() =>
              void runGuarded(async () => {
                const deleted = await purgeWorkspaceMemories(workspaceRoot);
                return `Purged ${deleted} memor${deleted === 1 ? "y" : "ies"} for ${workspaceBasename}.`;
              })
            }
          />
        )}
      </SectionShell>

      <SectionShell
        tone="red"
        title="Purge All Workspace Data"
        description={`Permanently deletes the transcript and memories for ${workspaceLabel} in one action, with a single audit event naming both counts. Irreversible. Profiles, wakes, harness registrations, and other workspaces are untouched.`}
        actionLabel="Purge Workspace Data"
        canAct={!!workspaceRoot}
        confirming={confirmingAction === "purge_data"}
        onStart={() => startConfirm("purge_data")}
        busy={blocked}
      >
        {confirmingAction === "purge_data" && workspaceRoot && (
          <DangerConfirmBox
            targetName={workspaceBasename}
            prompt={`To confirm permanently deleting all data for ${workspaceBasename}, type the workspace name below:`}
            mismatchError={`Type "${workspaceBasename}" to confirm workspace data purge.`}
            confirmLabel="Confirm Purge"
            cancelLabel="Cancel (Keep Data)"
            busy={blocked}
            onCancel={cancelConfirm}
            onError={setDangerError}
            onConfirm={() =>
              void runGuarded(async () => {
                const report = await purgeWorkspaceData(workspaceRoot);
                return `Purged ${report.messages} transcript message${report.messages === 1 ? "" : "s"} and ${report.memories} memor${report.memories === 1 ? "y" : "ies"} for ${workspaceBasename}.`;
              })
            }
          />
        )}
      </SectionShell>

      <SectionShell
        tone="red"
        title="Delete Provider Profile"
        description="Permanently deletes a global named provider profile. Workspaces using it fall back to no workspace default. Irreversible, though an identical profile can be re-created afterwards. An audit event records the removal."
        actionLabel="Delete Profile"
        canAct={profiles.length > 0 && !!profileToDelete}
        confirming={confirmingAction === "delete_profile"}
        onStart={() => startConfirm("delete_profile")}
        busy={blocked}
      >
        <div style={{ marginTop: "0.75rem", display: "flex", gap: "0.5rem", alignItems: "center" }}>
          <label htmlFor="danger-profile-select" style={{ fontSize: "0.8rem", color: "var(--text-muted)" }}>
            Profile
          </label>
          <select
            id="danger-profile-select"
            value={profileToDelete}
            disabled={blocked || profiles.length === 0}
            onChange={(e) => setProfileToDelete(e.target.value)}
            style={{
              padding: "0.4rem 0.6rem",
              borderRadius: "8px",
              border: "1px solid rgba(248, 113, 113, 0.4)",
              background: "rgba(0,0,0,0.4)",
              color: "white",
              fontSize: "0.82rem",
            }}
          >
            <option value="">{profiles.length === 0 ? "No profiles" : "Select a profile"}</option>
            {profiles.map((p) => (
              <option key={p.name} value={p.name}>
                {p.name}
              </option>
            ))}
          </select>
        </div>
        {confirmingAction === "delete_profile" && profileToDelete && (
          <DangerConfirmBox
            targetName={profileToDelete}
            prompt={`To confirm permanently deleting profile ${profileToDelete}, type the profile name below:`}
            mismatchError={`Type "${profileToDelete}" to confirm profile deletion.`}
            confirmLabel="Confirm Delete"
            cancelLabel="Cancel (Keep Profile)"
            busy={blocked}
            onCancel={cancelConfirm}
            onError={setDangerError}
            onConfirm={() =>
              void runGuarded(async () => {
                await removeSettingsProfile(profileToDelete);
                await refreshProfiles();
                return `Profile ${profileToDelete} deleted.`;
              })
            }
          />
        )}
      </SectionShell>
    </div>
  );
}
