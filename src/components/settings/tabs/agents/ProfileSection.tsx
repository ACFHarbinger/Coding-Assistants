import { useState } from "react";
import type { ProfileSnapshot, ProviderProfile, SecretReference, SecretSourceKind } from "../../types";
import { inputStyle } from "../shared";

const PROVIDER_OPTIONS = [
  { id: "claude", label: "Claude Code" },
  { id: "codex", label: "Codex / OpenAI" },
  { id: "gemini", label: "Gemini / Antigravity" },
  { id: "grok", label: "Grok" },
  { id: "opencode", label: "OpenCode" },
  { id: "deepseek", label: "DeepSeek" },
  { id: "vibe", label: "Mistral Vibe" },
  { id: "custom", label: "Custom Provider" },
];

export interface ProfileSectionProps {
  profiles: ProfileSnapshot[];
  onSaveProfile: (profile: ProviderProfile) => Promise<void>;
  onRemoveProfile: (name: string) => Promise<void>;
  setProfileError: (error: string | null) => void;
}

export function ProfileSection({
  profiles,
  onSaveProfile,
  onRemoveProfile,
  setProfileError,
}: ProfileSectionProps) {
  const [editingProfile, setEditingProfile] = useState<boolean>(false);
  const [nameDraft, setNameDraft] = useState("");
  const [providerDraft, setProviderDraft] = useState("claude");
  const [modelDraft, setModelDraft] = useState("");
  const [baseUrlDraft, setBaseUrlDraft] = useState("");
  const [secretKindDraft, setSecretKindDraft] = useState<SecretSourceKind>("env_var");
  const [secretParamDraft, setSecretParamDraft] = useState("ANTHROPIC_API_KEY");

  const handleSave = async () => {
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
      await onSaveProfile(payload);
      setEditingProfile(false);
      setNameDraft("");
      setModelDraft("");
      setBaseUrlDraft("");
      setProfileError(null);
    } catch (err) {
      setProfileError(String(err));
    }
  };

  return (
    <section style={{ marginBottom: "1.75rem" }}>
      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          marginBottom: "0.5rem",
        }}
      >
        <div>
          <h3 style={{ margin: 0, fontSize: "0.95rem", fontWeight: 700 }}>Provider Profiles</h3>
          <p style={{ color: "var(--text-muted)", fontSize: "0.8rem", margin: "0.2rem 0 0" }}>
            Global credential bindings and endpoint overrides (e.g. Anthropic, OpenAI, xAI).
          </p>
        </div>
        {!editingProfile && (
          <button
            type="button"
            className="btn-primary"
            style={{ marginTop: 0, padding: "0.3rem 0.75rem", fontSize: "0.8rem" }}
            onClick={() => {
              setEditingProfile(true);
              setProfileError(null);
            }}
          >
            + Add Profile
          </button>
        )}
      </div>

      {editingProfile && (
        <div
          style={{
            padding: "0.85rem",
            borderRadius: "10px",
            border: "1px solid var(--accent-primary, #3b82f6)",
            background: "rgba(59, 130, 246, 0.05)",
            display: "grid",
            gap: "0.6rem",
            marginBottom: "0.85rem",
          }}
        >
          <strong style={{ fontSize: "0.88rem", color: "var(--text-main)" }}>
            New Provider Profile
          </strong>

          <div style={{ display: "flex", gap: "0.6rem", flexWrap: "wrap" }}>
            <input
              style={{ ...inputStyle, flex: "1 1 180px" }}
              placeholder="Profile Name (e.g. personal, work)"
              value={nameDraft}
              onChange={(e) => setNameDraft(e.target.value)}
            />
            <select
              style={{ ...inputStyle, flex: "1 1 180px" }}
              value={providerDraft}
              onChange={(e) => {
                setProviderDraft(e.target.value);
                if (e.target.value === "claude") setSecretParamDraft("ANTHROPIC_API_KEY");
                else if (e.target.value === "codex") setSecretParamDraft("OPENAI_API_KEY");
                else if (e.target.value === "gemini") setSecretParamDraft("GEMINI_API_KEY");
                else if (e.target.value === "grok") setSecretParamDraft("XAI_API_KEY");
                else if (e.target.value === "opencode") setSecretParamDraft("OPENCODE_API_KEY");
                else if (e.target.value === "deepseek") setSecretParamDraft("DEEPSEEK_API_KEY");
              }}
            >
              {PROVIDER_OPTIONS.map((opt) => (
                <option key={opt.id} value={opt.id}>
                  {opt.label}
                </option>
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
                placeholder={
                  secretKindDraft === "env_var"
                    ? "Variable name (e.g. ANTHROPIC_API_KEY)"
                    : "Keychain ID"
                }
                value={secretParamDraft}
                onChange={(e) => setSecretParamDraft(e.target.value)}
              />
            )}
          </div>

          <div
            style={{
              display: "flex",
              gap: "0.5rem",
              justifyContent: "flex-end",
              marginTop: "0.25rem",
            }}
          >
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
              onClick={() => void handleSave()}
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
              <span
                style={{
                  color: "var(--text-muted)",
                  fontSize: "0.78rem",
                  marginLeft: "0.5rem",
                }}
              >
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
              style={{
                marginTop: 0,
                padding: "0.25rem 0.6rem",
                fontSize: "0.75rem",
                color: "#fca5a5",
                borderColor: "rgba(248, 113, 113, 0.35)",
              }}
              onClick={() => void onRemoveProfile(p.name)}
            >
              Delete
            </button>
          </div>
        ))}
      </div>
    </section>
  );
}
