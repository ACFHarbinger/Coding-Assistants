package com.codingassistants.remotelauncher.ui

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.CheckCircle
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.Info
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.codingassistants.remotelauncher.viewmodel.AppState

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun TaskExecutionScreen(
    state: AppState,
    onUpdateTask: (String) -> Unit,
    onExecuteTask: () -> Unit,
    onCancelTask: () -> Unit,
    onBack: () -> Unit,
    onDisconnect: () -> Unit,
) {
    BackHandler {
        onBack()
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Execute Task") },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, "Back")
                    }
                },
                actions = {
                    IconButton(onClick = onDisconnect) {
                        Icon(Icons.Default.Close, "Disconnect")
                    }
                },
            )
        },
    ) { padding ->
        Column(
            modifier =
                Modifier
                    .fillMaxSize()
                    .padding(padding)
                    .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            val isLive = state.isConnected && !state.isConnectionLost

            // Server info card
            Card(
                colors =
                    CardDefaults.cardColors(
                        containerColor =
                            if (isLive) {
                                MaterialTheme.colorScheme.primaryContainer
                            } else {
                                MaterialTheme.colorScheme.errorContainer
                            },
                    ),
            ) {
                Row(
                    modifier =
                        Modifier
                            .fillMaxWidth()
                            .padding(16.dp),
                    horizontalArrangement = Arrangement.SpaceBetween,
                ) {
                    Column {
                        Text(
                            if (isLive) "Connected to" else "Connection lost",
                            style = MaterialTheme.typography.labelSmall,
                            color =
                                if (isLive) {
                                    MaterialTheme.colorScheme.onPrimaryContainer
                                } else {
                                    MaterialTheme.colorScheme.onErrorContainer
                                },
                        )
                        Text(
                            state.serverAddress,
                            style = MaterialTheme.typography.bodyLarge,
                            color =
                                if (isLive) {
                                    MaterialTheme.colorScheme.onPrimaryContainer
                                } else {
                                    MaterialTheme.colorScheme.onErrorContainer
                                },
                        )
                    }
                    Icon(
                        if (isLive) Icons.Default.CheckCircle else Icons.Default.Info,
                        contentDescription = if (isLive) "Connected" else "Connection lost",
                        tint =
                            if (isLive) {
                                MaterialTheme.colorScheme.primary
                            } else {
                                MaterialTheme.colorScheme.error
                            },
                    )
                }
            }

            // Task input
            OutlinedTextField(
                value = state.task,
                onValueChange = onUpdateTask,
                label = { Text("Task Description") },
                placeholder = { Text("What should the agents build?") },
                modifier =
                    Modifier
                        .fillMaxWidth()
                        .height(200.dp),
                maxLines = 8,
                enabled = !state.isExecutingTask,
            )

            // Workspace directory (optional)
            OutlinedTextField(
                value = state.workDir,
                onValueChange = {},
                label = { Text("Workspace (configured on PC)") },
                modifier = Modifier.fillMaxWidth(),
                readOnly = true,
                enabled = false,
            )

            // Result message
            if (state.taskResult.isNotEmpty()) {
                Card(
                    modifier = Modifier.weight(1f),
                    colors =
                        CardDefaults.cardColors(
                            containerColor = MaterialTheme.colorScheme.tertiaryContainer,
                        ),
                ) {
                    Column(
                        modifier =
                            Modifier
                                .fillMaxSize()
                                .padding(16.dp)
                                .verticalScroll(rememberScrollState()),
                    ) {
                        Row(
                            horizontalArrangement = Arrangement.spacedBy(8.dp),
                        ) {
                            Icon(
                                Icons.Default.Info,
                                contentDescription = null,
                                tint = MaterialTheme.colorScheme.onTertiaryContainer,
                            )
                            Text(
                                text = state.taskResult,
                                color = MaterialTheme.colorScheme.onTertiaryContainer,
                            )
                        }
                    }
                }
            } else {
                Spacer(Modifier.weight(1f))
            }

            // Error message
            state.errorMessage?.let { error ->
                Card(
                    colors =
                        CardDefaults.cardColors(
                            containerColor = MaterialTheme.colorScheme.errorContainer,
                        ),
                ) {
                    Text(
                        text = error,
                        modifier = Modifier.padding(16.dp),
                        color = MaterialTheme.colorScheme.onErrorContainer,
                    )
                }
            }

            // Execute button
            if (state.isExecutingTask) {
                Button(
                    onClick = onCancelTask,
                    modifier =
                        Modifier
                            .fillMaxWidth()
                            .height(56.dp),
                    colors =
                        ButtonDefaults.buttonColors(
                            containerColor = MaterialTheme.colorScheme.error,
                        ),
                ) {
                    CircularProgressIndicator(
                        modifier = Modifier.size(20.dp),
                        color = MaterialTheme.colorScheme.onError,
                    )
                    Spacer(Modifier.width(8.dp))
                    Text("Cancel Task")
                }
            } else {
                Button(
                    onClick = onExecuteTask,
                    modifier =
                        Modifier
                            .fillMaxWidth()
                            .height(56.dp),
                    enabled = isLive && state.task.isNotBlank(),
                ) {
                    Icon(Icons.Default.PlayArrow, contentDescription = null)
                    Spacer(Modifier.width(8.dp))
                    Text(if (isLive) "Launch Sequence" else "Launch Sequence (Disconnected)")
                }
            }
        }
    }
}
