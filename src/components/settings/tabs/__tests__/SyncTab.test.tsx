import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import SyncTab from "../SyncTab";
import * as api from "../syncApi";

vi.mock("../syncApi", () => ({
  hubSyncPreview: vi.fn(),
  hubSyncStart: vi.fn(),
  hubSyncStatus: vi.fn(),
  hubSyncCancel: vi.fn(),
  hubSyncConflicts: vi.fn(),
  hubSyncResolve: vi.fn(),
  hubSyncTombstones: vi.fn(),
  hubSyncExpired: vi.fn(),
  hubSyncPurgeExpired: vi.fn(),
  hubSyncDevices: vi.fn(),
  hubSyncRegister: vi.fn(),
  hubSyncRevoke: vi.fn(),
  hubSyncTrust: vi.fn(),
  hubSyncRetry: vi.fn(),
  hubSyncHistory: vi.fn(),
  hubSyncDiagnostics: vi.fn(),
  hubSyncLimits: vi.fn(),
}));

const plan = {
  action: "preview",
  account_connected: true,
  provider: "google-drive",
  local_schema: "3",
  replica_schema: "2",
  schema_warning: "hub schema mismatch: local 3, replica 2",
  last_verified_base: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  category_counts: { hub_database: 1, private_journal: 1 },
  lock_held: false,
  errors: [],
};

describe("SyncTab (#94)", () => {
  beforeEach(() => {
    vi.mocked(api.hubSyncStatus).mockResolvedValue({ plan, lock: null });
    vi.mocked(api.hubSyncPreview).mockResolvedValue(plan);
    vi.mocked(api.hubSyncStart).mockResolvedValue({
      plan: { ...plan, action: "up", lock_held: true },
      result: {
        uploaded: 0,
        downloaded: 0,
        pruned: 0,
        conflicts: 0,
        warnings: ["live hub.db was not replaced"],
      },
    });
    vi.mocked(api.hubSyncCancel).mockResolvedValue();
    vi.mocked(api.hubSyncConflicts).mockResolvedValue([
      {
        slug: "markdown__note.md",
        path: "markdown/note.md",
        reason: "same-path-edit",
        decision: null,
        local_hash: "aa",
        remote_hash: "bb",
        base_hash: null,
      },
    ]);
    vi.mocked(api.hubSyncResolve).mockResolvedValue({
      choice: "local",
      decided_at: "t",
      local_hash: "aa",
      remote_hash: "bb",
      base_hash: null,
    });
    vi.mocked(api.hubSyncTombstones).mockResolvedValue([
      {
        slug: "markdown__gone.md",
        path: "markdown/gone.md",
        created_at: "t0",
        expires_at: "t1",
        content_hash: "cc",
        policy: "confirm-only",
      },
    ]);
    vi.mocked(api.hubSyncExpired).mockResolvedValue([
      { kind: "tombstone", slug: "markdown__gone.md", aged_at: "t1" },
    ]);
    vi.mocked(api.hubSyncPurgeExpired).mockResolvedValue({ purged: ["tombstone:markdown__gone.md"] });
    vi.mocked(api.hubSyncDevices).mockResolvedValue({
      schema: 1,
      devices: [
        {
          id: "aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee",
          folder: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
          status: "trusted",
          registered_at: "t0",
          revoked_at: null,
        },
      ],
    });
    vi.mocked(api.hubSyncLimits).mockResolvedValue({
      max_objects: 10000,
      max_bytes: 536870912,
      max_concurrent: 2,
    });
    vi.mocked(api.hubSyncRetry).mockResolvedValue({
      plan: { ...plan, action: "up", lock_held: true },
      result: {
        uploaded: 1,
        downloaded: 0,
        pruned: 0,
        conflicts: 0,
        warnings: ["live hub.db was not replaced"],
      },
    });
    vi.mocked(api.hubSyncDiagnostics).mockResolvedValue({
      schema: 1,
      last_verified_base: plan.last_verified_base,
      trust: [{ folder: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", status: "trusted" }],
      limits: { max_objects: 10000, max_bytes: 536870912, max_concurrent: 2 },
      resume_action: null,
      history: [],
    });
    vi.mocked(api.hubSyncHistory).mockResolvedValue([]);
  });

  it("shows account, schema warning, and hashed base without secret names", async () => {
    render(<SyncTab />);
    expect(await screen.findByText(/connected \(google-drive\)/)).toBeInTheDocument();
    expect(screen.getByText(/hub schema mismatch/)).toBeInTheDocument();
    expect(screen.getByText(plan.last_verified_base)).toBeInTheDocument();
    expect(screen.queryByText("cloud-sync.key")).not.toBeInTheDocument();
    expect(screen.queryByText(/Bearer|ya29/)).not.toBeInTheDocument();
  });

  it("preview and start/cancel call the same IPC surface as the CLI", async () => {
    render(<SyncTab />);
    fireEvent.click(await screen.findByRole("button", { name: "Preview" }));
    await waitFor(() => expect(api.hubSyncPreview).toHaveBeenCalled());
    fireEvent.click(screen.getByRole("button", { name: "up" }));
    await waitFor(() => expect(api.hubSyncStart).toHaveBeenCalledWith("up"));
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    await waitFor(() => expect(api.hubSyncCancel).toHaveBeenCalled());
  });

  it("lists queued conflicts and resolve click applies a choice", async () => {
    render(<SyncTab />);
    expect(await screen.findByText("markdown__note.md")).toBeInTheDocument();
    expect(screen.getByText("same-path-edit")).toBeInTheDocument();
    expect(screen.getByText("pending")).toBeInTheDocument();
    expect(screen.queryByText("cloud-sync.key")).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Local" }));
    await waitFor(() =>
      expect(api.hubSyncResolve).toHaveBeenCalledWith("markdown__note.md", "local"),
    );
    expect(screen.getByText("markdown__gone.md")).toBeInTheDocument();
    expect(screen.getByText(/Expired copies \(1\)/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Purge expired (30d)" }));
    await waitFor(() => expect(api.hubSyncPurgeExpired).toHaveBeenCalledWith(true));
  });

  it("shows hashed trust counts and retry/diagnostics without secrets", async () => {
    render(<SyncTab />);
    expect(await screen.findByText(/Trusted 1 · Revoked 0/)).toBeInTheDocument();
    expect(screen.getByText("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")).toBeInTheDocument();
    expect(screen.queryByText("cloud-sync.key")).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Retry" }));
    await waitFor(() => expect(api.hubSyncRetry).toHaveBeenCalled());
    fireEvent.click(screen.getByRole("button", { name: "Diagnostics" }));
    await waitFor(() => expect(api.hubSyncDiagnostics).toHaveBeenCalled());
    expect(screen.queryByText(/Bearer|ya29/)).not.toBeInTheDocument();
  });
});
