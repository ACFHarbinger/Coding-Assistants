import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { EndpointLinkField } from "../EndpointLinkField";
import type { ModelConfig } from "../types";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const mockedInvoke = vi.mocked(invoke);

beforeEach(() => {
  (window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ = {};
  mockedInvoke.mockReset();
});

function renderField(config: ModelConfig, onConfigChange = vi.fn()) {
  render(<EndpointLinkField index={0} config={config} onConfigChange={onConfigChange} />);
  return onConfigChange;
}

describe("EndpointLinkField", () => {
  it("connects through the endpoint command and does not spawn a process", async () => {
    mockedInvoke.mockResolvedValue({
      endpoint: "http://127.0.0.1:1234/v1",
      state: "connected",
      owns_process: false,
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
    expect(mockedInvoke).toHaveBeenCalledTimes(1);
    expect(mockedInvoke).toHaveBeenCalledWith("endpoint_link_connect", {
      endpoint: "http://127.0.0.1:1234/v1",
    });
    expect(onConfigChange).toHaveBeenCalledWith(0, "endpoint", "http://127.0.0.1:1234/v1");
  });

  it("disconnect keeps the address and refuses a link that owns a process", async () => {
    mockedInvoke.mockResolvedValueOnce({
      endpoint: "http://127.0.0.1:1234/v1",
      state: "disconnected",
      owns_process: false,
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
    mockedInvoke.mockResolvedValueOnce({
      endpoint: "http://127.0.0.1:1234/v1",
      state: "connected",
      owns_process: true,
    });
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
    mockedInvoke.mockReturnValueOnce(new Promise<Link>((accept, fail) => {
      resolve = accept;
      reject = fail;
    }));
    const onConfigChange = renderField({
      provider: "ollama",
      model: "llama3",
      endpoint,
      endpoint_state: action === "Connect" ? "disconnected" : "connected",
    });
    const input = screen.getByLabelText("Existing process endpoint");
    fireEvent.click(screen.getByRole("button", { name: action }));
    expect(input).toBeDisabled();
    expect(screen.getByRole("button", { name: "Connect" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Disconnect" })).toBeDisabled();
    // Even a late synthetic change event cannot save a different endpoint.
    fireEvent.change(input, { target: { value: replacement } });
    expect(input).toHaveValue(endpoint);
    expect(onConfigChange).not.toHaveBeenCalled();

    if (outcome === "success") {
      resolve({ endpoint, state: action === "Connect" ? "connected" : "disconnected", owns_process: false });
    } else {
      reject(new Error("Endpoint command failed"));
    }
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

});
