import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import type { HubAgent } from "../../../../app/hubState";
import { isTauriRuntime } from "../../../../lib/tauri";
import { AgentAvatar } from "../../../panels/messager/AgentAvatar";
import { listHubAgents, setAgentDisplayName } from "../../api";
import { inputStyle } from "../shared";
import RoleAssignmentControl from "./RoleAssignmentControl";

export interface TeamProfilesSectionProps {
  onChanged?: () => void;
}

export function TeamProfilesSection({ onChanged }: TeamProfilesSectionProps) {
  const [agents, setAgents] = useState<HubAgent[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const [editingId, setEditingId] = useState<string | null>(null);
  const [draftName, setDraftName] = useState("");
  const [validationError, setValidationError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  const loadAgents = useCallback(async () => {
    try {
      const list = await listHubAgents();
      setAgents(list);
      setError(null);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadAgents();
  }, [loadAgents]);

  // Reactive updates on external or cross-window agent changes
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    if (isTauriRuntime()) {
      listen("hub:agents-changed", () => {
        void loadAgents();
      }).then((u) => {
        unlisten = u;
      });
    }
    return () => {
      unlisten?.();
    };
  }, [loadAgents]);

  const handleStartEdit = (agent: HubAgent) => {
    setEditingId(agent.id);
    setDraftName(agent.display_name);
    setValidationError(null);
  };

  const handleCancelEdit = () => {
    setEditingId(null);
    setDraftName("");
    setValidationError(null);
  };

  const handleSaveRename = async (agentId: string) => {
    const trimmed = draftName.trim();
    if (!trimmed) {
      setValidationError("Display name cannot be empty.");
      return;
    }
    if (trimmed.length > 64) {
      setValidationError("Display name cannot exceed 64 characters.");
      return;
    }
    const collision = agents.find(
      (a) => a.id !== agentId && a.display_name.toLowerCase() === trimmed.toLowerCase(),
    );
    if (collision) {
      setValidationError(`Display name "${trimmed}" is already in use by @${collision.id}.`);
      return;
    }

    setSaving(true);
    try {
      const updated = await setAgentDisplayName(agentId, trimmed);
      setAgents((prev) => prev.map((a) => (a.id === agentId ? updated : a)));
      setEditingId(null);
      setDraftName("");
      setValidationError(null);
      onChanged?.();
    } catch (err) {
      setValidationError(String(err));
    } finally {
      setSaving(false);
    }
  };

  const handleAvatarChanged = async () => {
    await loadAgents();
    onChanged?.();
  };

  return (
    <section style={{ marginBottom: "1.75rem" }}>
      <div style={{ marginBottom: "0.85rem" }}>
        <h3 style={{ margin: 0, fontSize: "0.95rem", fontWeight: 700 }}>Team & Identity Profiles</h3>
        <p style={{ color: "var(--text-muted)", fontSize: "0.8rem", margin: "0.2rem 0 0" }}>
          Avatars and display names for human developers and AI coding agents. Any identity can be renamed.
        </p>
      </div>

      {error && (
        <div
          style={{
            padding: "0.6rem 0.85rem",
            borderRadius: "8px",
            background: "rgba(239, 68, 68, 0.12)",
            border: "1px solid rgba(248, 113, 113, 0.45)",
            color: "#fca5a5",
            fontSize: "0.82rem",
            marginBottom: "1rem",
          }}
        >
          {error}
        </div>
      )}

      {loading && agents.length === 0 ? (
        <div style={{ color: "var(--text-muted)", fontSize: "0.82rem", fontStyle: "italic" }}>
          Loading profiles…
        </div>
      ) : (
        <div style={{ display: "grid", gap: "0.65rem" }}>
          {agents.map((agent) => {
            const isEditing = editingId === agent.id;
            const isHuman = agent.id === "human";

            return (
              <div
                key={agent.id}
                style={{
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "space-between",
                  gap: "1rem",
                  padding: "0.75rem 1rem",
                  borderRadius: "10px",
                  border: "1px solid var(--border-color)",
                  background: isHuman ? "rgba(59, 130, 246, 0.05)" : "rgba(255, 255, 255, 0.02)",
                  flexWrap: "wrap",
                }}
              >
                <div style={{ display: "flex", alignItems: "center", gap: "0.85rem", flex: 1, minWidth: "240px" }}>
                  <AgentAvatar
                    agentId={agent.id}
                    displayName={agent.display_name}
                    avatarAttachmentId={agent.avatar_attachment_id}
                    size={40}
                    editable={true}
                    onChanged={handleAvatarChanged}
                  />

                  <div style={{ flex: 1, minWidth: 0 }}>
                    {isEditing ? (
                      <div style={{ display: "flex", flexDirection: "column", gap: "0.4rem" }}>
                        <div style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
                          <input
                            type="text"
                            autoFocus
                            aria-label={`Display name for ${agent.id}`}
                            value={draftName}
                            onChange={(e) => {
                              setDraftName(e.target.value);
                              setValidationError(null);
                            }}
                            onKeyDown={(e) => {
                              if (e.key === "Enter") void handleSaveRename(agent.id);
                              if (e.key === "Escape") handleCancelEdit();
                            }}
                            style={{
                              ...inputStyle,
                              flex: "1 1 auto",
                              minWidth: "160px",
                              padding: "0.35rem 0.6rem",
                              fontSize: "0.85rem",
                            }}
                            disabled={saving}
                          />
                          <button
                            type="button"
                            className="btn-primary"
                            style={{ marginTop: 0, padding: "0.35rem 0.7rem", fontSize: "0.78rem" }}
                            onClick={() => void handleSaveRename(agent.id)}
                            disabled={saving}
                          >
                            {saving ? "Saving…" : "Save"}
                          </button>
                          <button
                            type="button"
                            className="btn-secondary"
                            style={{ marginTop: 0, padding: "0.35rem 0.7rem", fontSize: "0.78rem" }}
                            onClick={handleCancelEdit}
                            disabled={saving}
                          >
                            Cancel
                          </button>
                        </div>
                        {validationError && (
                          <span style={{ color: "#fca5a5", fontSize: "0.75rem" }}>{validationError}</span>
                        )}
                      </div>
                    ) : (
                      <div style={{ display: "flex", alignItems: "center", gap: "0.6rem", flexWrap: "wrap" }}>
                        <span style={{ fontWeight: 600, fontSize: "0.95rem", color: "var(--text-main)" }}>
                          {agent.display_name}
                        </span>
                        <button
                          type="button"
                          className="btn-secondary"
                          style={{
                            marginTop: 0,
                            padding: "0.2rem 0.5rem",
                            fontSize: "0.72rem",
                            borderRadius: "6px",
                          }}
                          onClick={() => handleStartEdit(agent)}
                          title={`Rename ${agent.display_name}`}
                        >
                          Rename
                        </button>
                      </div>
                    )}

                    <div style={{ display: "flex", alignItems: "center", gap: "0.45rem", marginTop: "0.25rem", flexWrap: "wrap" }}>
                      <span
                        style={{
                          fontFamily: "var(--font-mono)",
                          fontSize: "0.72rem",
                          color: "var(--text-muted)",
                          padding: "0.1rem 0.4rem",
                          borderRadius: "4px",
                          background: "rgba(255, 255, 255, 0.05)",
                        }}
                      >
                        @{agent.id}
                      </span>
                      {isHuman && (
                        <span
                          style={{
                            fontSize: "0.7rem",
                            fontWeight: 600,
                            color: "#93c5fd",
                            background: "rgba(59, 130, 246, 0.15)",
                            border: "1px solid rgba(59, 130, 246, 0.35)",
                            borderRadius: "4px",
                            padding: "0.1rem 0.4rem",
                          }}
                        >
                          You (Developer)
                        </span>
                      )}
                      {agent.team_member && (
                        <span
                          style={{
                            fontSize: "0.7rem",
                            fontWeight: 500,
                            color: "#a7f3d0",
                            background: "rgba(16, 185, 129, 0.12)",
                            border: "1px solid rgba(16, 185, 129, 0.3)",
                            borderRadius: "4px",
                            padding: "0.1rem 0.4rem",
                          }}
                        >
                          Enrolled Member
                        </span>
                      )}
                      <RoleAssignmentControl
                        agentId={agent.id}
                        displayName={agent.display_name}
                        currentRole={agent.role}
                        onRoleChanged={() => {
                          void loadAgents();
                          onChanged?.();
                        }}
                      />
                    </div>
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </section>
  );
}

export default TeamProfilesSection;
