import { useCallback, useEffect, useState } from "react";
import { invoke } from "../../../lib/tauri";
import { loadWorkspaceRoot } from "../../../app/hubState";

export type GitBranch = {
  name: string;
  kind: string;
  tip: string;
  committed_at: number;
  subject: string;
  ahead: number;
  behind: number;
  inferred_issue: number | null;
  issue_number: number | null;
  issue_title: string | null;
  issue_state: string | null;
  issue_url: string | null;
  override_issue: boolean;
};

export type BranchList = {
  branches: GitBranch[];
  notice: string | null;
};

export function formatCommitAge(unixSeconds: number, nowSeconds = Math.floor(Date.now() / 1000)): string {
  if (!unixSeconds) return "unknown age";
  const secs = Math.max(0, nowSeconds - unixSeconds);
  if (secs < 60) return "just now";
  if (secs < 3600) return `${Math.floor(secs / 60)}m ago`;
  if (secs < 86400) return `${Math.floor(secs / 3600)}h ago`;
  return `${Math.floor(secs / 86400)}d ago`;
}

function workspaceRoot(): string | null {
  const value = loadWorkspaceRoot();
  return value.startsWith("/") ? value : null;
}

export default function BranchesTab() {
  const workspace = workspaceRoot();
  const [list, setList] = useState<BranchList | null>(null);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [drafts, setDrafts] = useState<Record<string, string>>({});

  const refresh = useCallback(async () => {
    if (!workspace) {
      setList(null);
      return;
    }
    setBusy(true);
    try {
      const next = await invoke<BranchList>("hub_list_git_branches", { workspace });
      setList(next);
      setError("");
    } catch (cause) {
      setError(String(cause));
    } finally {
      setBusy(false);
    }
  }, [workspace]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const saveOverride = async (branch: string, raw: string) => {
    if (!workspace) return;
    const trimmed = raw.trim();
    const issueNumber = trimmed === "" ? null : Number(trimmed);
    if (trimmed !== "" && (!Number.isInteger(issueNumber) || (issueNumber ?? 0) <= 0)) {
      setError("Linked issue must be a positive number, or empty to clear.");
      return;
    }
    setBusy(true);
    try {
      const next = await invoke<BranchList>("hub_set_branch_issue", {
        workspace,
        branch,
        issueNumber,
      });
      setList(next);
      setError("");
    } catch (cause) {
      setError(String(cause));
    } finally {
      setBusy(false);
    }
  };

  if (!workspace) {
    return (
      <div>
        <strong style={{ color: "var(--text-main)" }}>Branches</strong>
        <p style={{ color: "var(--text-muted)", fontSize: "0.85rem" }}>
          Set an absolute workspace root in Config to list local and remote-tracking branches.
        </p>
      </div>
    );
  }

  return (
    <div>
      <div style={{ display: "flex", gap: "0.5rem", alignItems: "center", marginBottom: "0.85rem", flexWrap: "wrap" }}>
        <strong style={{ color: "var(--text-main)" }}>Branches</strong>
        <button
          type="button"
          className="btn-secondary"
          style={{ marginTop: 0 }}
          disabled={busy}
          onClick={() => void refresh()}
        >
          {busy ? "Refreshing…" : "Refresh"}
        </button>
        <span style={{ color: "var(--text-muted)", fontSize: "0.78rem" }}>
          Read-only listing. Checkout and delete stay in the terminal.
        </span>
      </div>
      {(error || list?.notice) && (
        <div style={{ marginBottom: "0.75rem", color: "#fecaca", fontSize: "0.82rem" }}>
          {error || list?.notice}
        </div>
      )}
      <div style={{ display: "grid", gap: "0.55rem" }}>
        {(list?.branches ?? []).map((branch) => (
          <article
            key={`${branch.kind}:${branch.name}`}
            style={{
              padding: "0.65rem 0.75rem",
              borderRadius: "10px",
              background: "rgba(15,23,42,0.7)",
              border: "1px solid rgba(148,163,184,0.25)",
            }}
          >
            <div style={{ display: "flex", justifyContent: "space-between", gap: "0.75rem", flexWrap: "wrap" }}>
              <div>
                <div style={{ color: "#fff", fontWeight: 600, fontSize: "0.88rem" }}>
                  {branch.name}
                  <span style={{ marginLeft: "0.5rem", color: "var(--text-muted)", fontWeight: 400, fontSize: "0.72rem" }}>
                    {branch.kind} · {branch.tip} · {formatCommitAge(branch.committed_at)}
                  </span>
                </div>
                <div style={{ color: "var(--text-muted)", fontSize: "0.78rem", marginTop: "0.2rem" }}>
                  {branch.subject || "(no subject)"} · +{branch.ahead} / −{branch.behind} vs main
                </div>
              </div>
              <div style={{ minWidth: "220px" }}>
                {branch.issue_number ? (
                  <div style={{ fontSize: "0.8rem", color: "#bbf7d0" }}>
                    #{branch.issue_number}
                    {branch.issue_state ? ` · ${branch.issue_state}` : ""}
                    {branch.override_issue ? " · override" : ""}
                    <div style={{ color: "var(--text-main)" }}>
                      {branch.issue_url ? (
                        <a href={branch.issue_url} style={{ color: "#7dd3fc" }}>
                          {branch.issue_title || "linked issue"}
                        </a>
                      ) : (
                        branch.issue_title || "linked issue (GitHub title unavailable)"
                      )}
                    </div>
                  </div>
                ) : (
                  <div style={{ color: "var(--text-muted)", fontSize: "0.78rem" }}>No linked issue</div>
                )}
                <form
                  onSubmit={(event) => {
                    event.preventDefault();
                    void saveOverride(branch.name, drafts[branch.name] ?? String(branch.issue_number ?? ""));
                  }}
                  style={{ display: "flex", gap: "0.35rem", marginTop: "0.35rem" }}
                >
                  <input
                    inputMode="numeric"
                    placeholder="issue #"
                    value={drafts[branch.name] ?? String(branch.issue_number ?? "")}
                    onChange={(event) =>
                      setDrafts((prev) => ({ ...prev, [branch.name]: event.target.value }))
                    }
                    style={{
                      width: "88px",
                      padding: "0.25rem 0.4rem",
                      borderRadius: "6px",
                      border: "1px solid var(--border-color)",
                      background: "rgba(0,0,0,0.3)",
                      color: "white",
                      fontSize: "0.75rem",
                    }}
                  />
                  <button type="submit" className="btn-secondary" style={{ marginTop: 0, fontSize: "0.72rem" }} disabled={busy}>
                    Link
                  </button>
                </form>
              </div>
            </div>
          </article>
        ))}
        {list && list.branches.length === 0 && (
          <p style={{ color: "var(--text-muted)" }}>No local or remote-tracking branches found.</p>
        )}
      </div>
    </div>
  );
}
