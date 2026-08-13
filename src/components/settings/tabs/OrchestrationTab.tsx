import type { BudgetStatus, EffectiveSettings, SandboxStrictness, SettingsField } from "../types";
import { FieldRow, StatusPill, ToggleRow, inputStyle, shortenPath } from "./shared";

const SANDBOX_LEVELS: SandboxStrictness[] = ["strict", "standard", "permissive"];

type OrchestrationField = "confirm_new_enrollment" | "confirm_broadcast" | "auto_enrollment_allowed" | "export_enabled";

export interface OrchestrationTabProps {
  effective: EffectiveSettings;
  scope: "global" | "workspace";
  setScope: (scope: "global" | "workspace") => void;
  workspaceRoot: string | null;
  busy: boolean;
  confirmWakes: boolean;
  toggleConfirmWakes: () => void;
  allowAutoWake: boolean;
  toggleAllowAutoWake: () => void;
  toggleOrchestrationField: (field: OrchestrationField, current: boolean) => void;
  setSandboxStrictness: (level: SandboxStrictness) => void;
  retentionDaysDraft: string;
  setRetentionDaysDraft: (value: string) => void;
  saveRetentionDays: () => void;
  resetField: (field: SettingsField) => void;
  budgets: BudgetStatus[];
  budgetAgentIdDraft: string;
  setBudgetAgentIdDraft: (value: string) => void;
  budgetLimitDraft: string;
  setBudgetLimitDraft: (value: string) => void;
  saveBudget: () => void;
}

export default function OrchestrationTab({
  effective,
  scope,
  setScope,
  workspaceRoot,
  busy,
  confirmWakes,
  toggleConfirmWakes,
  allowAutoWake,
  toggleAllowAutoWake,
  toggleOrchestrationField,
  setSandboxStrictness,
  retentionDaysDraft,
  setRetentionDaysDraft,
  saveRetentionDays,
  resetField,
  budgets,
  budgetAgentIdDraft,
  setBudgetAgentIdDraft,
  budgetLimitDraft,
  setBudgetLimitDraft,
  saveBudget,
}: OrchestrationTabProps) {
  return (
    <>
      <ToggleRow
        label="Confirm before wakes"
        hint="Standing wake human-gate. Global only — Hub's WakePolicy has no per-workspace scope today."
        checked={confirmWakes}
        onToggle={toggleConfirmWakes}
        disabled={busy}
      />

      <ToggleRow
        label="Allow auto-wake requests"
        hint="If off, any wake attempting to bypass the human gate is rejected outright rather than falling back to requiring approval. Global only, same as the human-gate toggle above."
        checked={allowAutoWake}
        onToggle={toggleAllowAutoWake}
        disabled={busy}
      />

      <div style={{ display: "flex", gap: "0.5rem", margin: "1.4rem 0 1.1rem" }}>
        <button type="button" className={scope === "global" ? "btn-primary" : "btn-secondary"} style={{ marginTop: 0 }} onClick={() => setScope("global")}>
          Global defaults
        </button>
        <button
          type="button"
          className={scope === "workspace" ? "btn-primary" : "btn-secondary"}
          style={{ marginTop: 0 }}
          disabled={!workspaceRoot}
          title={workspaceRoot ?? "No workspace root is set in the main window"}
          onClick={() => setScope("workspace")}
        >
          This workspace{workspaceRoot ? ` (${shortenPath(workspaceRoot)})` : ""}
        </button>
      </div>

      <ToggleRow
        label="Confirm new enrollment"
        hint="Confirm before a wake enrolls a not-yet-team-member identity."
        checked={effective.orchestration.confirm_new_enrollment}
        onToggle={() => toggleOrchestrationField("confirm_new_enrollment", effective.orchestration.confirm_new_enrollment)}
        disabled={busy}
        pill={scope === "workspace" ? <StatusPill status={effective.orchestration.confirm_new_enrollment_status} /> : undefined}
        resetButton={
          scope === "workspace" && effective.orchestration.confirm_new_enrollment_status === "override" ? (
            <button type="button" className="btn-secondary" style={{ marginTop: 0, padding: "0.25rem 0.6rem", fontSize: "0.72rem" }} disabled={busy} onClick={() => resetField("confirm_new_enrollment")}>
              Reset to Global
            </button>
          ) : undefined
        }
      />

      <ToggleRow
        label="Confirm broadcasts"
        hint="Confirm before an all/team broadcast send."
        checked={effective.orchestration.confirm_broadcast}
        onToggle={() => toggleOrchestrationField("confirm_broadcast", effective.orchestration.confirm_broadcast)}
        disabled={busy}
        pill={scope === "workspace" ? <StatusPill status={effective.orchestration.confirm_broadcast_status} /> : undefined}
        resetButton={
          scope === "workspace" && effective.orchestration.confirm_broadcast_status === "override" ? (
            <button type="button" className="btn-secondary" style={{ marginTop: 0, padding: "0.25rem 0.6rem", fontSize: "0.72rem" }} disabled={busy} onClick={() => resetField("confirm_broadcast")}>
              Reset to Global
            </button>
          ) : undefined
        }
      />

      <ToggleRow
        label="Allow auto-enrollment"
        hint="Whether a wake may enroll a brand-new harness identity at all. When off, a wake to an unknown identity is refused rather than silently enrolling it."
        checked={effective.orchestration.auto_enrollment_allowed}
        onToggle={() => toggleOrchestrationField("auto_enrollment_allowed", effective.orchestration.auto_enrollment_allowed)}
        disabled={busy}
        pill={scope === "workspace" ? <StatusPill status={effective.orchestration.auto_enrollment_allowed_status} /> : undefined}
        resetButton={
          scope === "workspace" && effective.orchestration.auto_enrollment_allowed_status === "override" ? (
            <button type="button" className="btn-secondary" style={{ marginTop: 0, padding: "0.25rem 0.6rem", fontSize: "0.72rem" }} disabled={busy} onClick={() => resetField("auto_enrollment_allowed")}>
              Reset to Global
            </button>
          ) : undefined
        }
      />

      <ToggleRow
        label="Allow non-destructive export"
        hint="Whether Markdown export actions are available."
        checked={effective.orchestration.export_enabled}
        onToggle={() => toggleOrchestrationField("export_enabled", effective.orchestration.export_enabled)}
        disabled={busy}
        pill={scope === "workspace" ? <StatusPill status={effective.orchestration.export_enabled_status} /> : undefined}
        resetButton={
          scope === "workspace" && effective.orchestration.export_enabled_status === "override" ? (
            <button type="button" className="btn-secondary" style={{ marginTop: 0, padding: "0.25rem 0.6rem", fontSize: "0.72rem" }} disabled={busy} onClick={() => resetField("export_enabled")}>
              Reset to Global
            </button>
          ) : undefined
        }
      />

      <FieldRow
        label="Sandbox strictness"
        hint="Strict refuses to start or inject a harness that can't run without bypassing approval (currently: vibe)."
        pill={scope === "workspace" ? <StatusPill status={effective.orchestration.sandbox_strictness_status} /> : undefined}
      >
        {SANDBOX_LEVELS.map((level) => (
          <button
            key={level}
            type="button"
            className={effective.orchestration.sandbox_strictness === level ? "btn-primary" : "btn-secondary"}
            style={{ marginTop: 0, textTransform: "capitalize" }}
            disabled={busy}
            onClick={() => setSandboxStrictness(level)}
          >
            {level}
          </button>
        ))}
        {scope === "workspace" && effective.orchestration.sandbox_strictness_status === "override" && (
          <button type="button" className="btn-secondary" style={{ marginTop: 0 }} disabled={busy} onClick={() => resetField("sandbox_strictness")}>
            Reset to Global
          </button>
        )}
      </FieldRow>

      <FieldRow
        label="Transcript/memory retention (days)"
        hint="Empty means indefinite. A workspace override always names a concrete day count — use Reset to Global to clear it."
        pill={scope === "workspace" ? <StatusPill status={effective.orchestration.retention_days_status} /> : undefined}
      >
        <input
          type="number"
          min={1}
          placeholder="indefinite"
          style={{ ...inputStyle, flex: "0 0 140px" }}
          value={retentionDaysDraft}
          onChange={(event) => setRetentionDaysDraft(event.target.value)}
          disabled={busy}
        />
        <button type="button" className="btn-primary" style={{ marginTop: 0 }} disabled={busy} onClick={saveRetentionDays}>
          Save
        </button>
        {scope === "workspace" && effective.orchestration.retention_days_status === "override" && (
          <button type="button" className="btn-secondary" style={{ marginTop: 0 }} disabled={busy} onClick={() => resetField("retention_days")}>
            Reset to Global
          </button>
        )}
      </FieldRow>

      <div style={{ marginTop: "1.5rem", paddingTop: "1.1rem", borderTop: "1px solid var(--border-color)" }}>
        <label style={{ fontWeight: 600, fontSize: "0.9rem", display: "block", marginBottom: "0.5rem" }}>Per-agent budgets</label>
        <p style={{ margin: "0 0 0.6rem", color: "var(--text-muted)", fontSize: "0.8rem" }}>
          Global only. Stored in the same Hub budget table every C6 flow already reads.
        </p>
        {budgets.length === 0 && <p style={{ color: "var(--text-muted)", fontSize: "0.8rem" }}>No agent budgets configured yet.</p>}
        {budgets.map((budget) => (
          <div key={budget.agent_id} style={{ display: "flex", gap: "0.6rem", alignItems: "center", fontSize: "0.82rem", padding: "0.3rem 0" }}>
            <strong style={{ minWidth: "90px" }}>{budget.agent_id}</strong>
            <span style={{ color: "var(--text-muted)" }}>
              {budget.spent_units} / {budget.limit_units} units
            </span>
            {budget.paused && (
              <span style={{ color: "#fca5a5", fontSize: "0.72rem", fontWeight: 600 }}>Paused</span>
            )}
          </div>
        ))}
        <div style={{ display: "flex", gap: "0.5rem", marginTop: "0.6rem", flexWrap: "wrap" }}>
          <input
            style={{ ...inputStyle, flex: "0 0 140px" }}
            placeholder="agent id"
            value={budgetAgentIdDraft}
            onChange={(event) => setBudgetAgentIdDraft(event.target.value)}
            disabled={busy}
          />
          <input
            type="number"
            min={0}
            style={{ ...inputStyle, flex: "0 0 120px" }}
            placeholder="limit units"
            value={budgetLimitDraft}
            onChange={(event) => setBudgetLimitDraft(event.target.value)}
            disabled={busy}
          />
          <button type="button" className="btn-primary" style={{ marginTop: 0 }} disabled={busy} onClick={saveBudget}>
            Set budget
          </button>
        </div>
      </div>
    </>
  );
}
