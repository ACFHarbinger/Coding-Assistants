const CAPABILITIES = [
  {
    title: "Local-first hub",
    body: "Team chat, memory, and wakes live in a SQLite hub on your machine. Nothing is last-writer-wins in the cloud.",
  },
  {
    title: "Multi-agent orchestration",
    body: "Address one teammate, a subset, or the whole roster. Tag a post as a task, a wake, or both.",
  },
  {
    title: "Harness capture",
    body: "Transcripts from Grok, Codex, Claude, and Gemini land back in the same work session.",
  },
  {
    title: "Explicit delivery",
    body: "Wakes may start a CLI. Tasks stay queued until a documented active-session bridge can deliver them.",
  },
  {
    title: "Usage you can see",
    body: "Shared Hub plots provider quota remaining so a runaway loop is visible before it burns the week.",
  },
  {
    title: "Your workspace, your keys",
    body: "Providers are local CLIs. Coding Assistants does not ship or hardcode API secrets.",
  },
] as const;

export function CapabilityGrid() {
  return (
    <section aria-labelledby="capabilities-heading" className="mx-auto max-w-6xl px-4 py-16">
      <h2 id="capabilities-heading" className="text-2xl font-bold tracking-tight text-slate-100">
        What the desktop app actually does
      </h2>
      <p className="mt-2 max-w-2xl text-slate-400">
        A glass-morphism control surface for a human owner and the coding agents already on the machine.
      </p>
      <ul className="mt-8 grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
        {CAPABILITIES.map((item) => (
          <li
            key={item.title}
            className="rounded-2xl border border-white/10 bg-[rgba(15,23,42,0.92)] p-5 shadow-xl backdrop-blur-[20px] motion-reduce:backdrop-blur-none"
          >
            <h3 className="text-base font-semibold text-indigo-300">{item.title}</h3>
            <p className="mt-2 text-sm leading-relaxed text-slate-400">{item.body}</p>
          </li>
        ))}
      </ul>
    </section>
  );
}
