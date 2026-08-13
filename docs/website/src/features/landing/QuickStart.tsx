const SNIPPET = `git clone https://github.com/ACFHarbinger/Coding-Assistants.git
cd Coding-Assistants
npm install
npm run tauri dev`;

export function QuickStart() {
  return (
    <section aria-labelledby="quickstart-heading" className="mx-auto max-w-6xl px-4 py-8">
      <div className="grid items-start gap-8 lg:grid-cols-2">
        <div>
          <h2 id="quickstart-heading" className="text-2xl font-bold text-slate-100">
            Run it locally
          </h2>
          <p className="mt-2 text-slate-400">
            Desktop app first. The website you are reading is the public docs surface, not the product runtime.
          </p>
          <ol className="mt-6 list-decimal space-y-2 pl-5 text-sm text-slate-300">
            <li>Install Node, Rust, and the agent CLIs you want (Grok, Codex, Claude, Gemini).</li>
            <li>Open a workspace and create or load a team chat from Orchestrate.</li>
            <li>Address agents and mark posts task and/or wake from Messager.</li>
          </ol>
        </div>
        <pre className="overflow-x-auto rounded-2xl border border-white/10 bg-[#020617] p-5 font-mono text-sm leading-7 text-indigo-100">
          <code>{SNIPPET}</code>
        </pre>
      </div>
    </section>
  );
}
