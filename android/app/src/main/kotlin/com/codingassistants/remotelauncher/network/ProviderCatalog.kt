package com.codingassistants.remotelauncher.network

/**
 * Display-name labels for known providers. The selectable list is the
 * desktop GetModels map unioned with these ids so the dropdown is never
 * empty offline or before the TCP reply.
 */
object ProviderCatalog {
    val labels: Map<String, String> =
        mapOf(
            "openai" to "OpenAI",
            "anthropic" to "Anthropic",
            "claude" to "Claude",
            "gemini" to "Gemini",
            "google" to "Google",
            "grok" to "Grok",
            "opencode" to "OpenCode",
            "deepseek" to "DeepSeek",
            "vibe" to "Vibe",
            "chat" to "Chat",
            "codex" to "Codex",
            "github_copilot" to "GitHub Copilot",
        )

    val fallbackModels: Map<String, List<String>> =
        mapOf(
            "openai" to listOf("gpt-4o", "gpt-4o-mini", "gpt-4.1", "o3", "o4-mini"),
            "anthropic" to listOf("claude-sonnet-4-5", "claude-opus-4-1", "claude-haiku-4-5"),
            "claude" to listOf("claude-sonnet-4-5", "claude-opus-4-1", "claude-haiku-4-5"),
            "gemini" to listOf("gemini-2.5-pro", "gemini-2.5-flash"),
            "google" to listOf("gemini-2.5-pro", "gemini-2.5-flash"),
            "grok" to listOf("grok-4", "grok-3"),
            "opencode" to listOf("big-pickle"),
            "deepseek" to listOf("deepseek-chat", "deepseek-reasoner"),
            "vibe" to listOf("mistral-large", "codestral"),
            "chat" to listOf("gpt-5", "gpt-4.1"),
            "codex" to listOf("gpt-5-codex", "o3"),
            "github_copilot" to listOf("gpt-4o", "claude-sonnet-4-5"),
        )

    fun displayName(id: String): String = labels[id] ?: id

    fun providerIds(live: Map<String, List<String>>): List<String> {
        return (live.keys + labels.keys).toSortedSet().toList()
    }

    /**
     * Live GetModels catalogs win when a provider actually returned models;
     * otherwise the static fallback list is used so a provider is still
     * selectable before the TCP reply.
     */
    fun merge(live: Map<String, List<String>>): Map<String, List<String>> =
        providerIds(live).associateWith { key ->
            live[key]?.takeIf { it.isNotEmpty() } ?: fallbackModels[key].orEmpty()
        }
}
