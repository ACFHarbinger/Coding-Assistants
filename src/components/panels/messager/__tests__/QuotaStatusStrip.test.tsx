import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QuotaStatusStrip } from "../QuotaStatusStrip";
import { invoke } from "@tauri-apps/api/core";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

describe("QuotaStatusStrip", () => {
  beforeEach(() => {
    (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ = {};
    vi.clearAllMocks();
  });

  const mockSettings = {
    schema_version: 1,
    workspace: null,
    backup_retention: 7,
    backup_retention_status: "inherited",
    default_workspace: null,
    default_workspace_status: "inherited",
    default_session: null,
    default_session_status: "inherited",
    profiles: [],
    harnesses: [],
    orchestration: {
      confirm_new_enrollment: false,
      confirm_new_enrollment_status: "inherited",
      confirm_broadcast: false,
      confirm_broadcast_status: "inherited",
      auto_enrollment_allowed: true,
      auto_enrollment_allowed_status: "inherited",
      sandbox_strictness: "standard",
      sandbox_strictness_status: "inherited",
      retention_days: null,
      retention_days_status: "inherited",
      export_enabled: true,
      export_enabled_status: "inherited",
      memory_recall_enabled: false,
      memory_recall_enabled_status: "inherited",
      memory_recall_limit: 5,
      memory_recall_limit_status: "inherited",
      allow_metered_quota_probes: true,
      quota_auto_refresh_enabled: false,
      quota_auto_refresh_interval_secs: 300,
    },
  };

  const mockDeepseekQuota = {
    agent_id: "deepseek",
    provider: "deepseek",
    harness_title: "DeepSeek",
    status: "ok",
    windows: [],
    fetched_at: 1_725_000_000,
    balance: "Balance $12.34 USD — granted $5.00 + topped-up $7.34",
    balance_info: {
      currency: "USD",
      total: 12.34,
      paid: 7.34,
      gift: 5.0,
      topped_up: 7.34,
      granted: 5.0,
    },
  };

  const mockOpencodeQuota = {
    agent_id: "opencode",
    provider: "opencode",
    harness_title: "Anomaly Opencode",
    status: "ok",
    windows: [
      {
        label: "daily",
        used_percent: 25,
        remaining_percent: 75,
      },
    ],
    fetched_at: 1_725_000_000,
  };

  it("renders quotas with segmented paid/gift balance for DeepSeek and metered probes toggle", async () => {
    vi.mocked(invoke).mockImplementation((cmd: string, args?: any) => {
      if (cmd === "settings_get_effective") {
        return Promise.resolve(mockSettings);
      }
      if (cmd === "hub_get_provider_health") {
        return Promise.resolve([]);
      }
      if (cmd === "hub_refresh_provider_quota") {
        if (args?.agentId === "deepseek") return Promise.resolve(mockDeepseekQuota);
        if (args?.agentId === "opencode") return Promise.resolve(mockOpencodeQuota);
      }
      return Promise.resolve(null);
    });

    render(<QuotaStatusStrip />);

    await waitFor(() => {
      expect(screen.getByText(/provider usage/i)).toBeInTheDocument();
    });

    expect(screen.getByText("Metered: ON")).toBeInTheDocument();
    expect(screen.getByText("DeepSeek")).toBeInTheDocument();
    expect(screen.getByText("$12.34 USD")).toBeInTheDocument();
    expect(screen.getByText("Paid $7.34")).toBeInTheDocument();
    expect(screen.getByText("Gift $5.00")).toBeInTheDocument();
    expect(screen.getByText("Anomaly Opencode")).toBeInTheDocument();
    expect(screen.getByText("25% used")).toBeInTheDocument();
  });

  it("toggles metered probes setting when clicked", async () => {
    vi.mocked(invoke).mockImplementation((cmd: string, args?: any) => {
      if (cmd === "settings_get_effective") {
        return Promise.resolve(mockSettings);
      }
      if (cmd === "hub_get_provider_health") {
        return Promise.resolve([]);
      }
      if (cmd === "hub_refresh_provider_quota") {
        if (args?.agentId === "deepseek") return Promise.resolve(mockDeepseekQuota);
        if (args?.agentId === "opencode") return Promise.resolve(mockOpencodeQuota);
      }
      if (cmd === "settings_update_orchestration") {
        return Promise.resolve({
          ...mockSettings,
          orchestration: {
            ...mockSettings.orchestration,
            allow_metered_quota_probes: false,
          },
        });
      }
      return Promise.resolve(null);
    });

    render(<QuotaStatusStrip />);

    await waitFor(() => {
      expect(screen.getByText("Metered: ON")).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("button", { name: /metered/i }));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("settings_update_orchestration", {
        workspace: null,
        patch: { allow_metered_quota_probes: false },
      });
    });
  });
});
