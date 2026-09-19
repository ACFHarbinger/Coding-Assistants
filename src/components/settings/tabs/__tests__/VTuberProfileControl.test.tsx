import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import VTuberProfileControl from "../agents/VTuberProfileControl";
import * as api from "../../api";

vi.mock("../../api", () => ({
  setAgentAnimatedAvatar: vi.fn(),
  testVtuberBridge: vi.fn(),
}));

describe("VTuberProfileControl (U17 / #307)", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders default static avatar button when animated_avatar is false", () => {
    render(
      <VTuberProfileControl
        agent={{ id: "claude", display_name: "Claude", animated_avatar: false }}
        onChanged={vi.fn()}
      />
    );

    expect(screen.getByRole("button", { name: /static avatar/i })).toBeTruthy();
  });

  it("toggles to animated avatar and calls setAgentAnimatedAvatar", async () => {
    const onChanged = vi.fn();
    vi.mocked(api.setAgentAnimatedAvatar).mockResolvedValue({
      id: "claude",
      display_name: "Claude",
      animated_avatar: true,
    });

    render(
      <VTuberProfileControl
        agent={{ id: "claude", display_name: "Claude", animated_avatar: false }}
        onChanged={onChanged}
      />
    );

    const toggleBtn = screen.getByRole("button", { name: /static avatar/i });
    fireEvent.click(toggleBtn);

    await waitFor(() => {
      expect(api.setAgentAnimatedAvatar).toHaveBeenCalledWith("claude", true, null);
      expect(onChanged).toHaveBeenCalled();
    });
  });

  it("expands config and saves character name", async () => {
    const onChanged = vi.fn();
    vi.mocked(api.setAgentAnimatedAvatar).mockResolvedValue({
      id: "gemini",
      display_name: "Gemini",
      animated_avatar: true,
      vtuber_character: "shizuku",
    });

    render(
      <VTuberProfileControl
        agent={{
          id: "gemini",
          display_name: "Gemini",
          animated_avatar: true,
          vtuber_character: "default",
        }}
        onChanged={onChanged}
      />
    );

    expect(screen.getByRole("button", { name: /animated live2d/i })).toBeTruthy();
    const configBtn = screen.getByRole("button", { name: /config…/i });
    fireEvent.click(configBtn);

    const input = screen.getByPlaceholderText(/e\.g\. shizuku/i);
    fireEvent.change(input, { target: { value: "shizuku" } });

    const saveBtn = screen.getByRole("button", { name: /save/i });
    fireEvent.click(saveBtn);

    await waitFor(() => {
      expect(api.setAgentAnimatedAvatar).toHaveBeenCalledWith("gemini", true, "shizuku");
      expect(onChanged).toHaveBeenCalled();
    });
  });

  it("tests bridge connection and renders online status", async () => {
    vi.mocked(api.testVtuberBridge).mockResolvedValue({
      available: true,
      endpoint: "http://127.0.0.1:12393",
      message: "Connected to Open-LLM-VTuber at http://127.0.0.1:12393",
    });

    render(
      <VTuberProfileControl
        agent={{
          id: "gemini",
          display_name: "Gemini",
          animated_avatar: true,
        }}
      />
    );

    const configBtn = screen.getByRole("button", { name: /config…/i });
    fireEvent.click(configBtn);

    const testBtn = screen.getByRole("button", { name: /test bridge/i });
    fireEvent.click(testBtn);

    await waitFor(() => {
      expect(api.testVtuberBridge).toHaveBeenCalled();
      expect(screen.getByText(/● available:/i)).toBeTruthy();
      expect(screen.getByText(/connected to open-llm-vtuber/i)).toBeTruthy();
    });
  });

  it("surfaces error if toggling fails", async () => {
    vi.mocked(api.setAgentAnimatedAvatar).mockRejectedValue(new Error("Database write locked"));

    render(
      <VTuberProfileControl
        agent={{ id: "claude", display_name: "Claude", animated_avatar: false }}
      />
    );

    const toggleBtn = screen.getByRole("button", { name: /static avatar/i });
    fireEvent.click(toggleBtn);

    await waitFor(() => {
      expect(screen.getByText(/database write locked/i)).toBeTruthy();
    });
  });
});
