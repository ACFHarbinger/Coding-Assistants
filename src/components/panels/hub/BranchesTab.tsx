import { useCallback, useEffect, useState } from "react";
import { invoke } from "../../../lib/tauri";
import { loadWorkspaceRoot } from "../../../app/hubState";
import { nocPanelStyle, nocCardDenseStyle } from "../../common/telemetry/panel";
import { EmptyStateChip } from "../../common/telemetry/status";
import { NOC_COLORS, NOC_SURFACES } from "../../common/telemetry/tokens";

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
      <div style={nocPanelStyle}>
        <EmptyStateChip
          title="No workspace root configured"
          detail="Set an absolute workspace root in Config to list local and remote-tracking branches."
          tone="idle"
        />
      </div>
    );
  }

  return (
    <div className="fade-in" style={{ display: "flex", flexDirection: "column", gap: "1.25rem" }}>
      {/* Header bar */}
      <div style={{ ...nocPanelStyle, display: "flex", gap: "0.75rem", alignItems: "center", justifyContent: "space-between", flexWrap: "wrap" }}>
        <div>
          <h3 style={{ margin: 0, fontSize: "1.05rem", fontWeight: 700, color: NOC_SURFACES.textPrimary }}>Branches</h3>
          <p style={{ margin: "0.2rem 0 0", color: NOC_SURFACES.textMuted, fontSize: "0.82rem" }}>
            Read-only listing with proportional ahead/behind diff bars. Checkout and branch mutations stay in the terminal.
          </p>
        </div>
        <button
          type="button"
          className="btn-secondary noc-focusable"
          style={{ fontSize: "0.78rem", padding: "0.35rem 0.75rem" }}
          disabled={busy}
          onClick={() => void refresh()}
        >
          {busy ? "Refreshing…" : "Refresh"}
        </button>
      </div>

      {(error || list?.notice) && (
        <div style={{ color: NOC_COLORS.crit, fontSize: "0.82rem", background: "rgba(239, 68, 68, 0.1)", padding: "0.45rem 0.75rem", borderRadius: "8px" }}>
          {error || list?.notice}
        </div>
      )}

      {/* Branches list */}
      <div style={{ display: "grid", gap: "0.65rem" }}>
        {(list?.branches ?? []).map((branch) => {
          const totalDiff = Math.max(1, branch.ahead + branch.behind);
          const aheadPct = Math.round((branch.ahead / totalDiff) * 100);
          const behindPct = 100 - aheadPct;
          const isInSync = branch.ahead === 0 && branch.behind === 0;

          return (
            <article
              key={`${branch.kind}:${branch.name}`}
              style={{
                ...nocCardDenseStyle,
                padding: "0.75rem 0.95rem",
              }}
            >
              <div style={{ display: "flex", justifyContent: "space-between", gap: "1rem", flexWrap: "wrap", alignItems: "center" }}>
                <div style={{ minWidth: 260, flex: 1 }}>
                  <div style={{ display: "flex", alignItems: "center", gap: "0.5rem", flexWrap: "wrap" }}>
                    <span style={{ color: NOC_COLORS.cyan, fontWeight: 700, fontSize: "0.92rem", fontFamily: "var(--font-mono)" }}>
                      {branch.name}
                    </span>
                    <span
                      style={{
                        color: NOC_SURFACES.textMuted,
                        fontSize: "0.72rem",
                        padding: "0.1rem 0.4rem",
                        background: "rgba(255,255,255,0.06)",
                        borderRadius: "4px",
                      }}
                    >
                      {branch.kind}
                    </span>
                    <span className="tabular-nums" style={{ color: NOC_SURFACES.textMuted, fontSize: "0.72rem", fontFamily: "var(--font-mono)" }}>
                      {branch.tip}
                    </span>
                    <span style={{ color: NOC_SURFACES.textMuted, fontSize: "0.72rem" }}>
                      · {formatCommitAge(branch.committed_at)}
                    </span>
                  </div>

                  <div style={{ color: NOC_SURFACES.textSecondary, fontSize: "0.82rem", marginTop: "0.35rem" }}>
                    {branch.subject || "(no subject)"}
                  </div>

                  {/* Proportional two-tone diff bar */}
                  <div style={{ display: "inline-flex", alignItems: "center", gap: "0.55rem", marginTop: "0.45rem" }}>
                    {isInSync ? (
                      <span
                        style={{
                          fontSize: "0.72rem",
                          fontWeight: 700,
                          color: NOC_COLORS.ok,
                          background: "rgba(16, 185, 129, 0.15)",
                          padding: "0.1rem 0.4rem",
                          borderRadius: "4px",
                        }}
                      >
                        In sync with main
                      </span>
                    ) : (
                      <>
                        <span className="tabular-nums" style={{ color: NOC_COLORS.ok, fontSize: "0.78rem", fontWeight: 700 }}>
                          +{branch.ahead}
                        </span>
                        <span className="tabular-nums" style={{ color: NOC_COLORS.crit, fontSize: "0.78rem", fontWeight: 700 }}>
                          −{branch.behind}
                        </span>
                        <div
                          style={{
                            width: 64,
                            height: 6,
                            background: "rgba(148, 163, 184, 0.2)",
                            borderRadius: 3,
                            display: "flex",
                            overflow: "hidden",
                          }}
                        >
                          {branch.ahead > 0 && <div style={{ width: `${aheadPct}%`, background: NOC_COLORS.ok }} />}
                          {branch.behind > 0 && <div style={{ width: `${behindPct}%`, background: NOC_COLORS.crit }} />}
                        </div>
                        <span style={{ fontSize: "0.74rem", color: NOC_SURFACES.textMuted }}>vs main</span>
                      </>
                    )}
                  </div>
                </div>

                {/* Linked Issue Chip */}
                <div style={{ minWidth: 200, display: "flex", flexDirection: "column", gap: "0.35rem" }}>
                  {branch.issue_number ? (
                    <div
                      style={{
                        fontSize: "0.8rem",
                        padding: "0.3rem 0.55rem",
                        borderRadius: "6px",
                        background: "rgba(16, 185, 129, 0.1)",
                        border: "1px solid rgba(16, 185, 129, 0.25)",
                      }}
                    >
                      <div style={{ fontWeight: 700, color: NOC_COLORS.ok }}>
                        #{branch.issue_number}
                        {branch.issue_state ? ` · ${branch.issue_state}` : ""}
                        {branch.override_issue ? " · override" : ""}
                      </div>
                      <div style={{ color: NOC_SURFACES.textPrimary, fontSize: "0.76rem", marginTop: "0.15rem" }}>
                        {branch.issue_url ? (
                          <a href={branch.issue_url} style={{ color: "#7dd3fc", textDecoration: "none" }}>
                            {branch.issue_title || "linked issue"}
                          </a>
                        ) : (
                          branch.issue_title || "linked issue"
                        )}
                      </div>
                    </div>
                  ) : (
                    <div style={{ color: NOC_SURFACES.textMuted, fontSize: "0.76rem" }}>No linked issue</div>
                  )}

                  <form
                    onSubmit={(event) => {
                      event.preventDefault();
                      void saveOverride(branch.name, drafts[branch.name] ?? String(branch.issue_number ?? ""));
                    }}
                    style={{ display: "flex", gap: "0.35rem", alignItems: "center" }}
                  >
                    <input
                      inputMode="numeric"
                      className="noc-focusable"
                      placeholder="issue #"
                      value={drafts[branch.name] ?? String(branch.issue_number ?? "")}
                      onChange={(event) =>
                        setDrafts((prev) => ({ ...prev, [branch.name]: event.target.value }))
                      }
                      style={{
                        width: "80px",
                        padding: "0.25rem 0.45rem",
                        borderRadius: "6px",
                        border: "1px solid rgba(148, 163, 184, 0.2)",
                        background: "rgba(0,0,0,0.35)",
                        color: NOC_SURFACES.textPrimary,
                        fontSize: "0.75rem",
                      }}
                    />
                    <button
                      type="submit"
                      className="btn-secondary noc-focusable"
                      style={{ marginTop: 0, fontSize: "0.72rem", padding: "0.25rem 0.6rem" }}
                      disabled={busy}
                    >
                      Link
                    </button>
                  </form>
                </div>
              </div>
            </article>
          );
        })}

        {list && list.branches.length === 0 && (
          <EmptyStateChip
            title="No branches found"
            detail="No local or remote-tracking branches found for this repository."
            tone="idle"
          />
        )}
      </div>
    </div>
  );
}
