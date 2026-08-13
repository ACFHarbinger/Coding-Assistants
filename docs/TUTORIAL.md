# Tutorial

[![Tauri](https://img.shields.io/badge/Tauri-2-24C8D8?logo=tauri&logoColor=white)](https://v2.tauri.app)
[![React](https://img.shields.io/badge/React-19-61DAFB?logo=react&logoColor=black)](https://react.dev)

Step-by-step guide to using Coding Assistants, from first launch to advanced multi-agent workflows.

---

## Table of Contents

1. [First Launch](#1-first-launch)
2. [Configuring Your First Agent](#2-configuring-your-first-agent)
3. [Running a Simple Task](#3-running-a-simple-task)
4. [Multi-Agent Workflows](#4-multi-agent-workflows)
5. [Custom Prompts, Rules, and Workflows](#5-custom-prompts-rules-and-workflows)
6. [Using the Remote Control](#6-using-the-remote-control)
7. [MCP Configuration](#7-mcp-configuration)
8. [Working with Local Models](#8-working-with-local-models)

---

## 1. First Launch

### Start the Application

```bash
cd Coding-Assistants
npm run tauri dev
```

The application window opens with a dark glass-morphism interface. You'll see:

- **Configuration section** at the top -- where you set up agent roles
- **Execute Task section** -- where you describe what you want done
- **Remote Control section** -- for Android app connectivity
- **Agent Activity** -- appears during/after task execution
- **Final Output** -- shows the completed result

### Initial State

On first launch, you'll have a single agent role called "Planner" configured. No workspace is selected and no task has been run.

---

## 2. Configuring Your First Agent

### Select a Provider

Click the **Provider** dropdown for the default role. Choose from:

| Provider         | Requirements                              |
| ---------------- | ----------------------------------------- |
| OpenCode Zen     | `opencode` CLI installed                  |
| DeepSeek         | `opencode` CLI; `opencode models` lists `deepseek/*` |
| Mistral (Vibe)   | `vibe` CLI installed; run `vibe --setup`  |
| Google           | `GOOGLE_GENAI_API_KEY` in `env/.env`      |
| Anthropic        | API key configured                        |
| OpenAI           | `OPENAI_API_KEY` in `env/.env`            |
| GitHub Copilot   | GitHub Copilot subscription               |
| Ollama           | `ollama` running with pulled models       |
| LM Studio        | LM Studio running                         |

### Select a Model

After choosing a provider, the **Model** dropdown populates with available models. Select the model you want this role to use.

### Choose a Workspace

Click the **folder icon** next to "Workspace" to open a directory picker. Select the project directory you want agents to work with. This directory is where:

- Agents read/write files
- The `.agent/` folder is scanned for custom prompts, rules, and workflows
- Project context is gathered

### Name the Role

The default role name is "Planner". You can change it to anything descriptive:
- "Architect" -- for high-level design
- "Developer" -- for implementation
- "Reviewer" -- for code review
- "Tester" -- for test generation

---

## 3. Running a Simple Task

### Write a Task Description

In the **Execute Task** section, type a clear description of what you want the agent to do:

```
Analyze the project structure and suggest improvements to the code organization.
Focus on separation of concerns and identifying any code that should be extracted
into separate modules.
```

### Launch the Task

Click the **Launch** button. You'll see:

1. The button changes to "Cancel" (you can stop the task at any time)
2. The **Agent Activity** section appears with real-time events
3. Events are color-coded by role with badges showing the agent name
4. Stream output appears line-by-line as the agent generates its response

### Review the Output

When the task completes:
- The **Agent Activity** log shows all events (thoughts, responses)
- The **Final Output** section displays the accumulated result
- The Launch button becomes available again

---

## 4. Multi-Agent Workflows

Multi-agent workflows are the core feature. You can chain multiple agents together, each with a different role, provider, and configuration.

### Adding Roles

Click **"+ Add Role"** to add additional agents. Each role gets its own:
- Name
- Provider and model selection
- Custom prompt/rule/workflow files

### Example: Plan-Develop-Review Pipeline

Set up three roles:

| Role       | Provider | Model           | Purpose                    |
| ---------- | -------- | --------------- | -------------------------- |
| Planner    | Anthropic| Claude 3.5      | Break down the task        |
| Developer  | OpenAI   | GPT-4o          | Write the implementation   |
| Reviewer   | Ollama   | llama3.2        | Review the output          |

### How Execution Works

Roles execute **sequentially** in the order they appear:

1. **Planner** receives the task description and produces a plan
2. **Developer** receives the original task PLUS the Planner's output as context
3. **Reviewer** receives everything above PLUS the Developer's output

Each subsequent role builds on the accumulated context from previous roles.

### Removing Roles

Click the **"x"** button on any role card to remove it from the pipeline.

### Reordering Roles

Currently, roles execute in the order they are added. To reorder, remove and re-add roles in the desired sequence.

---

## 5. Custom Prompts, Rules, and Workflows

### Setting Up the `.agent/` Directory

In your workspace, create the following structure:

```
your-project/
└── .agent/
    ├── prompts/
    │   ├── architect-prompt.md
    │   ├── developer-prompt.md
    │   └── reviewer-prompt.md
    ├── rules/
    │   ├── coding-standards.md
    │   └── security-rules.md
    └── workflows/
        ├── feature-workflow.md
        └── bugfix-workflow.md
```

### Writing Custom Prompts

A prompt file is a markdown file that serves as the system instruction for an agent role:

```markdown
# Architect Prompt

You are a senior software architect. When given a task:

1. Analyze the existing codebase structure
2. Identify architectural patterns in use
3. Propose a high-level design that fits the existing patterns
4. List specific files that need to be created or modified
5. Define the interfaces between components

Always consider maintainability, testability, and performance.
```

### Assigning Resources to Roles

After selecting a workspace with an `.agent/` directory:

1. The **Prompt**, **Rules**, and **Workflow** dropdowns populate with discovered files
2. Select the appropriate file for each role
3. If no custom file is selected, the agent uses a default prompt

### Rules vs Prompts vs Workflows

| Resource   | Purpose                                              |
| ---------- | ---------------------------------------------------- |
| **Prompt** | System instruction defining the agent's role/persona |
| **Rules**  | Constraints and guidelines the agent must follow      |
| **Workflow**| Step-by-step procedure the agent should execute      |

---

## 6. Using the Remote Control

### Starting the Server

1. Scroll to the **Remote Control** section
2. Click **"Start Server"**
3. The display shows the server IP address (e.g., `192.168.1.100:5555`)

### Connecting from Android

1. Install the Android companion app (see [android/README.md](android/README.md))
2. Ensure both devices are on the **same WiFi network**
3. Enter the IP address shown in the desktop app
4. Tap **Connect**

### Remote Operations

From the Android app, you can:

- **Browse models** -- View available LLM providers and models
- **Configure roles** -- Set up agent roles remotely
- **Submit tasks** -- Enter and launch tasks
- **Monitor progress** -- View real-time agent events
- **Cancel tasks** -- Stop running tasks
- **Provide input** -- Respond to agent questions

### Stopping the Server

Click **"Stop Server"** in the Remote Control section when done. The server is off by default and only runs when explicitly started.

---

## 7. MCP Configuration

Model Context Protocol (MCP) servers extend agent capabilities with additional tools.

### Configuring MCP Servers

The MCP configuration is a JSON textarea in the Configuration section. The default configuration includes:

```json
{
  "mcpServers": {
    "sequential-thinking": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-sequential-thinking"],
      "env": {}
    },
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "/path/to/workspace"],
      "disabledTools": ["read_file"]
    },
    "memory": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-memory"]
    }
  }
}
```

### Available MCP Servers

| Server                | Purpose                                       |
| --------------------- | --------------------------------------------- |
| `sequential-thinking` | Structured step-by-step reasoning             |
| `filesystem`          | File read/write/search within a directory     |
| `memory`              | Persistent key-value memory across sessions   |

### Adding Custom MCP Servers

Add new entries to the JSON configuration following the same pattern:

```json
"my-server": {
  "command": "npx",
  "args": ["-y", "@my-org/my-mcp-server"],
  "env": {
    "API_KEY": "..."
  }
}
```

---

## 8. Working with Local Models

### Ollama Setup

1. Install Ollama:
   ```bash
   curl -fsSL https://ollama.com/install.sh | sh
   ```

2. Pull models:
   ```bash
   ollama pull llama3.2
   ollama pull codellama
   ollama pull mistral
   ```

3. In the app, select **Ollama** as the provider -- pulled models appear automatically

### Benefits of Local Models

- **Privacy** -- Data never leaves your machine
- **No API costs** -- Free inference
- **Offline capable** -- Works without internet
- **Low latency** -- No network round-trips

### Tips for Local Models

- Smaller models (7B parameters) are faster but less capable
- Code-specific models (CodeLlama, DeepSeek Coder) perform better on coding tasks
- Ensure you have sufficient GPU VRAM for your chosen model size

---

## Example Workflows

### Code Review Pipeline

| Role       | Model          | Prompt Focus                        |
| ---------- | -------------- | ----------------------------------- |
| Analyzer   | GPT-4o         | Identify patterns and issues        |
| Reviewer   | Claude 3.5     | Detailed review with suggestions    |
| Summarizer | llama3.2 (local)| Concise summary of findings        |

**Task**: "Review the authentication module for security vulnerabilities, code quality issues, and potential performance bottlenecks."

### Feature Implementation

| Role       | Model          | Prompt Focus                        |
| ---------- | -------------- | ----------------------------------- |
| Planner    | Claude 3.5     | Architecture and task breakdown     |
| Developer  | GPT-4o         | Code implementation                 |
| Tester     | CodeLlama      | Unit test generation                |

**Task**: "Implement a user notification system with email and in-app notifications. Include database schema, API endpoints, and a React component for the notification bell."

### Bug Investigation

| Role         | Model          | Prompt Focus                      |
| ------------ | -------------- | --------------------------------- |
| Investigator | GPT-4o         | Root cause analysis               |
| Fixer        | Claude 3.5     | Propose and implement fix         |
| Verifier     | llama3.2       | Verify fix and check for regressions |

**Task**: "The login endpoint returns 500 errors intermittently under load. Investigate the possible causes, propose a fix, and verify it handles concurrent requests correctly."

---

## Inter-Agent Communication

During execution, agents can interact with you and each other.

### Agent Asking You a Question

When an agent's response contains `[[ASK_USER]]`, a modal dialog appears asking for your input. Type your response and click Submit. The agent continues with your answer as additional context.

### Agent Asking Another Agent

When a response contains `[[ASK_AGENT:Developer]]`, an authorization modal appears. You can:

- **Approve** -- The target agent is queried and its response is fed back
- **Deny** -- The requesting agent continues without the response

This ensures you maintain control over inter-agent interactions.

---

## Next Steps

- Explore [ARCHITECTURE.md](ARCHITECTURE.md) to understand the system design
- Set up custom prompts in `.agent/` for your specific workflows
- Try different provider combinations to find what works best for your tasks
- See [TROUBLESHOOTING.md](TROUBLESHOOTING.md) if you run into issues
