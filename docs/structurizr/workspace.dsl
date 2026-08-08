/*
 * Coding-Assistants — Structurizr DSL workspace (C4 model)
 *
 * See docs/structurizr/README.md for rendering instructions.
 */

workspace "Coding-Assistants" "C4 model for the Coding-Assistants desktop app." {

    model {
        user = person "User" "Runs the desktop app and optionally pairs the Android companion app."

        llmProviders = softwareSystem "LLM Provider CLIs/APIs" "OpenCode Zen, Gemini, Anthropic, OpenAI, GitHub Copilot, Ollama, LM Studio." "External"

        system = softwareSystem "Coding-Assistants" "Tauri desktop app for orchestrating LLM coding agents." {
            webapp = container "Frontend" "React UI: chat, agent controls, file browser." "TypeScript / React / Vite" "src/"
            backend = container "Backend" "Agent orchestration, LLM client, file tools, TCP server." "Rust / Tauri" "src-tauri/"
            android = container "Android Companion App" "Remote-controls the desktop app over TCP/IP." "Kotlin / Jetpack Compose" "android/"
        }

        user -> webapp "Uses"
        webapp -> backend "invoke() IPC"
        backend -> llmProviders "Calls"
        user -> android "Uses"
        android -> backend "TCP/IP"
    }

    views {
        systemContext system "SystemContext" {
            include *
            autoLayout
        }

        container system "Containers" {
            include *
            autoLayout
        }

        theme default
    }
}
