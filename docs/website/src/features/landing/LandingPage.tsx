import { Link } from "react-router-dom";
import { ArchitectureGraphic } from "./ArchitectureGraphic";
import { CapabilityGrid } from "./CapabilityGrid";
import { QuickStart } from "./QuickStart";

export function LandingPage() {
  return (
    <div className="relative overflow-hidden">
      <div
        aria-hidden="true"
        className="pointer-events-none absolute inset-0 motion-reduce:hidden"
        style={{
          background:
            "radial-gradient(circle at 0% 0%, rgba(99, 102, 241, 0.18) 0%, transparent 46%), radial-gradient(circle at 100% 100%, rgba(168, 85, 247, 0.16) 0%, transparent 48%)",
        }}
      />

      <section className="relative mx-auto grid max-w-6xl items-center gap-10 px-4 py-16 lg:grid-cols-2 lg:py-24">
        <div>
          <p className="text-sm font-semibold uppercase tracking-[0.18em] text-indigo-300">
            Local-first · desktop
          </p>
          <h1 className="mt-3 text-4xl font-extrabold tracking-tight text-[var(--text-primary)] sm:text-5xl">
            Coding-Assistants
          </h1>
          <p className="mt-5 max-w-xl text-lg leading-relaxed text-[var(--text-secondary)]">
            A Slack-like hub on your machine for you and the coding agents you already run.
            Grok, Codex, Claude, and Gemini stay in one work session — no markdown bus required.
          </p>
          <div className="mt-8 flex flex-wrap gap-3">
            <Link
              to="/docs"
              className="rounded-lg bg-indigo-500 px-5 py-2.5 text-sm font-semibold text-white shadow-lg shadow-indigo-500/20 hover:bg-indigo-600 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-400"
            >
              Read the docs
            </Link>
            <a
              href="https://github.com/ACFHarbinger/Coding-Assistants"
              className="rounded-lg border border-[var(--glass-border)] bg-[var(--glass-bg)] px-5 py-2.5 text-sm font-semibold text-[var(--text-primary)] hover:bg-indigo-500/10 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-purple-400"
            >
              View GitHub
            </a>
          </div>
        </div>
        <ArchitectureGraphic className="mx-auto w-full max-w-md" />
      </section>

      <CapabilityGrid />
      <QuickStart />

      <section aria-labelledby="workflow-heading" className="mx-auto max-w-6xl px-4 pb-20">
        <h2 id="workflow-heading" className="text-2xl font-bold text-[var(--text-primary)]">
          The v1 loop
        </h2>
        <ol className="mt-6 grid gap-4 md:grid-cols-3">
          <li className="rounded-2xl border border-[var(--glass-border)] bg-[var(--glass-bg)] p-5">
            <p className="text-xs font-semibold uppercase tracking-wide text-purple-300">1. Session</p>
            <p className="mt-2 text-sm text-[var(--text-secondary)]">Create or load a named team chat. The roster is explicit.</p>
          </li>
          <li className="rounded-2xl border border-[var(--glass-border)] bg-[var(--glass-bg)] p-5">
            <p className="text-xs font-semibold uppercase tracking-wide text-purple-300">2. Address</p>
            <p className="mt-2 text-sm text-[var(--text-secondary)]">All, a subset, or one member. Mark task and/or wake.</p>
          </li>
          <li className="rounded-2xl border border-[var(--glass-border)] bg-[var(--glass-bg)] p-5">
            <p className="text-xs font-semibold uppercase tracking-wide text-purple-300">3. Capture</p>
            <p className="mt-2 text-sm text-[var(--text-secondary)]">Harness replies return to the same transcript. Tasks queue if no bridge.</p>
          </li>
        </ol>
      </section>
    </div>
  );
}
