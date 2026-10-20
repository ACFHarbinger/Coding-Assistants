import { useState } from "react";
import type { GateCombinator, GateCriterion } from "./types";

interface GateCriteriaSectionProps {
  gateCriteria: GateCriterion[];
  gateCombinator: GateCombinator;
  onChangeCombinator: (combinator: GateCombinator) => void;
  onAddCriterion: (criterion: GateCriterion) => void;
  onRemoveCriterion: (index: number) => void;
}

export function GateCriteriaSection({
  gateCriteria,
  gateCombinator,
  onChangeCombinator,
  onAddCriterion,
  onRemoveCriterion,
}: GateCriteriaSectionProps) {
  const [newCriterionType, setNewCriterionType] = useState<"command" | "max_lines" | "tokens">("command");

  const handleAdd = () => {
    if (newCriterionType === "command") {
      onAddCriterion({
        kind: "command",
        program: "cargo",
        args: ["test", "--lib"],
        check: "exit_code_zero",
      });
    } else if (newCriterionType === "max_lines") {
      onAddCriterion({
        kind: "max_lines_per_file",
        max_lines: 500,
        globs: ["src/**/*.ts", "src/**/*.tsx", "crates/**/*.rs"],
      });
    } else {
      onAddCriterion({
        kind: "min_tokens_written",
        min_tokens: 1000,
      });
    }
  };

  return (
    <div style={{ padding: "0.8rem", borderRadius: "8px", background: "rgba(255,255,255,0.02)", border: "1px solid rgba(255,255,255,0.06)" }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "0.5rem" }}>
        <div>
          <span style={{ fontSize: "0.85rem", fontWeight: 600, color: "#fff" }}>Gate Criteria</span>
          <span style={{ fontSize: "0.75rem", color: "var(--text-muted, #94a3b8)", marginLeft: "0.5rem" }}>
            Conditions evaluated before marking task completed
          </span>
        </div>
        {gateCriteria.length > 1 && (
          <select
            value={gateCombinator}
            onChange={(e) => onChangeCombinator(e.target.value as GateCombinator)}
            style={{ padding: "0.2rem 0.5rem", borderRadius: "4px", background: "rgba(0,0,0,0.3)", border: "1px solid rgba(255,255,255,0.15)", color: "#fff", fontSize: "0.75rem" }}
          >
            <option value="all">Match ALL criteria</option>
            <option value="any">Match ANY criterion</option>
          </select>
        )}
      </div>

      {gateCriteria.length > 0 && (
        <div style={{ display: "flex", flexDirection: "column", gap: "0.5rem", marginBottom: "0.75rem" }}>
          {gateCriteria.map((crit, idx) => (
            <div
              key={idx}
              style={{
                display: "flex",
                justifyContent: "space-between",
                alignItems: "center",
                padding: "0.5rem 0.75rem",
                borderRadius: "6px",
                background: "rgba(255,255,255,0.03)",
                border: "1px solid rgba(255,255,255,0.08)",
                fontSize: "0.8rem",
              }}
            >
              <div>
                {crit.kind === "command" && (
                  <span>
                    🖥️ Command: <code>{crit.program} {crit.args.join(" ")}</code> ({crit.check === "exit_code_zero" ? "exit 0" : "at most max"})
                  </span>
                )}
                {crit.kind === "max_lines_per_file" && (
                  <span>
                    📏 Max Lines: ≤ {crit.max_lines} lines on globs <code>{crit.globs.join(", ")}</code>
                  </span>
                )}
                {crit.kind === "min_tokens_written" && (
                  <span>
                    ✍️ Min Tokens: ≥ {crit.min_tokens} tokens
                  </span>
                )}
              </div>
              <button
                type="button"
                onClick={() => onRemoveCriterion(idx)}
                style={{ background: "transparent", border: "none", color: "#f87171", cursor: "pointer", fontSize: "0.8rem" }}
              >
                Remove
              </button>
            </div>
          ))}
        </div>
      )}

      <div style={{ display: "flex", gap: "0.5rem", alignItems: "center" }}>
        <select
          value={newCriterionType}
          onChange={(e) => setNewCriterionType(e.target.value as "command" | "max_lines" | "tokens")}
          style={{ padding: "0.35rem 0.6rem", borderRadius: "6px", background: "rgba(0,0,0,0.3)", border: "1px solid rgba(255,255,255,0.15)", color: "#fff", fontSize: "0.8rem" }}
        >
          <option value="command">Command (P5 sandboxed os_tool)</option>
          <option value="max_lines">Max Lines Per File (500-LoC check)</option>
          <option value="tokens">Min Tokens Written (delta)</option>
        </select>
        <button
          type="button"
          onClick={handleAdd}
          className="btn-secondary"
          style={{ padding: "0.35rem 0.8rem", fontSize: "0.8rem", marginTop: 0 }}
        >
          + Add Criterion
        </button>
      </div>

      {gateCriteria.some((c) => c.kind === "min_tokens_written") && (
        <div style={{ marginTop: "0.5rem", padding: "0.4rem 0.6rem", borderRadius: "4px", background: "rgba(245, 158, 11, 0.1)", border: "1px solid rgba(245, 158, 11, 0.25)", color: "#fbbf24", fontSize: "0.75rem" }}>
          ⚠️ <strong>Accounting Gap Note:</strong> MinTokensWritten delta reads <code>agent_metrics</code> which reflects in-app AgentSystem completions, not CLI harness agents. For harness-CLI targets, rely on Command, MaxLinesPerFile, and Max Attempts.
        </div>
      )}
    </div>
  );
}
