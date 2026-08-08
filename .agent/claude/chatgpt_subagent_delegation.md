---
### SUBAGENT DELEGATION PROTOCOL: CHATGPT

**Identity & Capability:**
You can orchestrate a ChatGPT AI subagent via your local terminal using the `chatgpt` CLI command. ChatGPT acts as a stateless, highly capable reasoning engine. It does not share your context window.

**When to Delegate:**
Invoke the ChatGPT subagent for:
*   **Protocol Design:** Drafting formal definitions for the agent communication marker protocol (`[[ASK_USER]]`, `[[ASK_AGENT:RoleName]]`).
*   **Provider Research:** Summarizing a new LLM provider's API surface before wiring it into `llm_client.rs`.
*   **Creative Brainstorming:** Generating varied architectural approaches before you commit to writing the implementation code.

**Execution Syntax:**
Run the command in your shell, wrapping the prompt in strong quotes to prevent shell evaluation errors.
`chatgpt 'YOUR_COMPREHENSIVE_PROMPT_HERE'`

**Subagent Prompting Rules (How to talk to ChatGPT):**
1.  **Zero-Shot Context:** You MUST include all necessary definitions, constraints, and current state.
2.  **Constraint Pattern:** Explicitly list what ChatGPT must *not* do to keep the response focused and token-efficient.
3.  **Role Definition:** Always assign ChatGPT a clear persona (e.g., "Act as a PhD-level Operations Research scientist").

**Example Usage:**
`chatgpt 'Act as an API integration specialist. Summarize the authentication flow and streaming response format for the <provider> chat completions API, in terms directly usable to extend llm_client.rs which already supports OpenAI-style streaming. Rules: 1. Focus only on auth and streaming. 2. Output a short bullet list, not prose.'`

**Failure Modes to Avoid:**
*   **Do not** use unescaped single quotes inside the `chatgpt` command string.
*   **Do not** assume ChatGPT knows our current project state.
*   **Do not** delegate tasks requiring direct file manipulation.
---