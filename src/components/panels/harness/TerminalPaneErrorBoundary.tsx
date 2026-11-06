import { Component, type ErrorInfo, type ReactNode } from "react";

/** Keeps an xterm/PTY render failure from replacing the whole app (#143). */
export default class TerminalPaneErrorBoundary extends Component<
  { children: ReactNode },
  { error: string | null }
> {
  state: { error: string | null } = { error: null };

  static getDerivedStateFromError(error: Error): { error: string } {
    return { error: error.message || "Terminal view failed to render" };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("Embedded terminal render failure", error, info);
  }

  render() {
    if (!this.state.error) {
      return this.props.children;
    }
    return (
      <div
        role="alert"
        style={{
          minHeight: "220px",
          padding: "1rem",
          color: "#fecaca",
          fontSize: "0.85rem",
          lineHeight: 1.45,
        }}
      >
        <strong>In-app terminal crashed.</strong>
        <div style={{ color: "var(--text-muted)", marginTop: "0.4rem" }}>{this.state.error}</div>
        <div style={{ color: "var(--text-muted)", marginTop: "0.35rem" }}>
          The rest of the app is still running. Close this pane and try Resume again.
        </div>
      </div>
    );
  }
}
