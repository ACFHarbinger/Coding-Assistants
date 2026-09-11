import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import DashboardPanel from "../DashboardPanel";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(async (cmd: string) => {
    if (cmd === "hub_get_budget") return null;
    return [];
  }),
}));

const agents = [
  { id: "kimi", display_name: "Kimi" },
  { id: "claude", display_name: "Claude" },
];

describe("DashboardPanel roster enroll (U22 / #225)", () => {
  it("renders Enroll for outsiders and Unenroll for members", async () => {
    render(
      <DashboardPanel
        agents={agents}
        teamMemberIds={["claude"]}
        onAddAgent={vi.fn()}
        onRemoveAgent={vi.fn()}
      />,
    );
    await waitFor(() => expect(screen.getByText("Kimi")).toBeTruthy());
    expect(screen.getByRole("button", { name: "Enroll" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Unenroll" })).toBeTruthy();
  });

  it("enrolls through the shared handler with the roster mapping", async () => {
    const onAddAgent = vi.fn();
    render(
      <DashboardPanel
        agents={agents}
        teamMemberIds={[]}
        onAddAgent={onAddAgent}
        onRemoveAgent={vi.fn()}
      />,
    );
    await waitFor(() => expect(screen.getByText("Kimi")).toBeTruthy());
    fireEvent.click(screen.getAllByRole("button", { name: "Enroll" })[0]);
    expect(onAddAgent).toHaveBeenCalledWith({
      id: "kimi",
      target_id: "kimi",
      name: "Kimi",
      provider: "",
      model: "",
      origin: "existing",
    });
  });

  it("unenrolls through the shared handler", async () => {
    const onRemoveAgent = vi.fn();
    render(
      <DashboardPanel
        agents={agents}
        teamMemberIds={["kimi", "claude"]}
        onAddAgent={vi.fn()}
        onRemoveAgent={onRemoveAgent}
      />,
    );
    await waitFor(() => expect(screen.getByText("Kimi")).toBeTruthy());
    fireEvent.click(screen.getAllByRole("button", { name: "Unenroll" })[0]);
    expect(onRemoveAgent).toHaveBeenCalledWith(
      expect.objectContaining({ id: "kimi", target_id: "kimi" }),
    );
  });

  it("displays visible error and does not toggle state on failed enroll", async () => {
    const onAddAgent = vi.fn().mockRejectedValue(new Error("Could not enroll kimi: HubStore failure"));
    render(
      <DashboardPanel
        agents={agents}
        teamMemberIds={[]}
        onAddAgent={onAddAgent}
        onRemoveAgent={vi.fn()}
      />,
    );
    await waitFor(() => expect(screen.getByText("Kimi")).toBeTruthy());
    const enrollButtons = screen.getAllByRole("button", { name: "Enroll" });
    fireEvent.click(enrollButtons[0]);
    expect(await screen.findByText(/Could not enroll kimi: HubStore failure/i)).toBeTruthy();
    expect(screen.getAllByRole("button", { name: "Enroll" })[0]).toBeTruthy();
  });

  it("displays visible error and does not toggle state on failed unenroll", async () => {
    const onRemoveAgent = vi.fn().mockRejectedValue(new Error("Could not remove claude: HubStore failure"));
    render(
      <DashboardPanel
        agents={agents}
        teamMemberIds={["claude"]}
        onAddAgent={vi.fn()}
        onRemoveAgent={onRemoveAgent}
      />,
    );
    await waitFor(() => expect(screen.getByText("Claude")).toBeTruthy());
    const unenrollBtn = screen.getByRole("button", { name: "Unenroll" });
    fireEvent.click(unenrollBtn);
    expect(await screen.findByText(/Could not remove claude: HubStore failure/i)).toBeTruthy();
    expect(screen.getByRole("button", { name: "Unenroll" })).toBeTruthy();
  });
});
