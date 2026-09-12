import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import WorkspaceRootSection from "../WorkspaceRootSection";
import * as tauriLib from "../../../../lib/tauri";
import { open } from "@tauri-apps/plugin-dialog";

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(),
}));

describe("WorkspaceRootSection Component (#215)", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders workspace input and buttons", () => {
    const onWorkDirChange = vi.fn();
    const onWorkspaceApplied = vi.fn();

    render(
      <WorkspaceRootSection
        workDir="/home/user/my-project"
        onWorkDirChange={onWorkDirChange}
        onWorkspaceApplied={onWorkspaceApplied}
      />
    );

    expect(screen.getByLabelText("Workspace Root")).toBeInTheDocument();
    expect(screen.getByDisplayValue("/home/user/my-project")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Switch Workspace" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Browse" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Initialize .agent/" })).toBeInTheDocument();
  });

  it("calls onWorkDirChange when user types in the input", () => {
    const onWorkDirChange = vi.fn();
    const onWorkspaceApplied = vi.fn();

    render(
      <WorkspaceRootSection
        workDir="/home/user/my-project"
        onWorkDirChange={onWorkDirChange}
        onWorkspaceApplied={onWorkspaceApplied}
      />
    );

    const input = screen.getByLabelText("Workspace Root");
    fireEvent.change(input, { target: { value: "/home/user/other-project" } });
    expect(onWorkDirChange).toHaveBeenCalledWith("/home/user/other-project");
  });

  it("applies workspace when Switch Workspace is clicked with non-empty path", () => {
    const onWorkDirChange = vi.fn();
    const onWorkspaceApplied = vi.fn();

    render(
      <WorkspaceRootSection
        workDir="/home/user/my-project"
        onWorkDirChange={onWorkDirChange}
        onWorkspaceApplied={onWorkspaceApplied}
      />
    );

    const switchBtn = screen.getByRole("button", { name: "Switch Workspace" });
    fireEvent.click(switchBtn);

    expect(onWorkspaceApplied).toHaveBeenCalledWith("/home/user/my-project");
  });

  it("alerts and does not apply workspace when path is empty", () => {
    const alertMock = vi.spyOn(window, "alert").mockImplementation(() => {});
    const onWorkDirChange = vi.fn();
    const onWorkspaceApplied = vi.fn();

    render(
      <WorkspaceRootSection
        workDir="   "
        onWorkDirChange={onWorkDirChange}
        onWorkspaceApplied={onWorkspaceApplied}
      />
    );

    const switchBtn = screen.getByRole("button", { name: "Switch Workspace" });
    fireEvent.click(switchBtn);

    expect(alertMock).toHaveBeenCalledWith("Set an absolute workspace path first.");
    expect(onWorkspaceApplied).not.toHaveBeenCalled();
    alertMock.mockRestore();
  });

  it("requires explicit user confirmation before creating non-existent directory on disk", async () => {
    vi.spyOn(tauriLib, "isTauriRuntime").mockReturnValue(true);
    const invokeSpy = vi.spyOn(tauriLib, "invoke").mockImplementation(async (cmd) => {
      if (cmd === "validate_workspace_path") {
        return {
          path: "/home/user/new-dir",
          valid: false,
          exists: false,
          is_dir: false,
          is_bootstrapped: false,
          parent_exists: true,
          is_system_dir: false,
          error: "Directory does not exist",
        };
      }
      if (cmd === "bootstrap_workspace") {
        return null;
      }
      return null;
    });

    const confirmMock = vi.spyOn(window, "confirm").mockReturnValue(false); // User cancels
    const onWorkDirChange = vi.fn();
    const onWorkspaceApplied = vi.fn();

    render(
      <WorkspaceRootSection
        workDir="/home/user/new-dir"
        onWorkDirChange={onWorkDirChange}
        onWorkspaceApplied={onWorkspaceApplied}
      />
    );

    // Wait for validation to settle
    await waitFor(() => {
      expect(screen.getByText(/Directory does not exist on disk/i)).toBeInTheDocument();
    });

    const initBtn = screen.getByRole("button", { name: "Initialize .agent/" });
    fireEvent.click(initBtn);

    // Confirm was shown
    expect(confirmMock).toHaveBeenCalled();
    // But bootstrap was NOT called because user cancelled
    expect(invokeSpy).not.toHaveBeenCalledWith("bootstrap_workspace", expect.anything());

    // Now test user confirming
    confirmMock.mockReturnValue(true);
    fireEvent.click(initBtn);

    await waitFor(() => {
      expect(invokeSpy).toHaveBeenCalledWith("bootstrap_workspace", {
        workDir: "/home/user/new-dir",
        createDir: true,
      });
    });

    confirmMock.mockRestore();
    invokeSpy.mockRestore();
  });

  it("rejects bootstrap when parent directory does not exist", async () => {
    vi.spyOn(tauriLib, "isTauriRuntime").mockReturnValue(true);
    const invokeSpy = vi.spyOn(tauriLib, "invoke").mockImplementation(async (cmd) => {
      if (cmd === "validate_workspace_path") {
        return {
          path: "/tmp/a/b/c",
          valid: false,
          exists: false,
          is_dir: false,
          is_bootstrapped: false,
          parent_exists: false,
          is_system_dir: false,
          error: "Directory does not exist",
        };
      }
      return null;
    });

    const alertMock = vi.spyOn(window, "alert").mockImplementation(() => {});
    const confirmMock = vi.spyOn(window, "confirm").mockReturnValue(true);

    render(
      <WorkspaceRootSection
        workDir="/tmp/a/b/c"
        onWorkDirChange={vi.fn()}
        onWorkspaceApplied={vi.fn()}
      />
    );

    await waitFor(() => {
      expect(screen.getByText(/parent also missing/i)).toBeInTheDocument();
    });

    const initBtn = screen.getByRole("button", { name: "Initialize .agent/" });
    fireEvent.click(initBtn);

    expect(alertMock).toHaveBeenCalledWith(
      expect.stringContaining("parent directory does not exist")
    );
    expect(invokeSpy).not.toHaveBeenCalledWith("bootstrap_workspace", expect.anything());

    alertMock.mockRestore();
    confirmMock.mockRestore();
    invokeSpy.mockRestore();
  });

  it("blocks bootstrap and switch if path is a forbidden system directory", async () => {
    vi.spyOn(tauriLib, "isTauriRuntime").mockReturnValue(true);
    const invokeSpy = vi.spyOn(tauriLib, "invoke").mockImplementation(async (cmd) => {
      if (cmd === "validate_workspace_path") {
        return {
          path: "/etc",
          valid: false,
          exists: true,
          is_dir: true,
          is_bootstrapped: false,
          parent_exists: true,
          is_system_dir: true,
          error: "Cannot use system directory as workspace",
        };
      }
      return null;
    });

    const alertMock = vi.spyOn(window, "alert").mockImplementation(() => {});

    render(
      <WorkspaceRootSection
        workDir="/etc"
        onWorkDirChange={vi.fn()}
        onWorkspaceApplied={vi.fn()}
      />
    );

    await waitFor(() => {
      expect(
        screen.getByText(/Forbidden: System directory cannot be used as workspace root/i)
      ).toBeInTheDocument();
    });

    const switchBtn = screen.getByRole("button", { name: "Switch Workspace" });
    fireEvent.click(switchBtn);
    expect(alertMock).toHaveBeenCalledWith(expect.stringContaining("system directory"));

    const initBtn = screen.getByRole("button", { name: "Initialize .agent/" });
    fireEvent.click(initBtn);
    expect(alertMock).toHaveBeenCalledWith(expect.stringContaining("system directory"));

    alertMock.mockRestore();
    invokeSpy.mockRestore();
  });

  it("handles directory browse dialog selection", async () => {
    vi.mocked(open).mockResolvedValue("/home/user/browsed-repo" as any);

    const onWorkDirChange = vi.fn();
    const onWorkspaceApplied = vi.fn();

    render(
      <WorkspaceRootSection
        workDir="/home/user/initial"
        onWorkDirChange={onWorkDirChange}
        onWorkspaceApplied={onWorkspaceApplied}
      />
    );

    const browseBtn = screen.getByRole("button", { name: "Browse" });
    fireEvent.click(browseBtn);

    await waitFor(() => {
      expect(onWorkDirChange).toHaveBeenCalledWith("/home/user/browsed-repo");
      expect(onWorkspaceApplied).toHaveBeenCalledWith("/home/user/browsed-repo");
    });
  });

  it("bootstraps existing unbootstrapped directory without createDir flag", async () => {
    vi.spyOn(tauriLib, "isTauriRuntime").mockReturnValue(true);
    const invokeSpy = vi.spyOn(tauriLib, "invoke").mockImplementation(async (cmd) => {
      if (cmd === "validate_workspace_path") {
        return {
          path: "/home/user/existing-repo",
          valid: true,
          exists: true,
          is_dir: true,
          is_bootstrapped: false,
          parent_exists: true,
          is_system_dir: false,
          error: null,
        };
      }
      if (cmd === "bootstrap_workspace") {
        return null;
      }
      return null;
    });

    const confirmSpy = vi.spyOn(window, "confirm");

    render(
      <WorkspaceRootSection
        workDir="/home/user/existing-repo"
        onWorkDirChange={vi.fn()}
        onWorkspaceApplied={vi.fn()}
      />
    );

    await waitFor(() => {
      expect(screen.getByText(/Directory exists; .agent\/ not initialized/i)).toBeInTheDocument();
    });

    const initBtn = screen.getByRole("button", { name: "Initialize .agent/" });
    fireEvent.click(initBtn);

    // Existing directory doesn't ask to create directory on disk
    expect(confirmSpy).not.toHaveBeenCalled();
    await waitFor(() => {
      expect(invokeSpy).toHaveBeenCalledWith("bootstrap_workspace", {
        workDir: "/home/user/existing-repo",
        createDir: false,
      });
    });

    confirmSpy.mockRestore();
    invokeSpy.mockRestore();
  });

  it("alerts if workspace is already bootstrapped", async () => {
    vi.spyOn(tauriLib, "isTauriRuntime").mockReturnValue(true);
    vi.spyOn(tauriLib, "invoke").mockImplementation(async (cmd) => {
      if (cmd === "validate_workspace_path") {
        return {
          path: "/home/user/bootstrapped-repo",
          valid: true,
          exists: true,
          is_dir: true,
          is_bootstrapped: true,
          parent_exists: true,
          is_system_dir: false,
          error: null,
        };
      }
      return null;
    });

    const alertMock = vi.spyOn(window, "alert").mockImplementation(() => {});

    render(
      <WorkspaceRootSection
        workDir="/home/user/bootstrapped-repo"
        onWorkDirChange={vi.fn()}
        onWorkspaceApplied={vi.fn()}
      />
    );

    await waitFor(() => {
      expect(screen.getByText(/Ready: .agent\/ initialized/i)).toBeInTheDocument();
    });

    const initBtn = screen.getByRole("button", { name: "Initialize .agent/" });
    fireEvent.click(initBtn);

    expect(alertMock).toHaveBeenCalledWith(
      expect.stringContaining("already bootstrapped")
    );

    alertMock.mockRestore();
  });
});
