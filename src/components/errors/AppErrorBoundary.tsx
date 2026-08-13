import { Component, type ErrorInfo, type ReactNode } from "react";

interface AppErrorBoundaryProps {
  children: ReactNode;
}

interface AppErrorBoundaryState {
  hasError: boolean;
}

export default class AppErrorBoundary extends Component<AppErrorBoundaryProps, AppErrorBoundaryState> {
  state: AppErrorBoundaryState = { hasError: false };

  static getDerivedStateFromError(): AppErrorBoundaryState {
    return { hasError: true };
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    if (import.meta.env.DEV) {
      console.error("Coding-Assistants frontend render failure", error, errorInfo);
    }
  }

  render() {
    if (!this.state.hasError) {
      return this.props.children;
    }

    return (
      <main
        role="alert"
        aria-live="assertive"
        style={{
          minHeight: "100vh",
          display: "grid",
          placeItems: "center",
          padding: "1.5rem",
          background: "var(--bg-primary, #020617)",
          color: "var(--text-primary, #e2e8f0)",
        }}
      >
        <section className="glass-card" style={{ width: "min(520px, 100%)", textAlign: "center" }}>
          <p style={{ color: "var(--text-muted)", marginBottom: "0.6rem" }}>⚠ Application recovery</p>
          <h1 style={{ marginBottom: "0.75rem" }}>Coding-Assistants needs to reload</h1>
          <p style={{ color: "var(--text-muted)", lineHeight: 1.55 }}>
            A screen could not be rendered. Your local Hub data has not been changed.
          </p>
          <button
            type="button"
            className="btn-primary"
            onClick={() => window.location.reload()}
            style={{ marginTop: "1.25rem" }}
          >
            Reload application
          </button>
        </section>
      </main>
    );
  }
}
