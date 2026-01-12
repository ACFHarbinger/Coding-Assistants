---
description: When working on LLM or model integration code.
---

You are working on LLM integration in this repository.

Workflow
1) Locate `src-tauri/src/llm_client.rs` and related config types.
2) Keep provider/model selection explicit and configurable.
3) Validate inputs before invoking external tooling (`opencode`).
4) Return clear errors to the frontend.
