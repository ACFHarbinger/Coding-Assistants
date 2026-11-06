# Glossary

| Term | Meaning |
| --- | --- |
| ADR | Architecture Decision Record — a short document capturing a significant, hard-to-reverse technical decision and its rationale. |
| C4 Model | A layered way to diagram software architecture: Context, Container, Component, Code. |
| Agent | An LLM coding-assistant role orchestrated by `AgentSystem` (`src-tauri/src/agent/orchestrator.rs`), backed by a CLI or API-based LLM provider. |
| IPC | The Tauri `invoke()` boundary between the React frontend (`src/`) and the Rust backend (`src-tauri/`). |
| Recipe | A named command defined in the root `justfile`. |
