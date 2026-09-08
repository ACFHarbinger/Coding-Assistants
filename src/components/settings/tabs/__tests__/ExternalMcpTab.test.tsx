import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import ExternalMcpTab from "../ExternalMcpTab";
import * as api from "../../api";
import type { ExternalMcpStatus } from "../../types";

vi.mock("../../api", () => ({
  getExternalMcpStatus: vi.fn(),
  setExternalMcpEnabled: vi.fn(),
  reapplyExternalMcp: vi.fn(),
}));

const mockStatus: ExternalMcpStatus = {
  workspace: "/test/workspace",
  servers: [
    {
      key: "perplexity",
      displayName: "Perplexity Official API",
      docsUrl: "https://docs.perplexity.ai",
      auth: { kind: "api_key", env_var: "PERPLEXITY_API_KEY" },
      authConfigured: false,
      launcherFound: true,
      launcherPath: "/usr/local/bin/npx",
      enabled: false,
    },
    {
      key: "perplexity-web",
      displayName: "Perplexity Subscription Web",
      docsUrl: "https://github.com/example/pwm",
      auth: { kind: "session_login", setup_cmd: "pwm login" },
      authConfigured: null,
      notes: "Quota-limited + ~30-day token expiry. Run pwm login to authenticate.",
      launcherFound: false,
      launcherPath: null,
      enabled: true,
    },
  ],
  writtenConfigs: [],
};

describe("ExternalMcpTab (#278)", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("prompts user to select a workspace if workspaceRoot is null", () => {
    render(<ExternalMcpTab workspaceRoot={null} busy={false} />);
    expect(screen.getByText(/Select an active workspace in Orchestrate/)).toBeInTheDocument();
  });

  it("loads and displays external MCP servers with status chips and auth hints", async () => {
    vi.mocked(api.getExternalMcpStatus).mockResolvedValue(mockStatus);

    render(<ExternalMcpTab workspaceRoot="/test/workspace" busy={false} />);

    await waitFor(() => {
      expect(screen.getByText("Perplexity Official API")).toBeInTheDocument();
      expect(screen.getByText("Perplexity Subscription Web")).toBeInTheDocument();
    });

    // Launcher status chips
    expect(screen.getByText("Launcher Installed")).toBeInTheDocument();
    expect(screen.getByText("Launcher Missing in App PATH")).toBeInTheDocument();

    // Auth status chips
    expect(screen.getByText("API Key Not In App Env")).toBeInTheDocument();
    expect(screen.getByText("Login Required")).toBeInTheDocument();
    expect(screen.queryByText("Session Active")).toBeNull();

    // Per-entry auth hint copy
    expect(screen.getByText(/Auth hint: export/)).toBeInTheDocument();
    expect(screen.getByText("PERPLEXITY_API_KEY")).toBeInTheDocument();
    expect(screen.getByText(/in the shell you start Claude Code \/ Gemini CLI from/)).toBeInTheDocument();

    expect(screen.getByText(/Auth hint: run/)).toBeInTheDocument();
    expect(screen.getByText("pwm login")).toBeInTheDocument();
    expect(screen.getByText(/~30-day expiry, quota-metered/)).toBeInTheDocument();
    expect(screen.getByText(/Quota-limited \+ ~30-day token expiry/)).toBeInTheDocument();

    // Security check: NEVER render secret/key input fields
    const inputs = screen.queryAllByRole("textbox");
    expect(inputs.length).toBe(0);
    expect(screen.queryByPlaceholderText(/api[_\s-]?key/i)).toBeNull();
  });

  it("renders Token File Present when session token file is detected on disk", async () => {
    const statusWithToken: ExternalMcpStatus = {
      ...mockStatus,
      servers: [
        mockStatus.servers[0],
        { ...mockStatus.servers[1], authConfigured: true },
      ],
    };
    vi.mocked(api.getExternalMcpStatus).mockResolvedValue(statusWithToken);

    render(<ExternalMcpTab workspaceRoot="/test/workspace" busy={false} />);

    await waitFor(() => {
      expect(screen.getByText("Token File Present")).toBeInTheDocument();
      // Ensure we do not overstate as "Session Active"
      expect(screen.queryByText("Session Active")).toBeNull();
    });
  });

  it("toggles server enabled state through setExternalMcpEnabled", async () => {
    vi.mocked(api.getExternalMcpStatus).mockResolvedValue(mockStatus);
    const updatedStatus: ExternalMcpStatus = {
      ...mockStatus,
      servers: [
        { ...mockStatus.servers[0], enabled: true },
        mockStatus.servers[1],
      ],
      writtenConfigs: ["/test/workspace/.mcp.json"],
    };
    vi.mocked(api.setExternalMcpEnabled).mockResolvedValue(updatedStatus);

    render(<ExternalMcpTab workspaceRoot="/test/workspace" busy={false} />);

    await waitFor(() => {
      expect(screen.getByText("Perplexity Official API")).toBeInTheDocument();
    });

    // Find the toggle switches
    const switches = screen.getAllByRole("switch");
    expect(switches.length).toBe(2);
    fireEvent.click(switches[0]);

    await waitFor(() => {
      expect(api.setExternalMcpEnabled).toHaveBeenCalledWith(
        "/test/workspace",
        "perplexity",
        true,
      );
    });

    expect(screen.getByText(/Updated MCP configs:/)).toBeInTheDocument();
  });

  it("calls reapplyExternalMcp when clicking Re-apply to Configs", async () => {
    vi.mocked(api.getExternalMcpStatus).mockResolvedValue(mockStatus);
    vi.mocked(api.reapplyExternalMcp).mockResolvedValue({
      ...mockStatus,
      writtenConfigs: ["/test/workspace/.mcp.json", "/test/workspace/.gemini.json"],
    });

    render(<ExternalMcpTab workspaceRoot="/test/workspace" busy={false} />);

    await waitFor(() => {
      expect(screen.getByText("Re-apply to Configs")).toBeInTheDocument();
    });

    fireEvent.click(screen.getByText("Re-apply to Configs"));

    await waitFor(() => {
      expect(api.reapplyExternalMcp).toHaveBeenCalledWith("/test/workspace");
      expect(screen.getByText(/Re-applied MCP configs:/)).toBeInTheDocument();
    });
  });
});
