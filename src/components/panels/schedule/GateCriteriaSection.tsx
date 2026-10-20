import { useState } from "react";
import type { GateCheck, GateCombinator, GateCriterion } from "./types";

interface GateCriteriaSectionProps {
  gateCriteria: GateCriterion[];
  gateCombinator: GateCombinator;
  isRecurring: boolean;
  onChangeCombinator: (combinator: GateCombinator) => void;
  onAddCriterion: (criterion: GateCriterion) => void;
  onRemoveCriterion: (index: number) => void;
}

export function parseArgv(input: string): string[] {
  const trimmed = input.trim();
  if (!trimmed) return [];
  const regex = /[^\s"']+|"([^"]*)"|'([^']*)'/g;
  const matches: string[] = [];
  let match: RegExpExecArray | null;
  while ((match = regex.exec(trimmed)) !== null) {
    if (match[1] !== undefined) {
      matches.push(match[1]);
    } else if (match[2] !== undefined) {
      matches.push(match[2]);
    } else {
      matches.push(match[0]);
    }
  }
  return matches;
}

export function GateCriteriaSection({
  gateCriteria,
  gateCombinator,
  isRecurring,
  onChangeCombinator,
  onAddCriterion,
  onRemoveCriterion,
}: GateCriteriaSectionProps) {
  const [newCriterionType, setNewCriterionType] = useState<"command" | "max_lines" | "tokens">("command");
  const [criterionError, setCriterionError] = useState<string | null>(null);

  // Command criterion inputs
  const [cmdProgram, setCmdProgram] = useState("cargo");
  const [cmdArgs, setCmdArgs] = useState("test --lib");
  const [cmdCheckType, setCmdCheckType] = useState<"exit_code_zero" | "at_most">("exit_code_zero");
  const [cmdCheckMax, setCmdCheckMax] = useState<number | string>(0);

  // Max lines criterion inputs
  const [maxLines, setMaxLines] = useState<number | string>(500);
  const [globs, setGlobs] = useState("src/**/*.ts, src/**/*.tsx, crates/**/*.rs");

  // Min tokens criterion inputs
  const [minTokens, setMinTokens] = useState<number | string>(1000);

  const handleAdd = () => {
    setCriterionError(null);

    if (!isRecurring) {
      setCriterionError("Gates are not evaluated on one-shot tasks. Enable Repeat Task to configure gates.");
      return;
    }

    if (newCriterionType === "command") {
      const prog = cmdProgram.trim();
      if (!prog) {
        setCriterionError("Program name is required (e.g. cargo, pytest, npm).");
        return;
      }
      let check: GateCheck = "exit_code_zero";
      if (cmdCheckType === "at_most") {
        const maxVal = parseInt(String(cmdCheckMax), 10);
        if (isNaN(maxVal) || maxVal < 0) {
          setCriterionError("Max exit code must be a non-negative integer (≥ 0).");
          return;
        }
        check = { at_most: { max: maxVal } };
      }
      const parsedArgs = parseArgv(cmdArgs);
      onAddCriterion({
        kind: "command",
        program: prog,
        args: parsedArgs,
        check,
      });
    } else if (newCriterionType === "max_lines") {
      const parsedLines = parseInt(String(maxLines), 10);
      if (isNaN(parsedLines) || parsedLines < 1) {
        setCriterionError("Max lines must be a positive integer (≥ 1).");
        return;
      }
      const parsedGlobs = globs
        .split(/[,\s]+/)
        .map((g) => g.trim())
        .filter(Boolean);
      if (parsedGlobs.length === 0) {
        setCriterionError("At least one glob pattern is required.");
        return;
      }
      onAddCriterion({
        kind: "max_lines_per_file",
        max_lines: parsedLines,
        globs: parsedGlobs,
      });
    } else {
      const parsedTokens = parseInt(String(minTokens), 10);
      if (isNaN(parsedTokens) || parsedTokens < 1) {
        setCriterionError("Min tokens written must be a positive integer (≥ 1).");
        return;
      }
      onAddCriterion({
        kind: "min_tokens_written",
        min_tokens: parsedTokens,
      });
    }
  };

  return (
    <div style={{ padding: "0.8rem", borderRadius: "8px", background: "rgba(255,255,255,0.02)", border: "1px solid rgba(255,255,255,0.06)" }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "0.5rem" }}>
        <div>
          <span style={{ fontSize: "0.85rem", fontWeight: 600, color: "#fff" }}>Gate Criteria</span>
          <span style={{ fontSize: "0.75rem", color: "var(--text-muted, #94a3b8)", marginLeft: "0.5rem" }}>
            Conditions evaluated before marking recurring task completed
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

      {!isRecurring && (
        <div
          data-testid="one-shot-gate-note"
          style={{
            padding: "0.5rem 0.75rem",
            borderRadius: "6px",
            background: "rgba(59, 130, 246, 0.1)",
            border: "1px solid rgba(59, 130, 246, 0.25)",
            color: "#93c5fd",
            fontSize: "0.75rem",
            lineHeight: 1.4,
            marginBottom: "0.6rem",
          }}
        >
          ℹ️ <strong>One-shot tasks do not evaluate gates:</strong> In one-shot mode, the engine runs once and transitions directly to completed or failed without checking gate criteria. Gate criteria are only evaluated between iterations of recurring tasks. Enable <strong>Repeat Task</strong> above to configure gate evaluation.
        </div>
      )}

      {gateCriteria.length > 0 && (
        <div style={{ display: "flex", flexDirection: "column", gap: "0.5rem", marginBottom: "0.75rem" }}>
          {gateCriteria.map((crit, idx) => (
            <div
              key={idx}
              data-testid={`gate-criterion-item-${idx}`}
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
                    🖥️ Command: <code>{crit.program} {crit.args.join(" ")}</code> (
                    {crit.check === "exit_code_zero"
                      ? "exit 0"
                      : `exit ≤ ${crit.check.at_most.max}`}
                    )
                  </span>
                )}
                {crit.kind === "max_lines_per_file" && (
                  <span>
                    📏 Max Lines: ≤ {crit.max_lines} lines on <code>{crit.globs.join(", ")}</code>
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
                aria-label={`Remove criterion ${idx + 1}`}
                style={{ background: "transparent", border: "none", color: "#f87171", cursor: "pointer", fontSize: "0.8rem" }}
              >
                Remove
              </button>
            </div>
          ))}
        </div>
      )}

      {/* Criterion Editor */}
      <div style={{ opacity: isRecurring ? 1 : 0.5, pointerEvents: isRecurring ? "auto" : "none" }}>
        <div style={{ display: "flex", gap: "0.5rem", alignItems: "center", marginBottom: "0.5rem" }}>
          <select
            value={newCriterionType}
            onChange={(e) => {
              setNewCriterionType(e.target.value as "command" | "max_lines" | "tokens");
              setCriterionError(null);
            }}
            aria-label="Criterion Type"
            disabled={!isRecurring}
            style={{ padding: "0.35rem 0.6rem", borderRadius: "6px", background: "rgba(0,0,0,0.3)", border: "1px solid rgba(255,255,255,0.15)", color: "#fff", fontSize: "0.8rem" }}
          >
            <option value="command">Command (P5 sandboxed os_tool)</option>
            <option value="max_lines">Max Lines Per File (LoC check)</option>
            <option value="tokens">Min Tokens Written (delta)</option>
          </select>

          {/* Quick presets */}
          {newCriterionType === "command" && (
            <div style={{ display: "flex", gap: "0.3rem" }}>
              <button
                type="button"
                onClick={() => {
                  setCmdProgram("cargo");
                  setCmdArgs("test --lib");
                  setCmdCheckType("exit_code_zero");
                  setCriterionError(null);
                }}
                style={{ background: "rgba(255,255,255,0.06)", border: "1px solid rgba(255,255,255,0.1)", borderRadius: "4px", color: "var(--text-muted, #94a3b8)", fontSize: "0.7rem", padding: "0.2rem 0.4rem", cursor: "pointer" }}
              >
                Preset: cargo test
              </button>
              <button
                type="button"
                onClick={() => {
                  setCmdProgram("cargo");
                  setCmdArgs("clippy --all-targets");
                  setCmdCheckType("exit_code_zero");
                  setCriterionError(null);
                }}
                style={{ background: "rgba(255,255,255,0.06)", border: "1px solid rgba(255,255,255,0.1)", borderRadius: "4px", color: "var(--text-muted, #94a3b8)", fontSize: "0.7rem", padding: "0.2rem 0.4rem", cursor: "pointer" }}
              >
                Preset: clippy
              </button>
            </div>
          )}
          {newCriterionType === "max_lines" && (
            <div style={{ display: "flex", gap: "0.3rem" }}>
              <button
                type="button"
                onClick={() => {
                  setMaxLines(500);
                  setGlobs("src/**/*.ts, src/**/*.tsx, crates/**/*.rs");
                  setCriterionError(null);
                }}
                style={{ background: "rgba(255,255,255,0.06)", border: "1px solid rgba(255,255,255,0.1)", borderRadius: "4px", color: "var(--text-muted, #94a3b8)", fontSize: "0.7rem", padding: "0.2rem 0.4rem", cursor: "pointer" }}
              >
                Preset: 500 lines
              </button>
              <button
                type="button"
                onClick={() => {
                  setMaxLines(300);
                  setGlobs("src/**/*.ts, src/**/*.tsx");
                  setCriterionError(null);
                }}
                style={{ background: "rgba(255,255,255,0.06)", border: "1px solid rgba(255,255,255,0.1)", borderRadius: "4px", color: "var(--text-muted, #94a3b8)", fontSize: "0.7rem", padding: "0.2rem 0.4rem", cursor: "pointer" }}
              >
                Preset: 300 lines
              </button>
            </div>
          )}
          {newCriterionType === "tokens" && (
            <div style={{ display: "flex", gap: "0.3rem" }}>
              <button
                type="button"
                onClick={() => {
                  setMinTokens(1000);
                  setCriterionError(null);
                }}
                style={{ background: "rgba(255,255,255,0.06)", border: "1px solid rgba(255,255,255,0.1)", borderRadius: "4px", color: "var(--text-muted, #94a3b8)", fontSize: "0.7rem", padding: "0.2rem 0.4rem", cursor: "pointer" }}
              >
                Preset: 1,000
              </button>
              <button
                type="button"
                onClick={() => {
                  setMinTokens(5000);
                  setCriterionError(null);
                }}
                style={{ background: "rgba(255,255,255,0.06)", border: "1px solid rgba(255,255,255,0.1)", borderRadius: "4px", color: "var(--text-muted, #94a3b8)", fontSize: "0.7rem", padding: "0.2rem 0.4rem", cursor: "pointer" }}
              >
                Preset: 5,000
              </button>
            </div>
          )}
        </div>

        {/* Dynamic input controls per criterion kind */}
        {newCriterionType === "command" && (
          <div style={{ display: "grid", gridTemplateColumns: "1fr 2fr 1.5fr", gap: "0.5rem", marginBottom: "0.5rem" }}>
            <div>
              <label style={{ display: "block", fontSize: "0.7rem", color: "var(--text-muted, #94a3b8)", marginBottom: "0.2rem" }}>
                Program *
              </label>
              <input
                type="text"
                value={cmdProgram}
                onChange={(e) => {
                  setCmdProgram(e.target.value);
                  setCriterionError(null);
                }}
                placeholder="e.g. cargo, pytest, npm"
                aria-label="Command Program"
                style={{ width: "100%", padding: "0.35rem 0.5rem", borderRadius: "4px", background: "rgba(0,0,0,0.3)", border: "1px solid rgba(255,255,255,0.15)", color: "#fff", fontSize: "0.8rem" }}
              />
            </div>
            <div>
              <label style={{ display: "block", fontSize: "0.7rem", color: "var(--text-muted, #94a3b8)", marginBottom: "0.2rem" }}>
                Arguments (explicit argv, space-separated, no shell)
              </label>
              <input
                type="text"
                value={cmdArgs}
                onChange={(e) => {
                  setCmdArgs(e.target.value);
                  setCriterionError(null);
                }}
                placeholder="e.g. test --lib"
                aria-label="Command Arguments"
                style={{ width: "100%", padding: "0.35rem 0.5rem", borderRadius: "4px", background: "rgba(0,0,0,0.3)", border: "1px solid rgba(255,255,255,0.15)", color: "#fff", fontSize: "0.8rem" }}
              />
            </div>
            <div>
              <label style={{ display: "block", fontSize: "0.7rem", color: "var(--text-muted, #94a3b8)", marginBottom: "0.2rem" }}>
                Exit Check
              </label>
              <div style={{ display: "flex", gap: "0.3rem" }}>
                <select
                  value={cmdCheckType}
                  onChange={(e) => {
                    setCmdCheckType(e.target.value as "exit_code_zero" | "at_most");
                    setCriterionError(null);
                  }}
                  aria-label="Command Check Type"
                  style={{ flex: 1, padding: "0.35rem 0.4rem", borderRadius: "4px", background: "rgba(0,0,0,0.3)", border: "1px solid rgba(255,255,255,0.15)", color: "#fff", fontSize: "0.75rem" }}
                >
                  <option value="exit_code_zero">Exit 0 (pass)</option>
                  <option value="at_most">Exit ≤ N</option>
                </select>
                {cmdCheckType === "at_most" && (
                  <input
                    type="number"
                    min={0}
                    value={cmdCheckMax}
                    onChange={(e) => {
                      setCmdCheckMax(e.target.value);
                      setCriterionError(null);
                    }}
                    placeholder="Max"
                    aria-label="Max Exit Code"
                    style={{ width: "55px", padding: "0.35rem 0.4rem", borderRadius: "4px", background: "rgba(0,0,0,0.3)", border: "1px solid rgba(255,255,255,0.15)", color: "#fff", fontSize: "0.75rem" }}
                  />
                )}
              </div>
            </div>
          </div>
        )}

        {newCriterionType === "max_lines" && (
          <div style={{ display: "grid", gridTemplateColumns: "1fr 2.5fr", gap: "0.5rem", marginBottom: "0.5rem" }}>
            <div>
              <label style={{ display: "block", fontSize: "0.7rem", color: "var(--text-muted, #94a3b8)", marginBottom: "0.2rem" }}>
                Max Code Lines *
              </label>
              <input
                type="number"
                min={1}
                value={maxLines}
                onChange={(e) => {
                  setMaxLines(e.target.value);
                  setCriterionError(null);
                }}
                placeholder="500"
                aria-label="Max Code Lines"
                style={{ width: "100%", padding: "0.35rem 0.5rem", borderRadius: "4px", background: "rgba(0,0,0,0.3)", border: "1px solid rgba(255,255,255,0.15)", color: "#fff", fontSize: "0.8rem" }}
              />
            </div>
            <div>
              <label style={{ display: "block", fontSize: "0.7rem", color: "var(--text-muted, #94a3b8)", marginBottom: "0.2rem" }}>
                File Globs (comma- or space-separated) *
              </label>
              <input
                type="text"
                value={globs}
                onChange={(e) => {
                  setGlobs(e.target.value);
                  setCriterionError(null);
                }}
                placeholder="src/**/*.ts, src/**/*.tsx"
                aria-label="Max Lines Globs"
                style={{ width: "100%", padding: "0.35rem 0.5rem", borderRadius: "4px", background: "rgba(0,0,0,0.3)", border: "1px solid rgba(255,255,255,0.15)", color: "#fff", fontSize: "0.8rem" }}
              />
            </div>
          </div>
        )}

        {newCriterionType === "tokens" && (
          <div style={{ display: "grid", gridTemplateColumns: "1fr", gap: "0.5rem", marginBottom: "0.5rem" }}>
            <div>
              <label style={{ display: "block", fontSize: "0.7rem", color: "var(--text-muted, #94a3b8)", marginBottom: "0.2rem" }}>
                Minimum Tokens Written (delta) *
              </label>
              <input
                type="number"
                min={1}
                value={minTokens}
                onChange={(e) => {
                  setMinTokens(e.target.value);
                  setCriterionError(null);
                }}
                placeholder="1000"
                aria-label="Min Tokens Written"
                style={{ width: "100%", padding: "0.35rem 0.5rem", borderRadius: "4px", background: "rgba(0,0,0,0.3)", border: "1px solid rgba(255,255,255,0.15)", color: "#fff", fontSize: "0.8rem" }}
              />
            </div>
          </div>
        )}

        {criterionError && (
          <div
            data-testid="gate-criterion-error"
            style={{ color: "#f87171", fontSize: "0.75rem", marginBottom: "0.4rem" }}
          >
            {criterionError}
          </div>
        )}

        <button
          type="button"
          onClick={handleAdd}
          disabled={!isRecurring}
          title={!isRecurring ? "Enable Repeat Task above to configure gates" : undefined}
          className="btn-secondary"
          style={{ padding: "0.35rem 0.8rem", fontSize: "0.8rem", marginTop: "0.2rem" }}
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
