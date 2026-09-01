import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { ModelSelect } from "../ModelSelect";
import type { RoleConfig, AgentResources } from "../types";

const mockResources: AgentResources = {
  prompts: [".agent/prompts/planner.md"],
  rules: [".agent/rules/standard.md"],
  workflows: [".agent/workflows/default.md"]
};

const mockProviders: Record<string, string> = {
  openai: "OpenAI",
  anthropic: "Anthropic",
  gemini: "Gemini",
  ollama: "Ollama (Local)"
};

describe("ModelSelect Component (#216 follow-up)", () => {
  it("renders available models for a configured provider", () => {
    const role: RoleConfig = {
      name: "Planner",
      config: {
        provider: "openai",
        model: "gpt-4o"
      }
    };

    const availableModels = {
      openai: ["gpt-4o", "gpt-4o-mini", "o1"],
      anthropic: ["claude-3-5-sonnet", "claude-3-haiku"]
    };

    const onProviderChange = vi.fn();
    const onConfigChange = vi.fn();
    const onNameChange = vi.fn();
    const onRemove = vi.fn();
    const onPreview = vi.fn();
    const onAddToTeam = vi.fn();
    const onRemoveFromTeam = vi.fn();

    render(
      <ModelSelect
        index={0}
        role={role}
        availableModels={availableModels}
        onProviderChange={onProviderChange}
        onConfigChange={onConfigChange}
        onNameChange={onNameChange}
        onRemove={onRemove}
        onPreview={onPreview}
        resources={mockResources}
        PROVIDERS={mockProviders}
        onAddToTeam={onAddToTeam}
        onRemoveFromTeam={onRemoveFromTeam}
        isOnTeam={false}
      />
    );

    expect(screen.getByDisplayValue("Planner")).toBeInTheDocument();
    expect(screen.getByDisplayValue("gpt-4o")).toBeInTheDocument();
    expect(screen.getByText("gpt-4o-mini")).toBeInTheDocument();
  });

  it("handles empty models edge without blank select or crash and shows validation hint", () => {
    const role: RoleConfig = {
      name: "Custom Agent",
      config: {
        provider: "ollama",
        model: ""
      }
    };

    const availableModels = {
      openai: ["gpt-4o"],
      ollama: [] // empty models list
    };

    const onProviderChange = vi.fn();
    const onConfigChange = vi.fn();
    const onNameChange = vi.fn();
    const onRemove = vi.fn();
    const onPreview = vi.fn();
    const onAddToTeam = vi.fn();
    const onRemoveFromTeam = vi.fn();

    render(
      <ModelSelect
        index={0}
        role={role}
        availableModels={availableModels}
        onProviderChange={onProviderChange}
        onConfigChange={onConfigChange}
        onNameChange={onNameChange}
        onRemove={onRemove}
        onPreview={onPreview}
        resources={mockResources}
        PROVIDERS={mockProviders}
        onAddToTeam={onAddToTeam}
        onRemoveFromTeam={onRemoveFromTeam}
        isOnTeam={false}
      />
    );

    expect(screen.getByText("No models available")).toBeInTheDocument();
    expect(
      screen.getByText(/No models discovered for this provider/i)
    ).toBeInTheDocument();
  });

  it("preserves custom selected model when not present in returned availableModels", () => {
    const role: RoleConfig = {
      name: "Developer",
      config: {
        provider: "gemini",
        model: "gemini-2.0-flash-custom"
      }
    };

    const availableModels = {
      gemini: ["gemini-1.5-pro", "gemini-1.5-flash"]
    };

    const onConfigChange = vi.fn();

    render(
      <ModelSelect
        index={0}
        role={role}
        availableModels={availableModels}
        onProviderChange={vi.fn()}
        onConfigChange={onConfigChange}
        onNameChange={vi.fn()}
        onRemove={vi.fn()}
        onPreview={vi.fn()}
        resources={mockResources}
        PROVIDERS={mockProviders}
        onAddToTeam={vi.fn()}
        onRemoveFromTeam={vi.fn()}
        isOnTeam={true}
      />
    );

    expect(screen.getByDisplayValue("gemini-2.0-flash-custom")).toBeInTheDocument();
    expect(screen.getByText("Remove from team")).toBeInTheDocument();
  });

  it("triggers callbacks on name edit, provider selection, and remove", () => {
    const role: RoleConfig = {
      name: "Reviewer",
      config: {
        provider: "anthropic",
        model: "claude-3-5-sonnet"
      }
    };

    const availableModels = {
      anthropic: ["claude-3-5-sonnet"]
    };

    const onProviderChange = vi.fn();
    const onNameChange = vi.fn();
    const onRemove = vi.fn();

    render(
      <ModelSelect
        index={1}
        role={role}
        availableModels={availableModels}
        onProviderChange={onProviderChange}
        onConfigChange={vi.fn()}
        onNameChange={onNameChange}
        onRemove={onRemove}
        onPreview={vi.fn()}
        resources={mockResources}
        PROVIDERS={mockProviders}
        onAddToTeam={vi.fn()}
        onRemoveFromTeam={vi.fn()}
        isOnTeam={false}
      />
    );

    const nameInput = screen.getByDisplayValue("Reviewer");
    fireEvent.change(nameInput, { target: { value: "Lead Reviewer" } });
    expect(onNameChange).toHaveBeenCalledWith(1, "Lead Reviewer");

    const providerSelect = screen.getAllByRole("combobox")[0];
    fireEvent.change(providerSelect, { target: { value: "gemini" } });
    expect(onProviderChange).toHaveBeenCalledWith(1, "gemini");

    const removeBtn = screen.getByRole("button", { name: "Remove" });
    fireEvent.click(removeBtn);
    expect(onRemove).toHaveBeenCalledWith(1);
  });
});
