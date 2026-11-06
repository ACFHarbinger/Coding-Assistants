import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { GateCriteriaSection, parseArgv } from "../GateCriteriaSection";
import { ScheduleTaskBuilder } from "../ScheduleTaskBuilder";
import type { GateCriterion, ScheduleCreateArgs } from "../types";

describe("parseArgv", () => {
  it("preserves adjacent quoted fragments and rejects unfinished quotes", () => {
    expect(parseArgv('--message="hello world" ""')).toEqual(['--message=hello world', '']);
    expect(() => parseArgv('"unfinished')).toThrow();
  });
  it("parses empty string to empty array", () => {
    expect(parseArgv("")).toEqual([]);
    expect(parseArgv("   ")).toEqual([]);
  });

  it("splits arguments by whitespace", () => {
    expect(parseArgv("test --lib --nocapture")).toEqual(["test", "--lib", "--nocapture"]);
  });

  it("preserves double and single quoted strings containing spaces", () => {
    expect(parseArgv('run --message "hello world" --flag')).toEqual([
      "run",
      "--message",
      "hello world",
      "--flag",
    ]);
    expect(parseArgv("run --name 'audit pass'")).toEqual(["run", "--name", "audit pass"]);
  });
});

describe("GateCriteriaSection", () => {
  it("rejects fractional thresholds instead of silently truncating them", () => {
    const onAdd = vi.fn();
    render(<GateCriteriaSection gateCriteria={[]} gateCombinator="all" isRecurring={true}
      onChangeCombinator={vi.fn()} onAddCriterion={onAdd} onRemoveCriterion={vi.fn()} />);
    fireEvent.change(screen.getByLabelText("Command Check Type"), { target: { value: "at_most" } });
    fireEvent.change(screen.getByLabelText("Maximum Stdout Value"), { target: { value: "2.5" } });
    fireEvent.click(screen.getByText("+ Add Criterion"));
    expect(onAdd).not.toHaveBeenCalled();
  });
  it("renders one-shot banner and disables adding gates when isRecurring is false", () => {
    const onAdd = vi.fn();
    render(
      <GateCriteriaSection
        gateCriteria={[]}
        gateCombinator="all"
        isRecurring={false}
        onChangeCombinator={vi.fn()}
        onAddCriterion={onAdd}
        onRemoveCriterion={vi.fn()}
      />
    );

    expect(screen.getByTestId("one-shot-gate-note")).toBeInTheDocument();
    expect(
      screen.getByText(/One-shot tasks do not evaluate gates/i)
    ).toBeInTheDocument();

    const addBtn = screen.getByText("+ Add Criterion");
    expect(addBtn).toBeDisabled();

    fireEvent.click(addBtn);
    expect(onAdd).not.toHaveBeenCalled();
  });

  it("allows configuring and adding custom command criteria with explicit argv and check", () => {
    const onAdd = vi.fn();
    render(
      <GateCriteriaSection
        gateCriteria={[]}
        gateCombinator="all"
        isRecurring={true}
        onChangeCombinator={vi.fn()}
        onAddCriterion={onAdd}
        onRemoveCriterion={vi.fn()}
      />
    );

    // Edit program & argv
    const progInput = screen.getByLabelText("Command Program");
    fireEvent.change(progInput, { target: { value: "pytest" } });

    const argsInput = screen.getByLabelText("Command Arguments");
    fireEvent.change(argsInput, { target: { value: "-v --tb=short tests/unit" } });

    // Select exit check: at most
    const checkSelect = screen.getByLabelText("Command Check Type");
    fireEvent.change(checkSelect, { target: { value: "at_most" } });

    const maxInput = screen.getByLabelText("Maximum Stdout Value");
    fireEvent.change(maxInput, { target: { value: "2" } });

    // Click Add
    const addBtn = screen.getByText("+ Add Criterion");
    expect(addBtn).not.toBeDisabled();
    fireEvent.click(addBtn);

    expect(onAdd).toHaveBeenCalledWith({
      kind: "command",
      program: "pytest",
      args: ["-v", "--tb=short", "tests/unit"],
      check: { at_most: { max: 2 } },
    });
  });

  it("validates empty command program", () => {
    const onAdd = vi.fn();
    render(
      <GateCriteriaSection
        gateCriteria={[]}
        gateCombinator="all"
        isRecurring={true}
        onChangeCombinator={vi.fn()}
        onAddCriterion={onAdd}
        onRemoveCriterion={vi.fn()}
      />
    );

    const progInput = screen.getByLabelText("Command Program");
    fireEvent.change(progInput, { target: { value: "   " } });

    fireEvent.click(screen.getByText("+ Add Criterion"));

    expect(screen.getByTestId("gate-criterion-error")).toHaveTextContent(
      "Program name is required"
    );
    expect(onAdd).not.toHaveBeenCalled();
  });

  it("allows configuring and adding max_lines criterion with numeric validation", () => {
    const onAdd = vi.fn();
    render(
      <GateCriteriaSection
        gateCriteria={[]}
        gateCombinator="all"
        isRecurring={true}
        onChangeCombinator={vi.fn()}
        onAddCriterion={onAdd}
        onRemoveCriterion={vi.fn()}
      />
    );

    // Select max_lines
    fireEvent.change(screen.getByLabelText("Criterion Type"), {
      target: { value: "max_lines" },
    });

    const linesInput = screen.getByLabelText("Max Code Lines");
    fireEvent.change(linesInput, { target: { value: "250" } });

    const globsInput = screen.getByLabelText("Max Lines Globs");
    fireEvent.change(globsInput, { target: { value: "src/**/*.ts, src/**/*.tsx" } });

    fireEvent.click(screen.getByText("+ Add Criterion"));

    expect(onAdd).toHaveBeenCalledWith({
      kind: "max_lines_per_file",
      max_lines: 250,
      globs: ["src/**/*.ts", "src/**/*.tsx"],
    });
  });

  it("allows configuring min_tokens criterion with numeric validation", () => {
    const onAdd = vi.fn();
    render(
      <GateCriteriaSection
        gateCriteria={[]}
        gateCombinator="all"
        isRecurring={true}
        onChangeCombinator={vi.fn()}
        onAddCriterion={onAdd}
        onRemoveCriterion={vi.fn()}
      />
    );

    fireEvent.change(screen.getByLabelText("Criterion Type"), {
      target: { value: "tokens" },
    });

    const tokensInput = screen.getByLabelText("Min Tokens Written");
    fireEvent.change(tokensInput, { target: { value: "3500" } });

    fireEvent.click(screen.getByText("+ Add Criterion"));

    expect(onAdd).toHaveBeenCalledWith({
      kind: "min_tokens_written",
      min_tokens: 3500,
    });
  });

  it("renders existing criteria and removes on click", () => {
    const onRemove = vi.fn();
    const criteria: GateCriterion[] = [
      {
        kind: "command",
        program: "cargo",
        args: ["clippy"],
        check: "exit_code_zero",
      },
      {
        kind: "max_lines_per_file",
        max_lines: 400,
        globs: ["src/**/*.rs"],
      },
    ];

    render(
      <GateCriteriaSection
        gateCriteria={criteria}
        gateCombinator="any"
        isRecurring={true}
        onChangeCombinator={vi.fn()}
        onAddCriterion={vi.fn()}
        onRemoveCriterion={onRemove}
      />
    );

    expect(screen.getByText(/cargo clippy/)).toBeInTheDocument();
    expect(screen.getByText(/≤ 400 lines on/)).toBeInTheDocument();

    const removeButtons = screen.getAllByText("Remove");
    expect(removeButtons).toHaveLength(2);
    fireEvent.click(removeButtons[0]);
    expect(onRemove).toHaveBeenCalledWith(0);
  });
});

describe("ScheduleTaskBuilder payload submission", () => {
  it("submits configured recurring task with gates payload", async () => {
    const onSubmit = vi.fn().mockResolvedValue(undefined);

    render(
      <ScheduleTaskBuilder
        workspace="/repo/test"
        availableAgents={["claude", "gemini"]}
        onSubmit={onSubmit}
        onCancel={vi.fn()}
        submitting={false}
      />
    );

    fireEvent.change(screen.getByPlaceholderText(/Nightly test & hygiene pass/i), {
      target: { value: "Nightly Refactor" },
    });
    fireEvent.change(
      screen.getByPlaceholderText(/Detailed instruction dispatched/i),
      { target: { value: "Audit file sizes" } }
    );

    // Enable recurring
    const repeatCheckbox = screen.getByLabelText(/Repeat Task/i);
    fireEvent.click(repeatCheckbox);

    // Add a custom command criterion
    const progInput = screen.getByLabelText("Command Program");
    fireEvent.change(progInput, { target: { value: "cargo" } });
    const argsInput = screen.getByLabelText("Command Arguments");
    fireEvent.change(argsInput, { target: { value: "test --lib" } });
    fireEvent.click(screen.getByText("+ Add Criterion"));

    // Submit form
    fireEvent.click(screen.getByText("Schedule Goal"));

    await waitFor(() => {
      expect(onSubmit).toHaveBeenCalledTimes(1);
    });

    const submitted: ScheduleCreateArgs = onSubmit.mock.calls[0][0];
    expect(submitted.name).toBe("Nightly Refactor");
    expect(submitted.instruction).toBe("Audit file sizes");
    expect(submitted.repeat_interval_secs).toBe(3600);
    expect(submitted.gate_criteria).toHaveLength(1);
    expect(submitted.gate_criteria[0]).toEqual({
      kind: "command",
      program: "cargo",
      args: ["test", "--lib"],
      check: "exit_code_zero",
    });
  });

  it("submits one-shot task with empty gate_criteria even if builder had criteria", async () => {
    const onSubmit = vi.fn().mockResolvedValue(undefined);

    render(
      <ScheduleTaskBuilder
        workspace="/repo/test"
        availableAgents={["claude", "gemini"]}
        onSubmit={onSubmit}
        onCancel={vi.fn()}
        submitting={false}
      />
    );

    fireEvent.change(screen.getByPlaceholderText(/Nightly test & hygiene pass/i), {
      target: { value: "Single Audit Run" },
    });
    fireEvent.change(
      screen.getByPlaceholderText(/Detailed instruction dispatched/i),
      { target: { value: "Run one-off inspection" } }
    );

    // In one-shot mode (Repeat Task unchecked)
    expect(screen.getByTestId("one-shot-gate-note")).toBeInTheDocument();

    fireEvent.click(screen.getByText("Schedule Goal"));

    await waitFor(() => {
      expect(onSubmit).toHaveBeenCalledTimes(1);
    });

    const submitted: ScheduleCreateArgs = onSubmit.mock.calls[0][0];
    expect(submitted.name).toBe("Single Audit Run");
    expect(submitted.repeat_interval_secs).toBeNull();
    expect(submitted.gate_criteria).toEqual([]);
  });
});
