import React from 'react';
import { Link } from 'react-router-dom';
import {
  Terminal,
  Cpu,
  Bot,
  Zap,
  Shield,
  ArrowRight,
  CheckCircle2,
  Code2,
  Workflow,
  Sparkles,
} from 'lucide-react';

export const LandingPage: React.FC = () => {
  const features = [
    {
      icon: <Bot className="w-6 h-6 text-cyan-400" />,
      title: 'Multi-Agent Team Orchestration',
      description:
        'Coordinate OpenAI Codex, Anthropic Claude Code, xAI Grok, and Google Antigravity CLI in one unified, local-first team session.',
    },
    {
      icon: <Cpu className="w-6 h-6 text-violet-400" />,
      title: 'Local-First Hub & Memory',
      description:
        'SQLite-backed shared memory store (`ca-hub`) with durable memory promotion, age-out retention, and audit trailing.',
    },
    {
      icon: <Zap className="w-6 h-6 text-amber-400" />,
      title: 'Bidirectional Harness Adapters',
      description:
        'Direct process spawning & active-session IPC sockets (`grok`, `codex`, `claude`, `agy`) with SHA-256 deduplicated transcript capture.',
    },
    {
      icon: <Workflow className="w-6 h-6 text-emerald-400" />,
      title: 'Intent Tags & Addressing',
      description:
        'Precision message addressing (`🌐 All Team`, `👥 Subset`, `🎯 Single Agent`) and intent tags (`⚡ [TASK]`, `🔔 [WAKE]`).',
    },
    {
      icon: <Shield className="w-6 h-6 text-blue-400" />,
      title: 'Budget & Policy Control',
      description:
        'Per-agent budget enforcement, automatic exhaustion pause, durable Markdown handoffs, and human-in-the-loop wake policy gates.',
    },
    {
      icon: <Code2 className="w-6 h-6 text-rose-400" />,
      title: 'Rust & React 19 Architecture',
      description:
        'High-performance Tauri 2 desktop app pairing Rust backend efficiency with React 19 glassmorphism UI elegance.',
    },
  ];

  return (
    <div className="space-y-24 pb-20">
      <section className="relative pt-20 pb-16 overflow-hidden">
        <div className="absolute top-1/4 left-1/2 -translate-x-1/2 -translate-y-1/2 w-96 h-96 bg-gradient-to-tr from-cyan-500/20 to-violet-600/20 rounded-full blur-3xl pointer-events-none" />

        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 text-center relative z-10">
          <div className="inline-flex items-center space-x-2 px-3.5 py-1.5 rounded-full bg-cyan-950/60 border border-cyan-800/50 text-cyan-300 text-xs font-semibold mb-8 backdrop-blur-md">
            <Sparkles className="w-3.5 h-3.5 text-cyan-400" />
            <span>v0.1.0 Hub-Native Release Available</span>
          </div>

          <h1 className="text-4xl sm:text-6xl font-extrabold text-slate-100 tracking-tight max-w-4xl mx-auto leading-tight">
            Local-First Multi-Agent <br />
            <span className="bg-gradient-to-r from-cyan-400 via-teal-300 to-violet-400 bg-clip-text text-transparent">
              Coding Assistant Orchestration
            </span>
          </h1>

          <p className="mt-6 text-lg sm:text-xl text-slate-400 max-w-2xl mx-auto leading-relaxed">
            Orchestrate Anthropic Claude, OpenAI Codex, xAI Grok, and Google Antigravity CLI side-by-side with durable local memory, recipient addressing, and bidirectional process capture.
          </p>

          <div className="mt-10 flex flex-col sm:flex-row items-center justify-center gap-4">
            <Link
              to="/docs"
              className="w-full sm:w-auto px-8 py-3.5 rounded-xl bg-gradient-to-r from-cyan-500 to-violet-600 text-white font-semibold shadow-lg shadow-cyan-500/25 hover:shadow-cyan-500/40 hover:scale-[1.02] transition-all flex items-center justify-center space-x-2"
            >
              <span>Explore Documentation</span>
              <ArrowRight className="w-4 h-4" />
            </Link>
            <a
              href="https://github.com/ACFHarbinger/Coding-Assistants"
              target="_blank"
              rel="noopener noreferrer"
              className="w-full sm:w-auto px-8 py-3.5 rounded-xl glass-panel text-slate-200 hover:text-white hover:bg-slate-800/80 font-semibold transition-all flex items-center justify-center space-x-2"
            >
              <Terminal className="w-4 h-4 text-cyan-400" />
              <span>View Source on GitHub</span>
            </a>
          </div>
        </div>
      </section>

      <section className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
        <div className="glass-panel rounded-2xl p-8 border border-slate-800/80 bg-slate-900/60 shadow-2xl relative overflow-hidden">
          <div className="flex flex-col md:flex-row items-center justify-between gap-8">
            <div className="space-y-4 max-w-md">
              <span className="text-xs font-semibold text-cyan-400 uppercase tracking-wider">
                Architecture Overview
              </span>
              <h2 className="text-2xl sm:text-3xl font-bold text-slate-100">
                Hub-Native Desktop Spine
              </h2>
              <p className="text-slate-400 text-sm leading-relaxed">
                Coding-Assistants pairs Tauri 2 and Rust backend efficiency with a React 19 frontend. System calls, process monitoring, LLM SDKs, and SQLite hub storage live securely in Rust, exposing typed commands over Tauri IPC.
              </p>
              <div className="space-y-2 pt-2">
                <div className="flex items-center space-x-2 text-xs text-slate-300">
                  <CheckCircle2 className="w-4 h-4 text-cyan-400 flex-shrink-0" />
                  <span>IPC Commands & Tauri Event Streaming</span>
                </div>
                <div className="flex items-center space-x-2 text-xs text-slate-300">
                  <CheckCircle2 className="w-4 h-4 text-cyan-400 flex-shrink-0" />
                  <span>SQLite Hub Memory & Audit Event Hash Chains</span>
                </div>
                <div className="flex items-center space-x-2 text-xs text-slate-300">
                  <CheckCircle2 className="w-4 h-4 text-cyan-400 flex-shrink-0" />
                  <span>On-Disk Harness Session Transcript Parsing</span>
                </div>
              </div>
            </div>

            <div className="w-full md:w-1/2 p-4 rounded-xl bg-slate-950 border border-slate-800 font-mono text-xs text-slate-300 space-y-2">
              <div className="text-slate-500 border-b border-slate-800 pb-2 flex items-center justify-between">
                <span>ca-hub :: harness_bridge.rs</span>
                <span className="text-cyan-400">ACTIVE</span>
              </div>
              <p><span className="text-violet-400">Grok:</span> grok --cwd /repo (ACP leader.sock)</p>
              <p><span className="text-cyan-400">Claude:</span> claude -p &quot;[TASK] review PR&quot;</p>
              <p><span className="text-emerald-400">Codex:</span> codex exec --cwd /repo</p>
              <p><span className="text-amber-400">Gemini:</span> agy agent --bridge-socket bridge.sock</p>
            </div>
          </div>
        </div>
      </section>

      <section className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
        <div className="text-center space-y-3 mb-12">
          <h2 className="text-3xl font-bold text-slate-100">Key Capabilities</h2>
          <p className="text-slate-400 text-sm max-w-xl mx-auto">
            Everything you need to orchestrate autonomous coding agents locally on your machine.
          </p>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
          {features.map((f, i) => (
            <div key={i} className="glass-card p-6 rounded-xl space-y-3">
              <div className="p-2.5 rounded-lg bg-slate-900/80 border border-slate-800 w-fit">
                {f.icon}
              </div>
              <h3 className="text-base font-semibold text-slate-100">{f.title}</h3>
              <p className="text-xs text-slate-400 leading-relaxed">{f.description}</p>
            </div>
          ))}
        </div>
      </section>
    </div>
  );
};
