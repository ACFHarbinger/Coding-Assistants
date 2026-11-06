import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import VTuberPresenceDock from "../VTuberPresenceDock";
import * as api from "../../../settings/api";

vi.mock("../../../settings/api", () => ({
  testVtuberBridge: vi.fn(),
}));

describe("VTuberPresenceDock (U17 / #307)", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders null when no agents have animated_avatar enabled", () => {
    const { container } = render(
      <VTuberPresenceDock
        hubAgents={[
          { id: "claude", display_name: "Claude", animated_avatar: false },
          { id: "human", display_name: "Human", animated_avatar: false },
        ]}
      />
    );

    expect(container.firstChild).toBeNull();
  });

  it("renders dock strip when an agent has animated_avatar enabled", () => {
    render(
      <VTuberPresenceDock
        hubAgents={[
          { id: "claude", display_name: "Claude", animated_avatar: true, vtuber_character: "hiyori" },
          { id: "human", display_name: "Human", animated_avatar: false },
        ]}
      />
    );

    expect(screen.getByTestId("vtuber-presence-dock")).toBeTruthy();
    expect(screen.getByText(/@claude/i)).toBeTruthy();
    expect(screen.getByText(/hiyori/i)).toBeTruthy();
  });

  it("expands and collapses the live Live2D iframe viewport", () => {
    render(
      <VTuberPresenceDock
        hubAgents={[
          { id: "gemini", display_name: "Gemini", animated_avatar: true },
        ]}
      />
    );

    const toggleBtn = screen.getByRole("button", { name: /show live2d view/i });
    fireEvent.click(toggleBtn);

    expect(screen.getByTitle("Open-LLM-VTuber Live2D Viewport")).toBeTruthy();
    expect(screen.getByRole("button", { name: /hide live2d view/i })).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: /hide live2d view/i }));
    expect(screen.queryByTitle("Open-LLM-VTuber Live2D Viewport")).toBeNull();
  });

  it("checks bridge connection and updates status display", async () => {
    vi.mocked(api.testVtuberBridge).mockResolvedValue({
      available: true,
      endpoint: "http://127.0.0.1:12393",
      message: "Connected to Open-LLM-VTuber",
    });

    render(
      <VTuberPresenceDock
        hubAgents={[
          { id: "claude", display_name: "Claude", animated_avatar: true },
        ]}
      />
    );

    const checkBtn = screen.getByRole("button", { name: /check bridge/i });
    fireEvent.click(checkBtn);

    await waitFor(() => {
      expect(api.testVtuberBridge).toHaveBeenCalledWith("http://127.0.0.1:12393");
      expect(screen.getByText(/● online/i)).toBeTruthy();
    });
  });

  it("renders failure notice and calls onDismissNotice on click", () => {
    const onDismiss = vi.fn();
    render(
      <VTuberPresenceDock
        hubAgents={[]}
        lastNotice="Open-LLM-VTuber offline: fallback to static avatar"
        onDismissNotice={onDismiss}
      />
    );

    expect(screen.getByTestId("vtuber-notice")).toBeTruthy();
    expect(screen.getByText(/open-llm-vtuber offline/i)).toBeTruthy();

    const dismissBtn = screen.getByRole("button", { name: /dismiss warning/i });
    fireEvent.click(dismissBtn);

    expect(onDismiss).toHaveBeenCalled();
  });
});
