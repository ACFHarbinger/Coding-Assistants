import { useState } from "react";
import { setAgentRole } from "../../api";
import { TeamRoleBadge, ROLE_PRESETS } from "../../../common/TeamRoleBadge";

export interface RoleAssignmentControlProps {
  agentId: string;
  displayName: string;
  currentRole?: string | null;
  onRoleChanged: () => void;
}

export function RoleAssignmentControl({
  agentId,
  displayName,
  currentRole,
  onRoleChanged,
}: RoleAssignmentControlProps) {
  const [isEditing, setIsEditing] = useState(false);
  const [selectedPreset, setSelectedPreset] = useState<string>("custom");
  const [customRole, setCustomRole] = useState("");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const startEdit = () => {
    setError(null);
    if (currentRole) {
      const match = ROLE_PRESETS.find(
        (p) => p.value.toLowerCase() === currentRole.trim().toLowerCase()
      );
      if (match) {
        setSelectedPreset(match.value);
        setCustomRole(currentRole);
      } else {
        setSelectedPreset("custom");
        setCustomRole(currentRole);
      }
    } else {
      setSelectedPreset("lead");
      setCustomRole("lead");
    }
    setIsEditing(true);
  };

  const cancelEdit = () => {
    setIsEditing(false);
    setError(null);
  };

  const handleClearRole = async () => {
    setSaving(true);
    setError(null);
    try {
      await setAgentRole(agentId, null);
      onRoleChanged();
    } catch (err) {
      setError(String(err));
    } finally {
      setSaving(false);
    }
  };

  const handleSaveRole = async () => {
    const rawValue = selectedPreset === "custom" ? customRole : selectedPreset;
    const trimmed = rawValue.trim();

    if (trimmed.length > 64) {
      setError("Role cannot exceed 64 characters");
      return;
    }

    setSaving(true);
    setError(null);
    try {
      await setAgentRole(agentId, trimmed || null);
      setIsEditing(false);
      onRoleChanged();
    } catch (err) {
      setError(String(err));
    } finally {
      setSaving(false);
    }
  };

  if (isEditing) {
    return (
      <div
        style={{
          display: "inline-flex",
          flexDirection: "column",
          gap: "0.3rem",
          background: "rgba(0, 0, 0, 0.4)",
          padding: "0.35rem 0.55rem",
          borderRadius: "6px",
          border: "1px solid var(--border-color)",
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: "0.35rem", flexWrap: "wrap" }}>
          <select
            value={selectedPreset}
            onChange={(e) => {
              const val = e.target.value;
              setSelectedPreset(val);
              if (val !== "custom") {
                setCustomRole(val);
              }
            }}
            disabled={saving}
            style={{
              padding: "0.2rem 0.4rem",
              borderRadius: "5px",
              background: "rgba(15, 23, 42, 0.9)",
              color: "white",
              border: "1px solid var(--border-color)",
              fontSize: "0.75rem",
            }}
          >
            {ROLE_PRESETS.map((p) => (
              <option key={p.value} value={p.value}>
                {p.label}
              </option>
            ))}
            <option value="custom">Custom role…</option>
          </select>

          {selectedPreset === "custom" && (
            <input
              type="text"
              value={customRole}
              onChange={(e) => setCustomRole(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") void handleSaveRole();
                if (e.key === "Escape") cancelEdit();
              }}
              placeholder="e.g. DevOps, QA, Architect"
              maxLength={64}
              autoFocus
              disabled={saving}
              style={{
                padding: "0.2rem 0.45rem",
                borderRadius: "5px",
                background: "rgba(15, 23, 42, 0.9)",
                color: "white",
                border: "1px solid var(--border-color)",
                fontSize: "0.75rem",
                width: "140px",
              }}
            />
          )}

          <button
            type="button"
            className="btn-primary"
            style={{ marginTop: 0, padding: "0.2rem 0.5rem", fontSize: "0.72rem" }}
            onClick={() => void handleSaveRole()}
            disabled={saving}
          >
            {saving ? "…" : "Save"}
          </button>
          <button
            type="button"
            className="btn-secondary"
            style={{ marginTop: 0, padding: "0.2rem 0.5rem", fontSize: "0.72rem" }}
            onClick={cancelEdit}
            disabled={saving}
          >
            Cancel
          </button>
        </div>
        {error && (
          <span style={{ color: "#fca5a5", fontSize: "0.7rem" }}>{error}</span>
        )}
      </div>
    );
  }

  return (
    <div style={{ display: "inline-flex", alignItems: "center", gap: "0.35rem" }}>
      {currentRole ? (
        <>
          <TeamRoleBadge
            role={currentRole}
            size="small"
            onClear={() => void handleClearRole()}
            title={`Role: ${currentRole} (click × to clear)`}
          />
          <button
            type="button"
            className="btn-secondary"
            style={{
              marginTop: 0,
              padding: "0.15rem 0.4rem",
              fontSize: "0.68rem",
              borderRadius: "4px",
              opacity: 0.8,
            }}
            onClick={startEdit}
            title={`Change role for ${displayName}`}
          >
            Change
          </button>
        </>
      ) : (
        <button
          type="button"
          className="btn-secondary"
          style={{
            marginTop: 0,
            padding: "0.15rem 0.45rem",
            fontSize: "0.68rem",
            borderRadius: "4px",
            borderStyle: "dashed",
            opacity: 0.75,
          }}
          onClick={startEdit}
          title={`Assign a team role to ${displayName}`}
        >
          + Role
        </button>
      )}
      {error && (
        <span style={{ color: "#fca5a5", fontSize: "0.7rem", marginLeft: "0.25rem" }}>
          {error}
        </span>
      )}
    </div>
  );
}

export default RoleAssignmentControl;
