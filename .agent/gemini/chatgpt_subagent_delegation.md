---
### SUBAGENT DELEGATION PROTOCOL: CHATGPT

**Identity & Capability:**
You are equipped to launch a ChatGPT AI subagent via the `chatgpt` command. ChatGPT executes statelessly and has no awareness of this current chat session.

**When to Delegate:**
Invoke the ChatGPT subagent for:
*   **Protocol Conceptualization:** Designing new agent-to-agent authorization or handoff logic for `AgentSystem`.
*   **Documentation & Abstraction:** Generating clear, high-level summaries of the agent/IPC/TCP-server interaction flow.
*   **Alternative Paradigms:** Asking for a completely different approach when the current design is stuck.

**Execution Syntax:**
Execute the command in your terminal environment. Always enclose the prompt in single quotes.
`chatgpt 'YOUR_COMPREHENSIVE_PROMPT_HERE'`

**Subagent Prompting Rules (How to talk to ChatGPT):**
1.  **Context Injection:** Paste all relevant snippets and constraints into the prompt.
2.  **Structured Output:** Use the Template Pattern. Define exactly how the output should look using a mock structure.
3.  **Chain-of-Thought:** For complex logic, explicitly ask ChatGPT to "Think step-by-step before providing the final answer."

**Example Usage:**
`chatgpt 'Act as a distributed-systems specialist. I need to design an authorization scheme for [[ASK_AGENT:RoleName]] handoffs between agents so one role cannot impersonate another. Think step-by-step about failure modes. Output format:
## Reasoning: [Step-by-step thoughts]
## Authorization Scheme: [Bullet points]'`

**Failure Modes to Avoid:**
*   **Do not** nest quotes improperly (e.g., `chatgpt 'He said 'hello''`).
*   **Do not** use ambiguous instructions; be explicit about the domain.
---