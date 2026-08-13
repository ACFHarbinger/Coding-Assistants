import { Link, useLocation } from "react-router-dom";

/**
 * Custom 404 recovery page (roadmap `*` route). HashRouter means a bad path
 * never round-trips to a server, so this has to be a real in-app route, not
 * a static host-level error page.
 */
export function NotFoundPage() {
  const location = useLocation();
  const attemptedPath = `${location.pathname}${location.search}${location.hash}`;

  return (
    <div className="relative overflow-hidden">
      <div
        aria-hidden="true"
        className="pointer-events-none absolute inset-0 motion-reduce:hidden"
        style={{
          background:
            "radial-gradient(circle at 0% 0%, rgba(99, 102, 241, 0.14) 0%, transparent 46%), radial-gradient(circle at 100% 100%, rgba(168, 85, 247, 0.12) 0%, transparent 48%)",
        }}
      />
      <section className="relative mx-auto flex min-h-[60vh] max-w-2xl flex-col items-center justify-center px-4 py-20 text-center">
        <p className="text-sm font-semibold uppercase tracking-[0.18em] text-indigo-300">
          404
        </p>
        <h1 className="mt-3 text-4xl font-extrabold tracking-tight text-[var(--text-primary)] sm:text-5xl">
          Page not found
        </h1>
        <p className="mt-5 max-w-lg text-lg leading-relaxed text-[var(--text-secondary)]">
          There's no page at{" "}
          <code className="rounded bg-[var(--glass-bg)] px-1.5 py-0.5 font-mono text-sm text-[var(--text-primary)]">
            {attemptedPath || "/"}
          </code>
          . It may have moved, or the link might be out of date.
        </p>
        <p className="mt-2 text-sm text-[var(--text-muted)]">
          Press <kbd className="rounded border border-[var(--glass-border)] bg-[var(--glass-bg)] px-1.5 py-0.5 font-mono text-xs">⌘K</kbd>{" "}
          / <kbd className="rounded border border-[var(--glass-border)] bg-[var(--glass-bg)] px-1.5 py-0.5 font-mono text-xs">Ctrl K</kbd>{" "}
          to search the documentation.
        </p>
        <div className="mt-8 flex flex-wrap justify-center gap-3">
          <Link
            to="/"
            className="rounded-lg bg-indigo-500 px-5 py-2.5 text-sm font-semibold text-white shadow-lg shadow-indigo-500/20 hover:bg-indigo-600 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-400"
          >
            Go to homepage
          </Link>
          <Link
            to="/docs"
            className="rounded-lg border border-[var(--glass-border)] bg-[var(--glass-bg)] px-5 py-2.5 text-sm font-semibold text-[var(--text-primary)] hover:bg-indigo-500/10 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-purple-400"
          >
            Browse the docs
          </Link>
          <a
            href="https://github.com/ACFHarbinger/Coding-Assistants"
            target="_blank"
            rel="noopener noreferrer"
            className="rounded-lg border border-[var(--glass-border)] bg-[var(--glass-bg)] px-5 py-2.5 text-sm font-semibold text-[var(--text-primary)] hover:bg-indigo-500/10 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-purple-400"
          >
            GitHub repository
          </a>
        </div>
      </section>
    </div>
  );
}
