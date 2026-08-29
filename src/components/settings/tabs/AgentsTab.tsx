import { useEffect, useState } from "react";
import type {
  EffectiveHarnessSettings,
  EffectiveSettings,
  HarnessSettings,
  ProfileSnapshot,
  ProviderProfile,
  SecretReference,
  SecretSourceKind,
} from "../types";
import {
  listSettingsHarnesses,
  listSettingsProfiles,
  removeSettingsProfile,
  resetWorkspaceDefaultProfile,
  setWorkspaceDefaultProfile,
  updateSettingsHarness,
  upsertSettingsProfile,
} from "../api";
import { StatusPill, ToggleRow, inputStyle, shortenPath } from "./shared";

const PROVIDER_OPTIONS = [
  { id: "claude", label: "Claude Code" },
  { id: "codex", label: "Codex / OpenAI" },
  { id: "gemini", label: "Gemini / Antigravity" },
  { id: "grok", label: "Grok" },
  { id: "custom", label: "Custom Provider" },
];

export interface AgentsTabProps {
  effective: EffectiveSettings;
  scope: "global" | "workspace";
  setScope: (scope: "global" | "workspace") => void;
  workspaceRoot: string | null;
  busy: boolean;
  onChanged: () => void;
}

export default function AgentsTab({
  effective,
  scope,
  setScope,
  workspaceRoot,
  busy,
  onChanged,
}: AgentsTabProps) {
  const [profiles, setProfiles] = useState<ProfileSnapshot[]>(effective.profiles ?? []);
  const [harnesses, setHarnesses] = useState<EffectiveHarnessSettings[]>(effective.harnesses ?? []);
  const [editingProfile, setEditingProfile] = useState<boolean>(false);
  const [nameDraft, setNameDraft] = useState("");
  const [providerDraft, setProviderDraft] = useState("claude");
  const [modelDraft, setModelDraft] = useState("");
  const [baseUrlDraft, setBaseUrlDraft] = useState("");
  const [secretKindDraft, setSecretKindDraft] = useState<SecretSourceKind>("env_var");
  const [secretParamDraft, setSecretParamDraft] = useState("ANTHROPIC_API_KEY");
  const [profileError, setProfileError] = useState<string | null>(null);

  const targetWorkspace = scope === "workspace" ? workspaceRoot : null;

  const refreshProfilesAndHarnesses = async () => {
    try {
      const [pList, hList] = await Promise.all([
        listSettingsProfiles(),
        listSettingsHarnesses(targetWorkspace),
      ]);
      setProfiles(pList);
      setHarnesses(hList);
    } catch {
      // Non-critical background refresh failure
    }
  };

  useEffect(() => {
    void refreshProfilesAndHarnesses();
  }, [targetWorkspace]);

  const handleSaveProfile = async () => {
    const name = nameDraft.trim();
    if (!name) {
      setProfileError("Profile name is required.");
      return;
    }
    let secret: SecretReference;
    if (secretKindDraft === "env_var") {
      const varName = secretParamDraft.trim();
      if (!varName) {
        setProfileError("Environment variable name is required (e.g. ANTHROPIC_API_KEY).");
        return;
      }
      secret = { kind: "env_var", name: varName };
    } else if (secretKindDraft === "keychain") {
      const keyId = secretParamDraft.trim();
      if (!keyId) {
        setProfileError("Keychain identifier is required.");
        return;
      }
      secret = { kind: "keychain", id: keyId };
    } else {
      secret = { kind: "provider_login" };
    }

    const payload: ProviderProfile = {
      name,
      provider: providerDraft,
      model: modelDraft.trim() || null,
      base_url: baseUrlDraft.trim() || null,
      secret,
    };

    try {
      const updated = await upsertSettingsProfile(payload);
      setProfiles(updated);
      setEditingProfile(false);
      setNameDraft("");
      setModelDraft("");
      setBaseUrlDraft("");
      setProfileError(null);
      onChanged();
    } catch (err) {
      setProfileError(String(err));
    }
  };

  const handleRemoveProfile = async (name: string) => {
    try {
      const updated = await removeSettingsProfile(name);
      setProfiles(updated);
      onChanged();
    } catch (err) {
      setProfileError(String(err));
    }
  };

  const handleSelectDefaultProfile = async (harness: string, profileName: string) => {
    if (!workspaceRoot) return;
    try {
      if (profileName) {
        await setWorkspaceDefaultProfile(workspaceRoot, harness, profileName);
      } else {
        await resetWorkspaceDefaultProfile(workspaceRoot, harness);
      }
      await refreshProfilesAndHarnesses();
      onChanged();
    } catch (err) {
      setProfileError(String(err));
    }
  };

  const handleToggleHarnessField = async (
    h: EffectiveHarnessSettings,
    field: "capture_polling" | "inject_permission",
  ) => {
    const updated: HarnessSettings = {
      harness: h.harness,
      executable: h.executable,
      workdir: h.workdir,
      capture_polling: field === "capture_polling" ? !h.capture_polling : h.capture_polling,
      inject_permission: field === "inject_permission" ? !h.inject_permission : h.inject_permission,
    };
    try {
      await updateSettingsHarness(updated);
      await refreshProfilesAndHarnesses();
      onChanged();
    } catch (err) {
      setProfileError(String(err));
    }
  };

  return (
    <div>
      {workspaceRoot && (
        <div style={{ display: "flex", gap: "0.5rem", marginBottom: "1.25rem" }}>
          <button
            type="button"
            className={scope === "workspace" ? "btn-primary" : "btn-secondary"}
            style={{ marginTop: 0, padding: "0.35rem 0.8rem", fontSize: "0.8rem" }}
            onClick={() => setScope("workspace")}
          >
            Workspace ({shortenPath(workspaceRoot, 28)})
          </button>
          <button
            type="button"
            className={scope === "global" ? "btn-primary" : "btn-secondary"}
            style={{ marginTop: 0, padding: "0.35rem 0.8rem", fontSize: "0.8rem" }}
            onClick={() => setScope("global")}
          >
            Global defaults
          </button>
        </div>
      )}

      {profileError && (
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
          {profileError}
        </div>
      )}

      <section style={{ marginBottom: "1.75rem" }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "baseline", marginBottom: "0.75rem" }}>
          <h3 style={{ margin: 0, fontSize: "0.95rem", fontWeight: 700 }}>Named Provider Profiles</h3>
          {!editingProfile && (
            <button
              type="button"
              className="btn-secondary"
              style={{ marginTop: 0, padding: "0.3rem 0.7rem", fontSize: "0.78rem" }}
              disabled={busy}
              onClick={() => setEditingProfile(true)}
            >
              + Add profile
            </button>
          )}
        </div>
        <p style={{ color: "var(--text-muted)", fontSize: "0.8rem", margin: "0 0 0.85rem" }}>
          Global profile definitions. Secrets are stored by reference (Keychain ID or Environment Variable name) — never raw credentials.
        </p>

        {editingProfile && (
          <div
            style={{
              padding: "1rem",
              borderRadius: "10px",
              border: "1px solid var(--border-color)",
              background: "rgba(0,0,0,0.25)",
              marginBottom: "1rem",
              display: "grid",
              gap: "0.75rem",
            }}
          >
            <div style={{ display: "flex", gap: "0.6rem", flexWrap: "wrap" }}>
              <input
                style={{ ...inputStyle, flex: "1 1 160px" }}
                placeholder="Profile name (e.g. sonnet-work)"
                value={nameDraft}
                onChange={(e) => setNameDraft(e.target.value)}
              />
              <select
                style={{ ...inputStyle, flex: "0 1 180px" }}
                value={providerDraft}
                onChange={(e) => {
                  setProviderDraft(e.target.value);
                  if (e.target.value === "claude") setSecretParamDraft("ANTHROPIC_API_KEY");
                  else if (e.target.value === "codex") setSecretParamDraft("OPENAI_API_KEY");
                  else if (e.target.value === "gemini") setSecretParamDraft("GEMINI_API_KEY");
                  else if (e.target.value === "grok") setSecretParamDraft("XAI_API_KEY");
                }}
              >
                {PROVIDER_OPTIONS.map((opt) => (
                  <option key={opt.id} value={opt.id}>{opt.label}</option>
                ))}
              </select>
            </div>

            <div style={{ display: "flex", gap: "0.6rem", flexWrap: "wrap" }}>
              <input
                style={{ ...inputStyle, flex: "1 1 160px" }}
                placeholder="Model ID (optional, e.g. claude-3-7-sonnet)"
                value={modelDraft}
                onChange={(e) => setModelDraft(e.target.value)}
              />
              <input
                style={{ ...inputStyle, flex: "1 1 200px" }}
                placeholder="Base URL (optional, defaults to official endpoint)"
                value={baseUrlDraft}
                onChange={(e) => setBaseUrlDraft(e.target.value)}
              />
            </div>

            <div style={{ display: "flex", gap: "0.6rem", flexWrap: "wrap", alignItems: "center" }}>
              <select
                style={{ ...inputStyle, flex: "0 1 180px" }}
                value={secretKindDraft}
                onChange={(e) => setSecretKindDraft(e.target.value as SecretSourceKind)}
              >
                <option value="env_var">Environment Variable ($NAME)</option>
                <option value="keychain">System Keychain (Key ID)</option>
                <option value="provider_login">CLI Native Login</option>
              </select>
              {secretKindDraft !== "provider_login" && (
                <input
                  style={{ ...inputStyle, flex: "1 1 200px" }}
                  placeholder={secretKindDraft === "env_var" ? "Variable name (e.g. ANTHROPIC_API_KEY)" : "Keychain ID"}
                  value={secretParamDraft}
                  onChange={(e) => setSecretParamDraft(e.target.value)}
                />
              )}
            </div>

            <div style={{ display: "flex", gap: "0.5rem", justifyContent: "flex-end", marginTop: "0.25rem" }}>
              <button
                type="button"
                className="btn-secondary"
                style={{ marginTop: 0, padding: "0.35rem 0.8rem", fontSize: "0.8rem" }}
                onClick={() => {
                  setEditingProfile(false);
                  setProfileError(null);
                }}
              >
                Cancel
              </button>
              <button
                type="button"
                className="btn-primary"
                style={{ marginTop: 0, padding: "0.35rem 0.8rem", fontSize: "0.8rem" }}
                onClick={() => void handleSaveProfile()}
              >
                Save Profile
              </button>
            </div>
          </div>
        )}

        <div style={{ display: "grid", gap: "0.5rem" }}>
          {profiles.length === 0 && (
            <p style={{ color: "var(--text-muted)", fontSize: "0.85rem", margin: "0.5rem 0" }}>
              No custom provider profiles saved. Add one to customize model selection or credentials.
            </p>
          )}
          {profiles.map((p) => (
            <div
              key={p.name}
              style={{
                display: "flex",
                justifyContent: "space-between",
                alignItems: "center",
                gap: "0.75rem",
                padding: "0.6rem 0.85rem",
                borderRadius: "8px",
                border: "1px solid var(--border-color)",
                background: "rgba(255,255,255,0.02)",
                flexWrap: "wrap",
              }}
            >
              <div>
                <strong style={{ color: "var(--text-main)", fontSize: "0.9rem" }}>{p.name}</strong>
                <span style={{ color: "var(--text-muted)", fontSize: "0.78rem", marginLeft: "0.5rem" }}>
                  {p.provider} {p.model ? `· ${p.model}` : ""}
                </span>
                <div style={{ marginTop: "0.25rem" }}>
                  <span
                    style={{
                      fontSize: "0.7rem",
                      fontWeight: 600,
                      padding: "0.12rem 0.5rem",
                      borderRadius: "999px",
                      background: "rgba(16, 185, 129, 0.12)",
                      border: "1px solid rgba(16, 185, 129, 0.35)",
                      color: "#6ee7b7",
                    }}
                  >
                    {p.secret_badge}
                  </span>
                </div>
              </div>
              <button
                type="button"
                className="btn-secondary"
                style={{ marginTop: 0, padding: "0.25rem 0.6rem", fontSize: "0.75rem", color: "#fca5a5", borderColor: "rgba(248, 113, 113, 0.35)" }}
                onClick={() => void handleRemoveProfile(p.name)}
              >
                Delete
              </button>
            </div>
          ))}
        </div>
      </section>

      <section style={{ marginBottom: "1.75rem" }}>
        <h3 style={{ margin: "0 0 0.4rem", fontSize: "0.95rem", fontWeight: 700 }}>Harness Defaults & Runtime Policy</h3>
        <p style={{ color: "var(--text-muted)", fontSize: "0.8rem", margin: "0 0 0.85rem" }}>
          Workspace-selected default profile per harness, plus capture polling and task injection permissions.
        </p>

        <div style={{ display: "grid", gap: "0.75rem" }}>
          {harnesses.map((h) => (
            <div
              key={h.harness}
              style={{
                padding: "0.85rem",
                borderRadius: "10px",
                border: "1px solid var(--border-color)",
                background: "rgba(0,0,0,0.20)",
                display: "grid",
                gap: "0.6rem",
              }}
            >
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", flexWrap: "wrap", gap: "0.5rem" }}>
                <div style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
                  <strong style={{ color: "var(--text-main)", textTransform: "capitalize", fontSize: "0.95rem" }}>
                    {h.harness}
                  </strong>
                  <StatusPill status={h.default_profile_status} />
                </div>
                {scope === "workspace" && h.default_profile_status === "override" && workspaceRoot && (
                  <button
                    type="button"
                    className="btn-secondary"
                    style={{ marginTop: 0, padding: "0.2rem 0.55rem", fontSize: "0.72rem" }}
                    onClick={() => void handleSelectDefaultProfile(h.harness, "")}
                  >
                    Reset to Global
                  </button>
                )}
              </div>

              <div style={{ display: "flex", gap: "0.6rem", alignItems: "center", flexWrap: "wrap" }}>
                <label style={{ fontSize: "0.82rem", color: "var(--text-muted)" }}>Default Profile:</label>
                <select
                  style={{ ...inputStyle, flex: "1 1 200px", padding: "0.35rem 0.6rem" }}
                  value={h.default_profile ?? ""}
                  disabled={scope === "workspace" && !workspaceRoot}
                  onChange={(e) => void handleSelectDefaultProfile(h.harness, e.target.value)}
                >
                  <option value="">(None / Default Provider CLI)</option>
                  {profiles
                    .filter((p) => p.provider === h.harness || p.provider === "custom")
                    .map((p) => (
                      <option key={p.name} value={p.name}>{p.name}</option>
                    ))}
                </select>
              </div>

              <div style={{ display: "flex", gap: "1rem", flexWrap: "wrap", marginTop: "0.2rem" }}>
                <ToggleRow
                  label="Capture polling"
                  hint="Periodically scan and capture on-disk session transcripts into the Hub transcript."
                  checked={h.capture_polling}
                  disabled={busy}
                  onToggle={() => void handleToggleHarnessField(h, "capture_polling")}
                />
                <ToggleRow
                  label="Inject permission"
                  hint="Allow Hub task and wake deliveries to inject turns directly into this harness."
                  checked={h.inject_permission}
                  disabled={busy}
                  onToggle={() => void handleToggleHarnessField(h, "inject_permission")}
                />
              </div>
            </div>
          ))}
        </div>
      </section>
    </div>
  );
}
