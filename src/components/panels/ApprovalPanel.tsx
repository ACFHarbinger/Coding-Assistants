
import { useEffect } from "react";
import { NOC_COLORS, NOC_ELEVATION, NOC_RADIUS, NOC_SURFACES } from "../common/telemetry/tokens";
import { nocPanelStyle, nocSubsurfaceStyle } from "../common/telemetry/panel";

interface ApprovalPanelProps {
  authorizationRequest: { role: string; question: string } | null;
  respondToAuthorization: (approved: boolean) => Promise<void>;
  currentQuestion: string | null;
  userInput: string;
  setUserInput: (val: string) => void;
  submitAnswer: () => Promise<void>;
}

export default function ApprovalPanel({
  authorizationRequest,
  respondToAuthorization,
  currentQuestion,
  userInput,
  setUserInput,
  submitAnswer,
}: ApprovalPanelProps) {
  useEffect(() => {
    if (!authorizationRequest && !currentQuestion) return;
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        if (authorizationRequest) void respondToAuthorization(false);
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [authorizationRequest, currentQuestion, respondToAuthorization]);

  if (!authorizationRequest && !currentQuestion) return null;

  return (
    <>
      {authorizationRequest && (
        <div
          role="dialog"
          aria-modal="true"
          aria-labelledby="auth-req-title"
          style={{
            position: "fixed",
            top: 0,
            left: 0,
            right: 0,
            bottom: 0,
            background: "rgba(9, 14, 26, 0.82)",
            backdropFilter: "blur(8px)",
            WebkitBackdropFilter: "blur(8px)",
            display: "flex",
            justifyContent: "center",
            alignItems: "center",
            zIndex: 2000,
            padding: "1rem",
          }}
        >
          <div
            className="fade-in"
            style={{
              ...nocPanelStyle,
              maxWidth: "540px",
              width: "100%",
              boxShadow: NOC_ELEVATION.elev3,
              border: `1px solid ${NOC_SURFACES.borderStrong}`,
              padding: "2rem",
            }}
          >
            <h2
              id="auth-req-title"
              style={{
                marginTop: 0,
                color: NOC_SURFACES.textPrimary,
                fontSize: "1.3rem",
                display: "flex",
                alignItems: "center",
                gap: "0.6rem",
                fontWeight: 700,
              }}
            >
              <span style={{ fontSize: "1.4rem" }}>🛡️</span> Authorization Required
            </h2>
            <div style={{ margin: "1.5rem 0", fontSize: "0.95rem", lineHeight: "1.6" }}>
              <p style={{ color: NOC_SURFACES.textMuted, margin: 0 }}>
                An agent wants to ask a question to the{" "}
                <strong style={{ color: NOC_COLORS.cyan, fontWeight: 600 }}>{authorizationRequest.role}</strong>:
              </p>
              <div
                style={{
                  ...nocSubsurfaceStyle,
                  fontStyle: "italic",
                  marginTop: "0.85rem",
                  borderLeft: `4px solid ${NOC_COLORS.cyan}`,
                  color: NOC_SURFACES.textPrimary,
                }}
              >
                &ldquo;{authorizationRequest.question}&rdquo;
              </div>
            </div>
            <div style={{ display: "flex", justifyContent: "flex-end", gap: "0.75rem" }}>
              <button
                type="button"
                className="btn-secondary noc-focusable"
                style={{ borderColor: "rgba(239, 68, 68, 0.5)", color: NOC_COLORS.crit, padding: "0.55rem 1.25rem" }}
                onClick={() => void respondToAuthorization(false)}
              >
                Deny Request
              </button>
              <button
                type="button"
                className="btn-primary noc-focusable"
                style={{ padding: "0.55rem 1.25rem" }}
                onClick={() => void respondToAuthorization(true)}
              >
                Approve Request
              </button>
            </div>
          </div>
        </div>
      )}

      {currentQuestion && (
        <div
          role="dialog"
          aria-modal="true"
          aria-labelledby="agent-input-title"
          style={{
            position: "fixed",
            top: 0,
            left: 0,
            right: 0,
            bottom: 0,
            background: "rgba(9, 14, 26, 0.82)",
            backdropFilter: "blur(8px)",
            WebkitBackdropFilter: "blur(8px)",
            display: "flex",
            justifyContent: "center",
            alignItems: "center",
            zIndex: 2000,
            padding: "1rem",
          }}
        >
          <div
            className="fade-in"
            style={{
              ...nocPanelStyle,
              maxWidth: "620px",
              width: "100%",
              boxShadow: NOC_ELEVATION.elev3,
              border: `1px solid ${NOC_SURFACES.borderStrong}`,
              padding: "2rem",
            }}
          >
            <h2
              id="agent-input-title"
              style={{
                marginTop: 0,
                color: NOC_SURFACES.textPrimary,
                fontSize: "1.3rem",
                display: "flex",
                alignItems: "center",
                gap: "0.6rem",
                fontWeight: 700,
              }}
            >
              <span style={{ fontSize: "1.4rem" }}>❓</span> Agent Needs Input
            </h2>
            <p style={{ fontSize: "0.95rem", lineHeight: "1.6", margin: "1.25rem 0", color: NOC_SURFACES.textMuted }}>
              {currentQuestion}
            </p>
            <textarea
              value={userInput}
              onChange={(e) => setUserInput(e.target.value)}
              placeholder="Type your answer here..."
              rows={5}
              className="noc-focusable"
              style={{
                width: "100%",
                background: NOC_SURFACES.bgDeep,
                border: `1px solid ${NOC_SURFACES.border}`,
                borderRadius: NOC_RADIUS.s,
                padding: "1rem",
                color: NOC_SURFACES.textPrimary,
                fontSize: "0.95rem",
                marginBottom: "1.25rem",
                resize: "vertical",
                boxSizing: "border-box",
              }}
              autoFocus
            />
            <div style={{ display: "flex", justifyContent: "flex-end", gap: "0.75rem" }}>
              <button
                type="button"
                className="btn-primary noc-focusable"
                style={{ padding: "0.65rem 1.75rem" }}
                onClick={() => void submitAnswer()}
              >
                Submit Answer
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
