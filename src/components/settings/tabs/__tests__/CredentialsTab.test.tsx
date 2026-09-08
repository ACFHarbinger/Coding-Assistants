import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import CredentialsTab from "../CredentialsTab";
import * as api from "../../api";
import type { FieldSpec, SecretStatus } from "../../types";

vi.mock("../../api", () => ({
  clearCredential: vi.fn(),
  getCredentialStatus: vi.fn(),
  listCredentialFields: vi.fn(),
  setCredential: vi.fn(),
  listLinkedAccounts: vi.fn(),
  linkAccountCli: vi.fn(),
  unlinkAccount: vi.fn(),
}));

const field: FieldSpec = {
  id: "provider.deepseek.api_key",
  displayName: "DeepSeek API Key",
  ownerKind: "provider",
  ownerKey: "deepseek",
  envVar: "DEEPSEEK_API_KEY",
  secret: true,
  scope: "global",
  docsUrl: null,
  notes: null,
};

const unsetStatus: SecretStatus = {
  key: "DEEPSEEK_API_KEY",
  source: "none",
  isSet: false,
  updatedAt: null,
};

describe("CredentialsTab (#284)", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(api.listCredentialFields).mockResolvedValue([field]);
    vi.mocked(api.getCredentialStatus).mockResolvedValue(unsetStatus);
    vi.mocked(api.listLinkedAccounts).mockResolvedValue([
      {
        provider: "openai",
        externalLabel: null,
        connectionKind: "oauth_device",
        isLinked: false,
        linkedAt: null,
        source: "none",
      },
      {
        provider: "anthropic",
        externalLabel: "claude-user@example.com",
        connectionKind: "vendor_cli_login",
        isLinked: true,
        linkedAt: 1_725_000_000,
        source: "vendor_cli",
      },
      {
        provider: "google",
        externalLabel: null,
        connectionKind: "vendor_cli_login",
        isLinked: false,
        linkedAt: null,
        source: "none",
      },
    ]);
  });

  it("uses a write-only password input and clears its draft after saving", async () => {
    vi.mocked(api.setCredential).mockResolvedValue({
      ...unsetStatus,
      source: "keychain",
      isSet: true,
      updatedAt: 1_725_000_000,
    });
    const { container } = render(<CredentialsTab />);

    const input = await screen.findByPlaceholderText("Paste credential…");
    expect(input).toHaveAttribute("type", "password");
    fireEvent.change(input, { target: { value: "secret-for-test-only" } });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => {
      expect(api.setCredential).toHaveBeenCalledWith(field.id, "secret-for-test-only");
      expect(input).toHaveValue("");
    });
    expect(container.textContent).not.toContain("secret-for-test-only");
  });

  it("renders connected external accounts and supports connect/disconnect actions (#286)", async () => {
    vi.mocked(api.unlinkAccount).mockResolvedValue(true);
    vi.mocked(api.linkAccountCli).mockResolvedValue({
      provider: "openai",
      externalLabel: "openai-user@example.com",
      connectionKind: "vendor_cli_login",
      isLinked: true,
      linkedAt: 1_725_000_100,
      source: "vendor_cli",
    });

    render(<CredentialsTab />);

    // Renders the required account connection rows
    expect(await screen.findByText("ChatGPT / OpenAI")).toBeInTheDocument();
    expect(screen.getByText("Claude (Anthropic)")).toBeInTheDocument();
    expect(screen.getByText("Google / Gemini")).toBeInTheDocument();

    // Anthropic is connected
    expect(screen.getByText("claude-user@example.com")).toBeInTheDocument();
    const disconnectBtn = screen.getByRole("button", { name: "Disconnect" });
    fireEvent.click(disconnectBtn);
    await waitFor(() => {
      expect(api.unlinkAccount).toHaveBeenCalledWith("anthropic");
    });

    // OpenAI is not connected, click Connect to open input and save link
    const connectBtns = screen.getAllByRole("button", { name: "Connect" });
    fireEvent.click(connectBtns[0]);

    const labelInput = screen.getByPlaceholderText("e.g. user@openai.com");
    fireEvent.change(labelInput, { target: { value: "openai-user@example.com" } });
    fireEvent.click(screen.getByRole("button", { name: "Save Link" }));

    await waitFor(() => {
      expect(api.linkAccountCli).toHaveBeenCalledWith("openai", "openai-user@example.com");
    });
  });
});
