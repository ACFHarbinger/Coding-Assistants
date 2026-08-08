---
### SUBAGENT DELEGATION PROTOCOL: GEMINI

**Identity & Capability:**
You have the authority to spawn a Gemini AI subagent via the terminal using the `agy` CLI command. Gemini operates independently, statelessly, and processes large contexts with high efficiency.

**When to Delegate:**
Invoke the Gemini subagent for:
*   **Log/Data Parsing:** Standardizing complex data transformations or parsing large agent-session log files.
*   **Cross-Boundary Boilerplate:** Drafting serde structs on the Rust side that mirror TypeScript IPC payload types, or vice versa.
*   **System Architecture:** Extracting structured metrics or designing usage/error dashboards from raw agent execution logs.

**Execution Syntax:**
Execute the command in your terminal. Ensure the prompt is enclosed in single quotes.
`agy 'YOUR_COMPREHENSIVE_PROMPT_HERE'`

**Subagent Prompting Rules (How to talk to Gemini):**
1.  **Explicit Context:** Provide all required schemas, data samples, and environmental constraints (e.g., Linux, KDE, specific GPU hardware).
2.  **Template Pattern:** Dictate the exact output structure using a template to ensure the response can be easily parsed or piped into another tool.
3.  **Action-Oriented Verbs:** Start instructions with clear directives like "Analyze," "Generate," or "Extract."

**Example Usage:**
`agy 'Act as an expert TypeScript/React developer. Generate a React component for displaying a streaming agent response in App.tsx, matching the existing glass-morphism CSS custom properties in index.css. Constraints: 1. Use function components with hooks. 2. Output only the TSX component. Context: [INSERT_CURRENT_UI_STATE]'`

**Failure Modes to Avoid:**
*   **Do not** use unescaped single quotes in the `agy` execution string.
*   **Do not** expect Gemini to read files from the disk automatically unless you ask it to generate the shell commands to do so.
---