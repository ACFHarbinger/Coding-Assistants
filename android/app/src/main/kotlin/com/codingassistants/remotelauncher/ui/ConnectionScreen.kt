package com.codingassistants.remotelauncher.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Computer
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.codingassistants.remotelauncher.viewmodel.AppState

private val ipv4 = Regex("""^(\d{1,3}\.){3}\d{1,3}$""")
private val hostname = Regex("""^[A-Za-z0-9][A-Za-z0-9.-]{0,253}$""")

fun isValidServerHost(input: String): Boolean {
    val trimmed = input.trim()
    if (trimmed.isEmpty()) return false
    val host = trimmed.substringBefore(':')
    val portPart = if (trimmed.contains(':')) trimmed.substringAfter(':') else ""
    if (portPart.isNotEmpty()) {
        val port = portPart.toIntOrNull() ?: return false
        if (port !in 1..65535) return false
    }
    if (host.matches(ipv4)) {
        return host.split('.').all { it.toInt() in 0..255 }
    }
    return host.matches(hostname)
}

@Composable
fun ConnectionScreen(
    state: AppState,
    onConnect: (String) -> Unit,
) {
    var ipAddress by remember(state.lastServerIp) { mutableStateOf(state.lastServerIp) }
    val valid = isValidServerHost(ipAddress)

    Column(
        modifier =
            Modifier
                .fillMaxSize()
                .padding(24.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center,
    ) {
        Column(
            modifier = Modifier.widthIn(max = 520.dp).fillMaxWidth(),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Icon(
                imageVector = Icons.Default.Computer,
                contentDescription = "Server",
                modifier = Modifier.size(72.dp),
                tint = MaterialTheme.colorScheme.primary,
            )

            Spacer(modifier = Modifier.height(24.dp))

            Text(
                text = "Coding Assistants",
                style = MaterialTheme.typography.headlineLarge,
                fontWeight = FontWeight.Bold,
            )

            Text(
                text = "Remote Control",
                style = MaterialTheme.typography.titleMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )

            Spacer(modifier = Modifier.height(36.dp))

            OutlinedTextField(
                value = ipAddress,
                onValueChange = { ipAddress = it },
                label = { Text("PC IP address") },
                placeholder = { Text("10.0.0.12") },
                supportingText = {
                    Text(
                        if (ipAddress.isBlank()) {
                            "Use the IPv4 address shown in the desktop app."
                        } else if (!valid) {
                            "Enter a valid IPv4 address or hostname (optional :port)."
                        } else {
                            "Port 5555 is used unless you append :port."
                        },
                    )
                },
                isError = ipAddress.isNotBlank() && !valid,
                modifier = Modifier.fillMaxWidth(),
                singleLine = true,
            )

            if (state.lastServerIp.isNotBlank()) {
                Spacer(modifier = Modifier.height(8.dp))
                SelectionContainer {
                    Text(
                        text = "Last connected: ${state.lastServerIp}",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }

            Spacer(modifier = Modifier.height(28.dp))

            Button(
                onClick = { onConnect(ipAddress.trim()) },
                modifier =
                    Modifier
                        .fillMaxWidth()
                        .height(52.dp),
                enabled = valid,
            ) {
                Text("Connect")
            }

            state.errorMessage?.let { error ->
                Spacer(modifier = Modifier.height(16.dp))
                Card(
                    colors =
                        CardDefaults.cardColors(
                            containerColor = MaterialTheme.colorScheme.errorContainer,
                        ),
                ) {
                    SelectionContainer {
                        Text(
                            text = error,
                            modifier = Modifier.padding(16.dp),
                            color = MaterialTheme.colorScheme.onErrorContainer,
                        )
                    }
                }
            }
        }
    }
}
