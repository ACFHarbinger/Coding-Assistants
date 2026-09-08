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
});
