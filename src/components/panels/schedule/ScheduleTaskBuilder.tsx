import React, { useState } from "react";
import { GateCriteriaSection } from "./GateCriteriaSection";
import type {
  GateCombinator,
  GateCriterion,
  ScheduleCreateArgs,
  ScheduleTarget,
} from "./types";

interface ScheduleTaskBuilderProps {
  workspace: string;
  availableAgents: string[];
  onSubmit: (args: ScheduleCreateArgs) => Promise<void>;
  onCancel: () => void;
  submitting: boolean;
}

const DEFAULT_AGENTS = [
  "claude",
  "gemini",
  "cursor",
  "grok",
  "opencode",
  "qwen",
  "kimi",
];

export function ScheduleTaskBuilder({
  workspace,
  availableAgents,
  onSubmit,
  onCancel,
  submitting,
}: ScheduleTaskBuilderProps) {
  const [name, setName] = useState("");
  const [instruction, setInstruction] = useState("");
  const [targetKind, setTargetKind] = useState<"team" | "agent">("team");
  const agentOptions = availableAgents.length > 0 ? availableAgents : DEFAULT_AGENTS;
  const [selectedAgent, setSelectedAgent] = useState(agentOptions[0] || "claude");

  // Fire timing (default 5 minutes from now)
  const defaultFireDate = new Date(Date.now() + 5 * 60 * 1000);
  const toLocalIso = (d: Date) => {
    const pad = (n: number) => String(n).padStart(2, "0");
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
  };
  const [fireAt, setFireAt] = useState(toLocalIso(defaultFireDate));

  // Repeat & response timeout
  const [isRecurring, setIsRecurring] = useState(false);
  const [repeatIntervalSecs, setRepeatIntervalSecs] = useState(3600);
  const [responseTimeoutSecs, setResponseTimeoutSecs] = useState(3600);

  // Gates
  const [gateCombinator, setGateCombinator] = useState<GateCombinator>("all");
  const [gateCriteria, setGateCriteria] = useState<GateCriterion[]>([]);

  // Stop conditions & budget
  const [maxAttempts, setMaxAttempts] = useState(5);
  const [hasDeadline, setHasDeadline] = useState(false);
  const [deadline, setDeadline] = useState(toLocalIso(new Date(Date.now() + 24 * 3600 * 1000)));
  const [creditAllowance, setCreditAllowance] = useState<string>("");
  const [stopOnQuota, setStopOnQuota] = useState(true);

  const [formError, setFormError] = useState<string | null>(null);

  const handleAddCriterion = (criterion: GateCriterion) => {
    setGateCriteria((prev) => [...prev, criterion]);
  };

  const handleRemoveCriterion = (index: number) => {
    setGateCriteria((prev) => prev.filter((_, i) => i !== index));
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setFormError(null);

    if (!name.trim()) {
      setFormError("Task name is required.");
      return;
    }
    if (!instruction.trim()) {
      setFormError("Task instruction is required.");
      return;
    }
    if (maxAttempts < 1) {
      setFormError("Max attempts must be at least 1.");
      return;
    }

    let fireAtIso: string;
    try {
      fireAtIso = new Date(fireAt).toISOString();
    } catch {
      setFormError("Invalid fire at date/time format.");
      return;
    }

    let deadlineIso: string | null = null;
    if (hasDeadline && deadline) {
      try {
        deadlineIso = new Date(deadline).toISOString();
      } catch {
        setFormError("Invalid deadline date/time format.");
        return;
      }
    }

    const target: ScheduleTarget =
      targetKind === "team"
        ? { kind: "team" }
        : { kind: "agent", agent_id: selectedAgent };

    const parsedCredit = creditAllowance.trim() ? parseFloat(creditAllowance) : null;

    const args: ScheduleCreateArgs = {
      name: name.trim(),
      instruction: instruction.trim(),
      target,
      workspace: workspace || "default",
      fire_at: fireAtIso,
      repeat_interval_secs: isRecurring ? repeatIntervalSecs : null,
      response_timeout_secs: isRecurring ? responseTimeoutSecs : 3600,
      gate_criteria: gateCriteria,
      gate_combinator: gateCombinator,
      budget: {
        credit_allowance: parsedCredit,
        credit_spent: 0,
        stop_on_quota_exhausted: stopOnQuota,
      },
      max_attempts: maxAttempts,
      deadline: deadlineIso,
    };

    try {
      await onSubmit(args);
    } catch (err) {
      setFormError(String(err));
    }
  };

  return (
    <form
      onSubmit={handleSubmit}
      style={{
        background: "rgba(255, 255, 255, 0.04)",
        border: "1px solid rgba(255, 255, 255, 0.12)",
        borderRadius: "12px",
        padding: "1.5rem",
        display: "flex",
        flexDirection: "column",
        gap: "1.25rem",
      }}
    >
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
        <h4 style={{ margin: 0, fontSize: "1.1rem", color: "var(--text-color, #fff)" }}>
          Create Scheduled Goal
        </h4>
        <button
          type="button"
          onClick={onCancel}
          style={{
            background: "transparent",
            border: "none",
            color: "var(--text-muted, #94a3b8)",
            cursor: "pointer",
            fontSize: "1.2rem",
          }}
        >
          ✕
        </button>
      </div>

      {formError && (
        <div style={{ padding: "0.6rem 0.8rem", borderRadius: "6px", background: "rgba(239, 68, 68, 0.15)", border: "1px solid rgba(239, 68, 68, 0.3)", color: "#ef4444", fontSize: "0.85rem" }}>
          {formError}
        </div>
      )}

      {/* Name and Workspace */}
      <div style={{ display: "grid", gridTemplateColumns: "2fr 1fr", gap: "1rem" }}>
        <div>
          <label style={{ display: "block", fontSize: "0.8rem", color: "var(--text-muted, #94a3b8)", marginBottom: "0.3rem" }}>
            Task Name *
          </label>
          <input
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="e.g. Nightly test & hygiene pass"
            required
            style={{ width: "100%", padding: "0.5rem", borderRadius: "6px", background: "rgba(0,0,0,0.3)", border: "1px solid rgba(255,255,255,0.15)", color: "#fff", fontSize: "0.85rem" }}
          />
        </div>
        <div>
          <label style={{ display: "block", fontSize: "0.8rem", color: "var(--text-muted, #94a3b8)", marginBottom: "0.3rem" }}>
            Workspace
          </label>
          <input
            type="text"
            value={workspace}
            readOnly
            style={{ width: "100%", padding: "0.5rem", borderRadius: "6px", background: "rgba(0,0,0,0.2)", border: "1px solid rgba(255,255,255,0.1)", color: "#94a3b8", fontSize: "0.85rem" }}
          />
        </div>
      </div>

      {/* Instruction */}
      <div>
        <label style={{ display: "block", fontSize: "0.8rem", color: "var(--text-muted, #94a3b8)", marginBottom: "0.3rem" }}>
          Goal / Instruction *
        </label>
        <textarea
          value={instruction}
          onChange={(e) => setInstruction(e.target.value)}
          placeholder="Detailed instruction dispatched to target agent or standing team..."
          rows={3}
          required
          style={{ width: "100%", padding: "0.5rem", borderRadius: "6px", background: "rgba(0,0,0,0.3)", border: "1px solid rgba(255,255,255,0.15)", color: "#fff", fontSize: "0.85rem", resize: "vertical" }}
        />
      </div>

      {/* Target & Fire At */}
      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "1rem" }}>
        <div>
          <label style={{ display: "block", fontSize: "0.8rem", color: "var(--text-muted, #94a3b8)", marginBottom: "0.3rem" }}>
            Target
          </label>
          <div style={{ display: "flex", gap: "0.5rem", alignItems: "center" }}>
            <select
              value={targetKind}
              onChange={(e) => setTargetKind(e.target.value as "team" | "agent")}
              style={{ padding: "0.5rem", borderRadius: "6px", background: "rgba(0,0,0,0.3)", border: "1px solid rgba(255,255,255,0.15)", color: "#fff", fontSize: "0.85rem" }}
            >
              <option value="team">Full Team (Standing Roster)</option>
              <option value="agent">Single Agent</option>
            </select>
            {targetKind === "agent" && (
              <select
                value={selectedAgent}
                onChange={(e) => setSelectedAgent(e.target.value)}
                style={{ flex: 1, padding: "0.5rem", borderRadius: "6px", background: "rgba(0,0,0,0.3)", border: "1px solid rgba(255,255,255,0.15)", color: "#fff", fontSize: "0.85rem" }}
              >
                {agentOptions.map((a) => (
                  <option key={a} value={a}>
                    {a}
                  </option>
                ))}
              </select>
            )}
          </div>
        </div>

        <div>
          <label style={{ display: "block", fontSize: "0.8rem", color: "var(--text-muted, #94a3b8)", marginBottom: "0.3rem" }}>
            Initial Fire Time
          </label>
          <input
            type="datetime-local"
            value={fireAt}
            onChange={(e) => setFireAt(e.target.value)}
            required
            style={{ width: "100%", padding: "0.45rem", borderRadius: "6px", background: "rgba(0,0,0,0.3)", border: "1px solid rgba(255,255,255,0.15)", color: "#fff", fontSize: "0.85rem" }}
          />
        </div>
      </div>

      {/* Repeat Schedule */}
      <div style={{ padding: "0.8rem", borderRadius: "8px", background: "rgba(255,255,255,0.02)", border: "1px solid rgba(255,255,255,0.06)" }}>
        <label style={{ display: "flex", alignItems: "center", gap: "0.5rem", cursor: "pointer", fontSize: "0.85rem", color: "#fff" }}>
          <input
            type="checkbox"
            checked={isRecurring}
            onChange={(e) => setIsRecurring(e.target.checked)}
          />
          <span>Continuous Loop (re-prompts until gate criteria are met or stop conditions trip)</span>
        </label>

        {isRecurring && (
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "1rem", marginTop: "0.75rem" }}>
            <div>
              <label style={{ display: "block", fontSize: "0.75rem", color: "var(--text-muted, #94a3b8)", marginBottom: "0.2rem" }}>
                Repeat Interval (seconds)
              </label>
              <select
                value={repeatIntervalSecs}
                onChange={(e) => setRepeatIntervalSecs(parseInt(e.target.value, 10))}
                style={{ width: "100%", padding: "0.4rem", borderRadius: "6px", background: "rgba(0,0,0,0.3)", border: "1px solid rgba(255,255,255,0.15)", color: "#fff", fontSize: "0.8rem" }}
              >
                <option value={300}>Every 5 minutes (300s)</option>
                <option value={900}>Every 15 minutes (900s)</option>
                <option value={1800}>Every 30 minutes (1800s)</option>
                <option value={3600}>Every 1 hour (3600s)</option>
                <option value={21600}>Every 6 hours (21600s)</option>
                <option value={86400}>Every 24 hours (86400s)</option>
              </select>
            </div>
            <div>
              <label style={{ display: "block", fontSize: "0.75rem", color: "var(--text-muted, #94a3b8)", marginBottom: "0.2rem" }}>
                Response Timeout (seconds)
              </label>
              <input
                type="number"
                min={60}
                value={responseTimeoutSecs}
                onChange={(e) => setResponseTimeoutSecs(parseInt(e.target.value, 10) || 3600)}
                style={{ width: "100%", padding: "0.4rem", borderRadius: "6px", background: "rgba(0,0,0,0.3)", border: "1px solid rgba(255,255,255,0.15)", color: "#fff", fontSize: "0.8rem" }}
              />
            </div>
          </div>
        )}
      </div>

      {/* Gate Criteria Section */}
      <GateCriteriaSection
        gateCriteria={gateCriteria}
        gateCombinator={gateCombinator}
        onChangeCombinator={setGateCombinator}
        onAddCriterion={handleAddCriterion}
        onRemoveCriterion={handleRemoveCriterion}
      />

      {/* Stop Conditions & Budget */}
      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "1rem" }}>
        <div>
          <label style={{ display: "block", fontSize: "0.8rem", color: "var(--text-muted, #94a3b8)", marginBottom: "0.2rem" }}>
            Max Attempts * (Mandatory Stop)
          </label>
          <input
            type="number"
            min={1}
            max={100}
            value={maxAttempts}
            onChange={(e) => setMaxAttempts(parseInt(e.target.value, 10) || 1)}
            required
            style={{ width: "100%", padding: "0.45rem", borderRadius: "6px", background: "rgba(0,0,0,0.3)", border: "1px solid rgba(255,255,255,0.15)", color: "#fff", fontSize: "0.85rem" }}
          />
          <span style={{ fontSize: "0.7rem", color: "var(--text-muted, #94a3b8)" }}>
            Unconditional stop condition: halts dispatch when attempts reach this limit.
          </span>
        </div>

        <div>
          <label style={{ display: "flex", alignItems: "center", gap: "0.4rem", fontSize: "0.8rem", color: "var(--text-muted, #94a3b8)", marginBottom: "0.2rem" }}>
            <input
              type="checkbox"
              checked={hasDeadline}
              onChange={(e) => setHasDeadline(e.target.checked)}
            />
            <span>Wall-Clock Deadline Cutoff</span>
          </label>
          {hasDeadline && (
            <input
              type="datetime-local"
              value={deadline}
              onChange={(e) => setDeadline(e.target.value)}
              style={{ width: "100%", padding: "0.45rem", borderRadius: "6px", background: "rgba(0,0,0,0.3)", border: "1px solid rgba(255,255,255,0.15)", color: "#fff", fontSize: "0.85rem", marginBottom: "0.5rem" }}
            />
          )}

          <label style={{ display: "block", fontSize: "0.8rem", color: "var(--text-muted, #94a3b8)", marginTop: "0.4rem", marginBottom: "0.2rem" }}>
            Credit Allowance (optional)
          </label>
          <input
            type="number"
            min={0}
            step="0.1"
            value={creditAllowance}
            onChange={(e) => setCreditAllowance(e.target.value)}
            placeholder="e.g. 50.0"
            style={{ width: "100%", padding: "0.45rem", borderRadius: "6px", background: "rgba(0,0,0,0.3)", border: "1px solid rgba(255,255,255,0.15)", color: "#fff", fontSize: "0.85rem" }}
          />
        </div>
      </div>

      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", paddingTop: "0.5rem", borderTop: "1px solid rgba(255,255,255,0.06)" }}>
        <label style={{ display: "flex", alignItems: "center", gap: "0.4rem", fontSize: "0.8rem", color: "var(--text-muted, #94a3b8)" }}>
          <input
            type="checkbox"
            checked={stopOnQuota}
            onChange={(e) => setStopOnQuota(e.target.checked)}
          />
          <span>Pause on provider quota exhaustion (active when app is open)</span>
        </label>

        <div style={{ display: "flex", gap: "0.75rem" }}>
          <button
            type="button"
            onClick={onCancel}
            disabled={submitting}
            className="btn-secondary"
            style={{ marginTop: 0, padding: "0.5rem 1rem", fontSize: "0.85rem" }}
          >
            Cancel
          </button>
          <button
            type="submit"
            disabled={submitting}
            className="btn-primary"
            style={{ marginTop: 0, padding: "0.5rem 1.25rem", fontSize: "0.85rem" }}
          >
            {submitting ? "Scheduling…" : "Schedule Goal"}
          </button>
        </div>
      </div>
    </form>
  );
}
