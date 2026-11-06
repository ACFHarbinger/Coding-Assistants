package com.codingassistants.remotelauncher.ui

import com.codingassistants.remotelauncher.network.WakeRecord

data class WakeDisplayContext(
    val actionLabel: String,
    val targetLabel: String,
    val scopeLabel: String?,
    val preview: String,
    val messageRef: String?,
    val requiresHumanGate: Boolean,
    val createdAt: String,
)

internal data class RoutingTag(
    val kind: String?,
    val channel: String?,
    val sessionId: String?,
    val isThread: Boolean,
    val raw: String,
) {
    fun scopeLabel(): String? =
        when {
            sessionId != null -> "Session ${sessionId.take(8)}"
            channel != null -> "#$channel"
            else -> null
        }
}

private val AGENT_LABELS =
    mapOf(
        "claude" to "Claude",
        "gemini" to "Gemini",
        "grok" to "Grok",
        "chat" to "Chat / Codex",
        "codex" to "Codex",
        "opencode" to "OpenCode",
        "deepseek" to "DeepSeek",
        "human" to "Human (Owner)",
        "planner" to "Planner",
        "developer" to "Developer",
        "reviewer" to "Reviewer",
        "vibe" to "Vibe",
        "openai" to "OpenAI",
        "anthropic" to "Anthropic",
        "google" to "Google",
        "system" to "System",
    )

private val UUID_RE =
    Regex("^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$")

fun WakeRecord.toDisplayContext(): WakeDisplayContext {
    val routing =
        parseRoutingTag(reason.orEmpty())
            ?: parseRoutingTag(target_agent)
    return WakeDisplayContext(
        actionLabel = actionLabelFor(reason, routing),
        targetLabel = resolveTarget(target_agent, routing),
        scopeLabel = routing?.scopeLabel(),
        preview = previewFor(reason, routing),
        messageRef =
            message_id?.takeIf { it.isNotBlank() }?.let { id ->
                "#msg-${id.take(8)}"
            },
        requiresHumanGate = requires_human_gate,
        createdAt = created_at,
    )
}

internal fun parseRoutingTag(text: String): RoutingTag? {
    val trimmed = text.removePrefix("tagged send:").trim()
    val start =
        listOf("channel:", "private:", "tagged:")
            .map { needle -> trimmed.indexOf(needle) }
            .filter { it >= 0 }
            .minOrNull() ?: return null
    val raw = trimmed.substring(start).substringBefore(' ')
    val parts = raw.split(':').filter { it.isNotEmpty() }
    if (parts.isEmpty()) return null
    if (parts[0] == "private" || parts[0] == "tagged") {
        return RoutingTag(
            kind = "wake",
            channel = null,
            sessionId = null,
            isThread = false,
            raw = raw,
        )
    }
    if (parts[0] != "channel") return null

    var index = 1
    var channel: String? = null
    var sessionId: String? = null
    var isThread = false
    var kind: String? = null

    if (parts.getOrNull(index) == "session") {
        index += 1
        sessionId = parts.getOrNull(index)
        index += 1
    } else {
        val token = parts.getOrNull(index)
        if (token != null && token != "kind" && token != "thread" && !isUuid(token)) {
            channel = token
            index += 1
        }
    }

    while (index < parts.size) {
        when (parts[index]) {
            "thread" -> {
                isThread = true
                index += 2
            }
            "kind" -> {
                kind = parts.getOrNull(index + 1)
                index += 2
            }
            else -> index += 1
        }
    }

    return RoutingTag(
        kind = kind,
        channel = channel,
        sessionId = sessionId,
        isThread = isThread,
        raw = raw,
    )
}

private fun actionLabelFor(
    reason: String?,
    routing: RoutingTag?,
): String {
    val reasonLower = reason.orEmpty().lowercase()
    val kind = routing?.kind
    val session = routing?.sessionId != null
    val channel = routing?.channel != null
    return when {
        "audit" in reasonLower -> "Audit Authorization"
        "handoff" in reasonLower || kind == "handoff" -> "Agent Handoff Gate"
        reasonLower.startsWith("task ") -> "Task Execution Request"
        kind == "task" && session -> "Work Session Task"
        kind == "task" && channel -> "Channel Task Assignment"
        kind == "task" -> "Task Execution Request"
        kind == "wake" && session -> "Work Session Wake"
        kind == "wake" && channel -> "Channel Wake Signal"
        kind == "wake" -> "Work Session Wake"
        reasonLower.contains("chat & memory") -> "Channel Wake Signal"
        else -> "Human Decision Required"
    }
}

private fun resolveTarget(
    targetAgent: String,
    routing: RoutingTag?,
): String {
    if (targetAgent.startsWith("channel:") ||
        targetAgent.startsWith("private:") ||
        targetAgent.startsWith("tagged:")
    ) {
        routing?.channel?.let { return "#$it" }
        routing?.sessionId?.let { return "Session ${it.take(8)}" }
    }
    return AGENT_LABELS[targetAgent.lowercase()] ?: titleCaseId(targetAgent)
}

private fun previewFor(
    reason: String?,
    routing: RoutingTag?,
): String {
    val raw = reason?.trim().orEmpty()
    if (raw.isEmpty()) return "No additional payload."
    val stripped = raw.removePrefix("tagged send:").trim()
    val routingOnly =
        stripped == routing?.raw ||
            stripped.startsWith("channel:") ||
            stripped.startsWith("private:") ||
            stripped.startsWith("tagged:")
    if (routingOnly) return "No additional payload."
    return stripped
}

private fun isUuid(value: String): Boolean = UUID_RE.matches(value)

private fun titleCaseId(value: String): String =
    value
        .replace('-', ' ')
        .replace('_', ' ')
        .split(' ')
        .filter { it.isNotEmpty() }
        .joinToString(" ") { token ->
            token.replaceFirstChar { ch -> ch.titlecase() }
        }
