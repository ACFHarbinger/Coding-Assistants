import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import ScheduleTriggerSection from "../ScheduleTriggerSection";
import * as api from "../scheduleTriggerApi";
import type { ScheduleTriggerStatus } from "../scheduleTriggerApi";

vi.mock("../scheduleTriggerApi", () => ({
  getScheduleTriggerStatus: vi.fn(),
  installScheduleTrigger: vi.fn(),
  removeScheduleTrigger: vi.fn(),
}));

const baseStatus: ScheduleTriggerStatus = {
  available: true,
  installed: false,
  enabled: false,
  active: false,
  service_path: "/home/u/.config/systemd/user/ca-schedule.service",
  timer_path: "/home/u/.config/systemd/user/ca-schedule.timer",
  exec_start: null,
  detail: null,
};

describe("ScheduleTriggerSection (#356)", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("shows the workspace prerequisite and disables install without one", async () => {
    vi.mocked(api.getScheduleTriggerStatus).mockResolvedValue(baseStatus);
    render(<ScheduleTriggerSection workspaceRoot={null} busy={false} />);

    await waitFor(() => expect(screen.getByText(/Select an active workspace/)).toBeInTheDocument());
    expect(screen.getByRole("button", { name: "Install trigger" })).toBeDisabled();
  });

  it("installs the timer with the chosen interval and workspace", async () => {
    vi.mocked(api.getScheduleTriggerStatus).mockResolvedValue(baseStatus);
    vi.mocked(api.installScheduleTrigger).mockResolvedValue({
      ...baseStatus,
      installed: true,
      enabled: true,
      active: true,
      exec_start: '"/usr/bin/ca" --home "/home/u/.coding-assistants" schedule tick',
    });
    render(<ScheduleTriggerSection workspaceRoot="/work/space" busy={false} />);

    await waitFor(() => expect(screen.getByRole("button", { name: "Install trigger" })).toBeEnabled());
    fireEvent.change(screen.getByLabelText(/Every/), { target: { value: "10" } });
    fireEvent.click(screen.getByRole("button", { name: "Install trigger" }));

    await waitFor(() =>
      expect(api.installScheduleTrigger).toHaveBeenCalledWith({
        ca_bin: null,
        interval_minutes: 10,
        workspace: "/work/space",
      }),
    );
    expect(await screen.findByText("Timer installed and enabled.")).toBeInTheDocument();
    expect(screen.getByText(/^Timer: .*ca-schedule\.timer$/)).toBeInTheDocument();
  });

  it("rejects a non-positive interval before calling the backend", async () => {
    vi.mocked(api.getScheduleTriggerStatus).mockResolvedValue(baseStatus);
    render(<ScheduleTriggerSection workspaceRoot="/work/space" busy={false} />);

    await waitFor(() => expect(screen.getByRole("button", { name: "Install trigger" })).toBeEnabled());
    fireEvent.change(screen.getByLabelText(/Every/), { target: { value: "0" } });
    fireEvent.click(screen.getByRole("button", { name: "Install trigger" }));

    expect(await screen.findByText(/at least 1/)).toBeInTheDocument();
    expect(api.installScheduleTrigger).not.toHaveBeenCalled();
  });

  it("removes an installed timer", async () => {
    vi.mocked(api.getScheduleTriggerStatus).mockResolvedValue({
      ...baseStatus,
      installed: true,
      enabled: true,
      active: true,
      exec_start: '"/usr/bin/ca" --home "/h" schedule tick',
    });
    vi.mocked(api.removeScheduleTrigger).mockResolvedValue(baseStatus);
    render(<ScheduleTriggerSection workspaceRoot="/work/space" busy={false} />);

    fireEvent.click(await screen.findByRole("button", { name: "Remove trigger" }));
    await waitFor(() => expect(api.removeScheduleTrigger).toHaveBeenCalled());
    expect(await screen.findByText(/disabled and unit files removed/)).toBeInTheDocument();
  });

  it("surfaces unavailable-systemd diagnostics", async () => {
    vi.mocked(api.getScheduleTriggerStatus).mockResolvedValue({
      ...baseStatus,
      available: false,
      detail: "systemd user manager is unavailable (systemctl --user failed: no bus).",
    });
    render(<ScheduleTriggerSection workspaceRoot="/work/space" busy={false} />);

    expect(await screen.findByText(/systemd unavailable/)).toBeInTheDocument();
    expect(screen.getByText(/no bus/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Install trigger" })).toBeDisabled();
  });
});
