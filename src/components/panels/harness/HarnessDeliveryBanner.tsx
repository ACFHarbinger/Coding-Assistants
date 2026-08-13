import { injectNotice, type HarnessDeliveryNotice } from "./types";

export default function HarnessDeliveryBanner({
  notices,
  onRetry,
  onDismiss,
}: {
  notices: HarnessDeliveryNotice[];
  onRetry: (notice: HarnessDeliveryNotice) => void;
  onDismiss: (harness: string) => void;
}) {
  if (notices.length === 0) return null;
  return (
    <div style={{ display: "grid", gap: "0.45rem", padding: "0.65rem 1rem", borderBottom: "1px solid var(--border-color)", background: "rgba(2, 6, 23, 0.72)" }}>
      {notices.map((notice) => {
        const tone = injectNotice(notice.status, notice.detail);
        const colors = tone.tone === "ok"
          ? { bg: "rgba(6, 78, 59, 0.45)", border: "rgba(52, 211, 153, 0.7)", color: "#bbf7d0" }
          : tone.tone === "warn"
            ? { bg: "rgba(120, 53, 15, 0.5)", border: "rgba(251, 191, 36, 0.8)", color: "#fde68a" }
            : { bg: "rgba(127, 29, 29, 0.5)", border: "rgba(248, 113, 113, 0.8)", color: "#fecaca" };
        return (
          <div key={`${notice.harness}:${notice.status}:${notice.detail}`} style={{ ...colors, border: `1px solid ${colors.border}`, borderRadius: "8px", padding: "0.5rem 0.7rem", fontSize: "0.8rem", display: "flex", justifyContent: "space-between", gap: "0.75rem", flexWrap: "wrap" }}>
            <div>
              <strong style={{ textTransform: "uppercase" }}>{notice.harness} · {notice.status}</strong>
              <div>{notice.detail}</div>
            </div>
            <div style={{ display: "flex", gap: "0.4rem" }}>
              {notice.retryable && (
                <button type="button" className="btn-primary" style={{ marginTop: 0, padding: "0.25rem 0.6rem", fontSize: "0.75rem" }} onClick={() => onRetry(notice)}>
                  Retry
                </button>
              )}
              <button
                type="button"
                className="btn-secondary"
                style={{ marginTop: 0, padding: "0.25rem 0.6rem", fontSize: "0.75rem" }}
                title="Hides this notice only. Does not release a writer lease or cancel the provider session."
                onClick={() => onDismiss(notice.harness)}
              >
                Dismiss
              </button>
            </div>
          </div>
        );
      })}
    </div>
  );
}
