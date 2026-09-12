import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import SystemTab, { formatBytes, formatRate } from "../SystemTab";
import { invoke } from "../../../../lib/tauri";

vi.mock("../../../../lib/tauri", () => ({ invoke: vi.fn() }));

const snapshot = {
  fetched_at: 1789000000,
  cpu_total_percent: 25,
  cpus: [{ name: "cpu0", usage_percent: 50, freq_mhz: 3000 }],
  memory: { total_bytes: 8 * 1024 ** 3, used_bytes: 4 * 1024 ** 3, available_bytes: 4 * 1024 ** 3 },
  swap: { total_bytes: 0, used_bytes: 0 },
  disks: [
    {
      name: "/dev/sda1",
      mount_point: "/",
      file_system: "ext4",
      total_bytes: 100 * 1024 ** 3,
      available_bytes: 40 * 1024 ** 3,
      read_bytes_total: 1000,
      written_bytes_total: 2000,
    },
  ],
  gpu: { available: false, detail: "nvidia-smi not found", gpus: [] },
  processes: [
    { pid: 123, name: "kimi", cpu_percent: 10, mem_bytes: 1024, harness: "kimi", workspace: "/ws" },
    { pid: 456, name: "brave", cpu_percent: 5, mem_bytes: 2048, harness: null, workspace: null },
  ],
};

describe("SystemTab (U23 / #316)", () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
  });

  it("renders meters, disks, gpu detail, and attributed processes", async () => {
    vi.mocked(invoke).mockResolvedValue(snapshot);
    render(<SystemTab />);
    await waitFor(() => expect(invoke).toHaveBeenCalledWith("system_snapshot"));
    expect(await screen.findByText(/System Resources/)).toBeTruthy();
    expect(screen.getByText("cpu0")).toBeTruthy();
    expect(screen.getByText(/nvidia-smi not found/)).toBeTruthy();
    expect(screen.getAllByText(/kimi/).length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText(/pid 123/)).toBeTruthy();
  });

  it("refreshes on demand without overlapping polls", async () => {
    vi.mocked(invoke).mockResolvedValue(snapshot);
    render(<SystemTab />);
    await waitFor(() => expect(invoke).toHaveBeenCalledTimes(1));
    fireEvent.click(screen.getByRole("button", { name: "Refresh now" }));
    await waitFor(() => expect(invoke).toHaveBeenCalledTimes(2));
  });

  it("shows backend errors instead of stale meters", async () => {
    vi.mocked(invoke).mockRejectedValueOnce(new Error("telemetry unavailable"));
    render(<SystemTab />);
    expect(await screen.findByText(/telemetry unavailable/)).toBeTruthy();
  });

  it("formats byte units and rates", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(1536)).toBe("1.5 KB");
    expect(formatBytes(8 * 1024 ** 3)).toBe("8.0 GB");
    expect(formatRate(NaN)).toBe("—");
    expect(formatRate(2048)).toBe("2.0 KB/s");
  });
});
