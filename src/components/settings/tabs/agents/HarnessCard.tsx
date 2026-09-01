import type {
  EffectiveHarnessSettings,
  HarnessModelCatalog,
  ProfileSnapshot,
} from "../../types";
import { StatusPill, ToggleRow, inputStyle } from "../shared";

export interface HarnessCardProps {
  harness: EffectiveHarnessSettings;
  catalog?: HarnessModelCatalog;
  profiles: ProfileSnapshot[];
  scope: "global" | "workspace";
  workspaceRoot: string | null;
  busy: boolean;
  onSelectProfile: (harness: string, profile: string) => Promise<void>;
  onSelectModel: (harness: string, model: string) => Promise<void>;
  onResetModel: (harness: string) => Promise<void>;
  onSelectEffort: (harness: string, effort: string) => Promise<void>;
  onResetEffort: (harness: string) => Promise<void>;
  onToggleField: (
    harness: EffectiveHarnessSettings,
    field: "capture_polling" | "inject_permission",
  ) => Promise<void>;
}

export function HarnessCard({
  harness,
  catalog,
  profiles,
  scope,
  workspaceRoot,
  busy,
  onSelectProfile,
  onSelectModel,
  onResetModel,
  onSelectEffort,
  onResetEffort,
  onToggleField,
}: HarnessCardProps) {
  const models = catalog?.models ?? [];
  const effortOptions = catalog?.effort_options ?? [];
  const isWorkspace = scope === "workspace" && !!workspaceRoot;

  const currentModel = harness.selected_model ?? "";
  const currentEffort = harness.selected_effort ?? "";

  return (
    <div
      style={{
        padding: "0.85rem",
        borderRadius: "10px",
        border: "1px solid var(--border-color)",
        background: "rgba(0,0,0,0.20)",
        display: "grid",
        gap: "0.75rem",
      }}
    >
      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          flexWrap: "wrap",
          gap: "0.5rem",
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
          <strong
            style={{
              color: "var(--text-main)",
              textTransform: "capitalize",
              fontSize: "0.95rem",
            }}
          >
            {harness.harness === "chat" ? "Codex / OpenAI" : harness.harness}
          </strong>
        </div>
      </div>

      {/* Model Selection */}
      <div style={{ display: "grid", gap: "0.3rem" }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
          <div style={{ display: "flex", alignItems: "center", gap: "0.4rem" }}>
            <label style={{ fontSize: "0.82rem", color: "var(--text-muted)", fontWeight: 600 }}>
              Model:
            </label>
            {harness.selected_model_status && (
              <StatusPill status={harness.selected_model_status} />
            )}
          </div>
          {isWorkspace && harness.selected_model_status === "override" && (
            <button
              type="button"
              className="btn-secondary"
              style={{ marginTop: 0, padding: "0.15rem 0.5rem", fontSize: "0.7rem" }}
              onClick={() => void onResetModel(harness.harness)}
            >
              Reset Model
            </button>
          )}
        </div>
        <select
          style={{ ...inputStyle, width: "100%", padding: "0.35rem 0.6rem" }}
          value={currentModel}
          disabled={busy || (scope === "workspace" && !workspaceRoot)}
          onChange={(e) => void onSelectModel(harness.harness, e.target.value)}
        >
          {currentModel && !models.includes(currentModel) && (
            <option value={currentModel}>{currentModel} (custom)</option>
          )}
          {models.map((m) => (
            <option key={m} value={m}>
              {m}
            </option>
          ))}
        </select>
      </div>

      {/* Effort Selection (if supported by harness) */}
      {effortOptions.length > 0 && (
        <div style={{ display: "grid", gap: "0.3rem" }}>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <div style={{ display: "flex", alignItems: "center", gap: "0.4rem" }}>
              <label style={{ fontSize: "0.82rem", color: "var(--text-muted)", fontWeight: 600 }}>
                Reasoning Effort:
              </label>
              {harness.selected_effort_status && (
                <StatusPill status={harness.selected_effort_status} />
              )}
            </div>
            {isWorkspace && harness.selected_effort_status === "override" && (
              <button
                type="button"
                className="btn-secondary"
                style={{ marginTop: 0, padding: "0.15rem 0.5rem", fontSize: "0.7rem" }}
                onClick={() => void onResetEffort(harness.harness)}
              >
                Reset Effort
              </button>
            )}
          </div>
          <select
            style={{ ...inputStyle, width: "100%", padding: "0.35rem 0.6rem" }}
            value={currentEffort}
            disabled={busy || (scope === "workspace" && !workspaceRoot)}
            onChange={(e) => void onSelectEffort(harness.harness, e.target.value)}
          >
            <option value="">Default (None)</option>
            {effortOptions.map((eff) => (
              <option key={eff} value={eff}>
                {eff.toUpperCase()}
              </option>
            ))}
          </select>
        </div>
      )}

      {/* Profile Selection */}
      <div style={{ display: "grid", gap: "0.3rem" }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
          <div style={{ display: "flex", alignItems: "center", gap: "0.4rem" }}>
            <label style={{ fontSize: "0.82rem", color: "var(--text-muted)" }}>
              Provider Profile:
            </label>
            <StatusPill status={harness.default_profile_status} />
          </div>
          {isWorkspace && harness.default_profile_status === "override" && (
            <button
              type="button"
              className="btn-secondary"
              style={{ marginTop: 0, padding: "0.15rem 0.5rem", fontSize: "0.7rem" }}
              onClick={() => void onSelectProfile(harness.harness, "")}
            >
              Reset Profile
            </button>
          )}
        </div>
        <select
          style={{ ...inputStyle, width: "100%", padding: "0.35rem 0.6rem" }}
          value={harness.default_profile ?? ""}
          disabled={busy || (scope === "workspace" && !workspaceRoot)}
          onChange={(e) => void onSelectProfile(harness.harness, e.target.value)}
        >
          <option value="">(None / Default Provider CLI)</option>
          {profiles
            .filter((p) => p.provider === harness.harness || p.provider === "custom")
            .map((p) => (
              <option key={p.name} value={p.name}>
                {p.name}
              </option>
            ))}
        </select>
      </div>

      {/* Toggles */}
      <div style={{ display: "flex", gap: "1rem", flexWrap: "wrap", marginTop: "0.2rem" }}>
        <ToggleRow
          label="Capture polling"
          hint="Periodically scan and capture on-disk session transcripts into the Hub transcript."
          checked={harness.capture_polling}
          disabled={busy}
          onToggle={() => void onToggleField(harness, "capture_polling")}
        />
        <ToggleRow
          label="Inject permission"
          hint="Allow Hub task and wake deliveries to inject turns directly into this harness."
          checked={harness.inject_permission}
          disabled={busy}
          onToggle={() => void onToggleField(harness, "inject_permission")}
        />
      </div>
    </div>
  );
}
