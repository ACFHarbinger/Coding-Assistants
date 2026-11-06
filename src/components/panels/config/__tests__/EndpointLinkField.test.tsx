import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor, act } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { EndpointLinkField } from "../EndpointLinkField";
import type { ModelConfig } from "../types";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const mockedInvoke = vi.mocked(invoke);

beforeEach(() => {
  (window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ = {};
  mockedInvoke.mockReset();
  mockedInvoke.mockImplementation(async (command: string) => {
    if (command === "endpoint_credential_status") return { is_set: false };
    if (command === "endpoint_credential_clear") return { is_set: false };
    throw new Error(`unexpected invoke ${command}`);
  });
});

function renderField(config: ModelConfig, onConfigChange = vi.fn()) {
  render(<EndpointLinkField index={0} config={config} onConfigChange={onConfigChange} />);
  return onConfigChange;
}

describe("EndpointLinkField", () => {
  it("connects through the endpoint command and does not spawn a process", async () => {
    mockedInvoke.mockImplementation(async (command: string) => {
      if (command === "endpoint_credential_status") return { is_set: false };
      if (command === "endpoint_link_connect") {
        return {
          endpoint: "http://127.0.0.1:1234/v1",
          state: "connected",
          owns_process: false,
        };
      }
      throw new Error(`unexpected invoke ${command}`);
    });
    const onConfigChange = renderField({ provider: "ollama", model: "llama3" });

    fireEvent.change(screen.getByLabelText("Existing process endpoint"), {
      target: { value: "http://127.0.0.1:1234/v1" },
    });
    expect(onConfigChange).toHaveBeenCalledWith(0, "endpoint", "http://127.0.0.1:1234/v1");
    expect(onConfigChange).toHaveBeenCalledWith(0, "endpoint_state", "disconnected");

    fireEvent.click(screen.getByRole("button", { name: "Connect" }));
    await waitFor(() => {
      expect(onConfigChange).toHaveBeenCalledWith(0, "endpoint_state", "connected");
    });
    expect(mockedInvoke).toHaveBeenCalledWith("endpoint_credential_status", {
      endpoint: "http://127.0.0.1:1234/v1",
    });
    expect(mockedInvoke).toHaveBeenCalledWith("endpoint_link_connect", {
      endpoint: "http://127.0.0.1:1234/v1",
    });
    expect(onConfigChange).toHaveBeenCalledWith(0, "endpoint", "http://127.0.0.1:1234/v1");
  });

  it("disconnect keeps the address and refuses a link that owns a process", async () => {
    mockedInvoke.mockImplementation(async (command: string) => {
      if (command === "endpoint_credential_status") return { is_set: false };
      if (command === "endpoint_link_disconnect") {
        return {
          endpoint: "http://127.0.0.1:1234/v1",
          state: "disconnected",
          owns_process: false,
        };
      }
      if (command === "endpoint_link_connect") {
        return {
          endpoint: "http://127.0.0.1:1234/v1",
          state: "connected",
          owns_process: true,
        };
      }
      throw new Error(`unexpected invoke ${command}`);
    });
    const onConfigChange = renderField({
      provider: "ollama",
      model: "llama3",
      endpoint: "http://127.0.0.1:1234/v1",
      endpoint_state: "connected",
    });

    fireEvent.click(screen.getByRole("button", { name: "Disconnect" }));
    await waitFor(() => {
      expect(onConfigChange).toHaveBeenCalledWith(0, "endpoint_state", "disconnected");
    });
    expect(mockedInvoke).toHaveBeenCalledWith("endpoint_link_disconnect", {
      endpoint: "http://127.0.0.1:1234/v1",
    });
    expect(onConfigChange).toHaveBeenCalledWith(0, "endpoint", "http://127.0.0.1:1234/v1");

    onConfigChange.mockClear();
    fireEvent.click(screen.getByRole("button", { name: "Connect" }));
    await waitFor(() => {
      expect(screen.getByText(/does not take ownership/)).toBeInTheDocument();
    });
    expect(onConfigChange).not.toHaveBeenCalled();
  });

  it.each([
    ["Connect", "success"],
    ["Disconnect", "success"],
    ["Connect", "failure"],
    ["Disconnect", "failure"],
  ] as const)("locks endpoint edits during %s and unlocks after %s", async (action, outcome) => {
    const endpoint = "http://127.0.0.1:1234/v1";
    const replacement = "http://127.0.0.1:5678/v1";
    type Link = { endpoint: string; state: "connected" | "disconnected"; owns_process: boolean };
    let resolve!: (link: Link) => void;
    let reject!: (error: Error) => void;
    mockedInvoke.mockImplementation((command: string) => {
      if (command === "endpoint_credential_status") return Promise.resolve({ is_set: false });
      if (command === "endpoint_link_connect" || command === "endpoint_link_disconnect") {
        return new Promise<Link>((accept, fail) => {
          resolve = accept;
          reject = fail;
        });
      }
      return Promise.reject(new Error(`unexpected invoke ${command}`));
    });
    const onConfigChange = renderField({
      provider: "ollama",
      model: "llama3",
      endpoint,
      endpoint_state: action === "Connect" ? "disconnected" : "connected",
    });
    const input = screen.getByLabelText("Existing process endpoint");
    await act(async () => {
      await Promise.resolve();
    });
    fireEvent.click(screen.getByRole("button", { name: action }));
    expect(input).toBeDisabled();
    expect(screen.getByRole("button", { name: "Connect" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Disconnect" })).toBeDisabled();
    // Even a late synthetic change event cannot save a different endpoint.
    fireEvent.change(input, { target: { value: replacement } });
    expect(input).toHaveValue(endpoint);
    expect(onConfigChange).not.toHaveBeenCalled();

    await act(async () => {
      if (outcome === "success") {
        resolve({ endpoint, state: action === "Connect" ? "connected" : "disconnected", owns_process: false });
      } else {
        reject(new Error("Endpoint command failed"));
      }
    });
    await waitFor(() => expect(input).toBeEnabled());
    if (outcome === "failure") {
      expect(screen.getByText("Endpoint command failed")).toBeInTheDocument();
      expect(onConfigChange).not.toHaveBeenCalled();
    } else {
      expect(onConfigChange).toHaveBeenCalledWith(0, "endpoint", endpoint);
      expect(onConfigChange).toHaveBeenCalledWith(0, "endpoint_state", action === "Connect" ? "connected" : "disconnected");
    }
    onConfigChange.mockClear();
    fireEvent.change(input, { target: { value: replacement } });
    expect(input).toHaveValue(replacement);
    expect(onConfigChange).toHaveBeenCalledWith(0, "endpoint", replacement);
    expect(onConfigChange).toHaveBeenCalledWith(0, "endpoint_state", "disconnected");
  });

  it("disables Check until the role is connected", () => {
    renderField({
      provider: "ollama",
      model: "llama3",
      endpoint: "http://127.0.0.1:1234/v1",
      endpoint_state: "disconnected",
    });
    expect(screen.getByRole("button", { name: "Check" })).toBeDisabled();
  });

  it("shows a reachable health result", async () => {
    mockedInvoke.mockImplementation(async (command: string) => {
      if (command === "endpoint_credential_status") return { is_set: false };
      if (command === "endpoint_link_health") {
        return { kind: "reachable", status: 200, message: "Reachable." };
      }
      throw new Error(`unexpected invoke ${command}`);
    });
    renderField({
      provider: "ollama",
      model: "llama3",
      endpoint: "http://127.0.0.1:1234/v1",
      endpoint_state: "connected",
    });

    fireEvent.click(screen.getByRole("button", { name: "Check" }));
    await waitFor(() => {
      expect(screen.getByText("Reachable.")).toBeInTheDocument();
    });
    expect(mockedInvoke).toHaveBeenCalledWith("endpoint_link_health", {
      endpoint: "http://127.0.0.1:1234/v1",
      endpointState: "connected",
    });
  });

  it("disables Check while the probe is pending", async () => {
    let finish: (value: unknown) => void = () => {};
    mockedInvoke.mockReturnValue(new Promise((resolve) => {
      finish = resolve;
    }));
    renderField({
      provider: "ollama",
      model: "llama3",
      endpoint: "http://127.0.0.1:1234/v1",
      endpoint_state: "connected",
    });

    fireEvent.click(screen.getByRole("button", { name: "Check" }));
    const pending = await screen.findByRole("button", { name: "Checking…" });
    expect(pending).toBeDisabled();
    const input = screen.getByLabelText("Existing process endpoint");
    expect(input).toBeDisabled();
    fireEvent.change(input, { target: { value: "http://127.0.0.1:5678/v1" } });
    expect(input).toHaveValue("http://127.0.0.1:1234/v1");
    finish({ kind: "reachable", status: 200, message: "Reachable.", is_set: false });
    await waitFor(() => {
      expect(screen.getByRole("button", { name: "Check" })).toBeEnabled();
      expect(input).toBeEnabled();
    });
  });

  it("shows a failed health invoke as the error string", async () => {
    mockedInvoke.mockImplementation(async (command: string) => {
      if (command === "endpoint_credential_status") return { is_set: false };
      if (command === "endpoint_link_health") throw new Error("probe failed");
      throw new Error(`unexpected invoke ${command}`);
    });
    renderField({
      provider: "ollama",
      model: "llama3",
      endpoint: "http://127.0.0.1:1234/v1",
      endpoint_state: "connected",
    });
    fireEvent.click(screen.getByRole("button", { name: "Check" }));
    await waitFor(() => {
      expect(screen.getByText("probe failed")).toBeInTheDocument();
    });
  });

  it("saves a key for the address and does not store it on the role", async () => {
    const onConfigChange = vi.fn();
    mockedInvoke.mockImplementation(async (command: string, args?: unknown) => {
      if (command === "endpoint_credential_status") return { is_set: false };
      if (command === "endpoint_credential_set") {
        expect(args).toEqual({
          endpoint: "http://127.0.0.1:1234/v1",
          apiKey: "secret-for-this-address",
        });
        return { is_set: true };
      }
      throw new Error(`unexpected invoke ${command}`);
    });
    renderField(
      {
        provider: "ollama",
        model: "llama3",
        endpoint: "http://127.0.0.1:1234/v1",
        endpoint_state: "connected",
      },
      onConfigChange,
    );
    const key = screen.getByLabelText("Endpoint API key");
    fireEvent.change(key, { target: { value: "secret-for-this-address" } });
    fireEvent.click(screen.getByRole("button", { name: "Save key" }));
    await waitFor(() => {
      expect(screen.getByText(/A key is saved for this address/)).toBeInTheDocument();
    });
    expect(key).toHaveValue("");
    expect(onConfigChange).not.toHaveBeenCalled();
    expect(JSON.stringify(onConfigChange.mock.calls)).not.toContain("secret-for-this-address");
  });
});
