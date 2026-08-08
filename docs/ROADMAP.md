# Roadmap

[![Status](https://img.shields.io/badge/Status-Active_Development-brightgreen)](ROADMAP.md)
[![Version](https://img.shields.io/badge/Version-0.1.0-orange)](package.json)

Project roadmap for Coding Assistants. Items are organized by priority and category.

---

## Current Status: v0.1.0 (Alpha)

The application is functional with core multi-agent orchestration, multiple LLM provider support, and Android remote control. The following features are planned for upcoming releases.

---

## Short-Term Goals

### Agent System Improvements

- [ ] **Parallel agent execution** -- Run independent agent roles concurrently instead of strictly sequential
- [ ] **Agent memory persistence** -- Store and recall agent outputs across sessions using a local database
- [ ] **Conversation history** -- Allow agents to reference previous task results and context
- [ ] **Configurable execution strategies** -- Support sequential, parallel, and conditional (branching) workflows
- [ ] **Agent templates** -- Pre-built role configurations for common tasks (code review, refactoring, debugging)

### LLM Provider Enhancements

- [ ] **Direct HTTP API calls** -- Add native HTTP integration for OpenAI, Anthropic, and Google APIs (alongside CLI-based providers)
- [ ] **LM Studio full support** -- Complete the LM Studio provider integration
- [ ] **Model parameter tuning** -- Expose temperature, top-p, max tokens, and other generation parameters per role
- [ ] **Provider health checks** -- Verify API connectivity and model availability before task execution
- [ ] **Cost estimation** -- Track and display estimated token usage and API costs

### UI/UX Improvements

- [ ] **Component extraction** -- Break `App.tsx` into modular components (`ConfigPanel`, `EventViewer`, `OutputDisplay`, etc.)
- [ ] **Dark/Light theme toggle** -- Add theme switching support beyond the current dark-only glass-morphism design
- [ ] **Task history sidebar** -- Browse and re-run previous tasks
- [ ] **Drag-and-drop role ordering** -- Reorder agent execution sequence visually
- [ ] **Keyboard shortcuts** -- Add hotkeys for common actions (launch task, cancel, switch panels)
- [ ] **Responsive layout** -- Improve layout for different window sizes

---

## Medium-Term Goals

### Architecture

- [ ] **Typed error system** -- Replace `Result<T, String>` with a structured error enum across all commands
- [ ] **Plugin/provider trait** -- Abstract LLM providers behind a Rust trait for cleaner extensibility
- [ ] **Configuration persistence** -- Save and load agent configurations to/from files
- [ ] **Workspace profiles** -- Associate agent configurations with specific project directories

### Testing Infrastructure

- [ ] **Frontend test suite** -- Set up Vitest + React Testing Library with component and integration tests
- [ ] **Backend test suite** -- Add `cargo test` coverage for agent orchestration logic, TCP protocol, and file operations
- [ ] **CI/CD pipeline** -- GitHub Actions for automated build, lint, and test on PR
- [ ] **End-to-end tests** -- Tauri's WebDriver-based testing for full application flows

### Remote Control

- [ ] **Authentication** -- Add token-based authentication for the TCP remote control server
- [ ] **Encryption** -- TLS support for TCP connections
- [ ] **iOS companion app** -- Build an iOS remote control app with SwiftUI
- [ ] **Web-based remote** -- Browser-based remote control interface

### MCP Integration

- [ ] **MCP server management UI** -- Visual interface for adding, removing, and configuring MCP servers
- [ ] **Built-in MCP servers** -- Bundle commonly used servers (filesystem, memory, sequential-thinking)
- [ ] **Custom MCP tool creation** -- Allow users to define custom MCP tools within the app

---

## Long-Term Vision

### Advanced Agent Capabilities

- [ ] **Agent graph execution** -- Define complex agent workflows as directed graphs with conditions and loops
- [ ] **Tool use / function calling** -- Agents can invoke tools (file operations, web search, code execution) during their reasoning
- [ ] **Self-improvement loop** -- Agents review their own output and iterate to improve quality
- [ ] **Multi-project orchestration** -- Coordinate agents across multiple workspace directories
- [ ] **Code execution sandbox** -- Safely execute generated code within an isolated environment

### Platform Expansion

- [ ] **Cross-platform builds** -- Automated builds for Windows, macOS (Intel + ARM), and Linux (.deb, .rpm, .AppImage)
- [ ] **Auto-updates** -- In-app update mechanism via Tauri's updater plugin
- [ ] **Headless mode** -- Run the agent system as a CLI tool without the GUI

### Community & Ecosystem

- [ ] **Agent marketplace** -- Share and discover agent configurations and prompts
- [ ] **Prompt library** -- Curated collection of effective system prompts for different roles
- [ ] **Documentation site** -- Hosted documentation with API reference and guides
- [ ] **Plugin system** -- Third-party extensions for new providers, tools, and UI components

---

## Completed

- [x] Multi-agent orchestration with sequential role execution
- [x] Multi-provider support (OpenCode, Ollama, OpenAI, Anthropic, Google, GitHub Copilot, LM Studio)
- [x] Real-time streaming output via Tauri events
- [x] Inter-agent communication (`[[ASK_AGENT:RoleName]]`)
- [x] User-in-the-loop interaction (`[[ASK_USER]]`)
- [x] Workspace directory selection and file operations
- [x] Custom prompt, rule, and workflow loading from `.agent/`
- [x] MCP server configuration
- [x] TCP remote control server (port 5555)
- [x] Android companion app (Kotlin/Jetpack Compose)
- [x] Glass-morphism dark theme UI
- [x] Task cancellation support
- [x] Project memory generation

---

## Contributing to the Roadmap

Have a feature request or idea? Open an issue on GitHub with the `enhancement` label, or see [CONTRIBUTING.md](../git/CONTRIBUTING.md) for how to propose changes.
