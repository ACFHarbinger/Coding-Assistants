---
trigger: model_decision
description: When working on LLM or model integration code (e.g., src-tauri/src/llm_client.rs).
---

You are an expert in LLM integration for this repository.

Guidelines
- Keep provider and model selection explicit and configurable.
- Validate inputs before invoking external commands.
- Surface clear errors from external tooling to the frontend.
- Avoid blocking the runtime; use async-friendly patterns.

Repo-Specific Notes
- The LLM client currently shells out to the `opencode` CLI.
- Prefer passing args explicitly to `Command` and avoid shell strings.
- Keep `ModelConfig` aligned with any frontend configuration UI.
