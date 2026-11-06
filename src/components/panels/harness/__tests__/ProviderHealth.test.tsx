import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { evaluateProviderHealth, type ProviderHealth } from "../healthTypes";
import { indexHealthList } from "../useProviderHealth";
import ProviderHealthChip, { ProviderHealthDot } from "../ProviderHealthChip";

describe("Provider Health Evaluation (evaluateProviderHealth)", () => {
  it("degrades gracefully to unknown when health is null or undefined", () => {
    const resNull = evaluateProviderHealth(null);
    expect(resNull.tone).toBe("unknown");
    expect(resNull.label).toBe("unknown");
    expect(resNull.dotColor).toBe("#64748b");

    const resUndef = evaluateProviderHealth(undefined);
    expect(resUndef.tone).toBe("unknown");
    expect(resUndef.dotColor).toBe("#64748b");
  });

  it("marks as bad when not installed", () => {
    const health: ProviderHealth = {
      agentId: "claude",
      provider: "anthropic",
      harnessTitle: "Claude",
      installed: false,
      detail: "claude not found on PATH",
      checkedAt: "2026-09-08T12:00:00Z",
    };
    const res = evaluateProviderHealth(health);
    expect(res.tone).toBe("bad");
    expect(res.label).toBe("not installed");
    expect(res.dotColor).toBe("#ef4444");
  });

  it("marks as warn when installed but authenticated is false", () => {
    const health: ProviderHealth = {
      agentId: "muse",
      provider: "muse",
      harnessTitle: "Muse",
      installed: true,
      authenticated: false,
      detail: "MODEL_API_KEY not set",
      checkedAt: "2026-09-08T12:00:00Z",
    };
    const res = evaluateProviderHealth(health);
    expect(res.tone).toBe("warn");
    expect(res.label).toBe("needs login");
    expect(res.dotColor).toBe("#f59e0b");
  });

  it("marks as warn when auth is expiring soon (< 48h)", () => {
    const futureDate = new Date(Date.now() + 12 * 3600 * 1000).toISOString();
    const health: ProviderHealth = {
      agentId: "cursor",
      provider: "cursor",
      harnessTitle: "Cursor",
      installed: true,
      authenticated: true,
      authExpiresAt: futureDate,
      detail: "Token expires in 12h",
      checkedAt: "2026-09-08T12:00:00Z",
    };
    const res = evaluateProviderHealth(health);
    expect(res.tone).toBe("warn");
    expect(res.label).toMatch(/^expires in \d+h$/);
    expect(res.dotColor).toBe("#f59e0b");
  });

  it("marks as warn when auth is expired", () => {
    const pastDate = new Date(Date.now() - 3600 * 1000).toISOString();
    const health: ProviderHealth = {
      agentId: "cursor",
      provider: "cursor",
      harnessTitle: "Cursor",
      installed: true,
      authenticated: true,
      authExpiresAt: pastDate,
      detail: "Token expired",
      checkedAt: "2026-09-08T12:00:00Z",
    };
    const res = evaluateProviderHealth(health);
    expect(res.tone).toBe("warn");
    expect(res.label).toBe("auth expired");
    expect(res.dotColor).toBe("#f59e0b");
  });

  it("marks as ready when installed and authenticated", () => {
    const health: ProviderHealth = {
      agentId: "grok",
      provider: "xai",
      harnessTitle: "Grok",
      installed: true,
      authenticated: true,
      detail: "grok 1.0.0 on PATH",
      checkedAt: "2026-09-08T12:00:00Z",
    };
    const res = evaluateProviderHealth(health);
    expect(res.tone).toBe("ok");
    expect(res.label).toBe("ready");
    expect(res.dotColor).toBe("#22c55e");
  });
});

describe("Health List Indexing (indexHealthList)", () => {
  it("indexes by agentId and creates bidirectional aliases", () => {
    const list: ProviderHealth[] = [
      {
        agentId: "chat",
        provider: "openai",
        harnessTitle: "Codex",
        installed: true,
        authenticated: true,
        detail: "ok",
        checkedAt: "2026-09-08T12:00:00Z",
      },
      {
        agentId: "gemini",
        provider: "google",
        harnessTitle: "Gemini",
        installed: true,
        authenticated: true,
        detail: "ok",
        checkedAt: "2026-09-08T12:00:00Z",
      },
    ];

    const map = indexHealthList(list);
    expect(map["chat"]).toBeDefined();
    expect(map["codex"]).toBeDefined();
    expect(map["chat"]).toBe(map["codex"]);

    expect(map["gemini"]).toBeDefined();
    expect(map["agy"]).toBeDefined();
    expect(map["gemini"]).toBe(map["agy"]);
  });
});

describe("ProviderHealthChip & ProviderHealthDot components", () => {
  it("renders ProviderHealthChip with proper labels and tooltips", () => {
    const health: ProviderHealth = {
      agentId: "claude",
      provider: "anthropic",
      harnessTitle: "Claude Code",
      installed: true,
      authenticated: true,
      detail: "claude 1.0.0",
      checkedAt: "2026-09-08T12:00:00Z",
    };

    render(<ProviderHealthChip providerId="claude" health={health} />);
    expect(screen.getByText("Claude Code")).toBeInTheDocument();
    expect(screen.getByText("ready")).toBeInTheDocument();
  });

  it("renders ProviderHealthDot with neutral tone when health is missing", () => {
    const { container } = render(<ProviderHealthDot health={null} titlePrefix="Grok" />);
    const dot = container.querySelector("span");
    expect(dot).toBeInTheDocument();
    expect(dot?.getAttribute("title")).toContain("unknown");
    expect(dot?.style.backgroundColor).toBe("rgb(100, 116, 139)");
  });
});
