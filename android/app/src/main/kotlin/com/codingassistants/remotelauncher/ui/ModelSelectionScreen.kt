package com.codingassistants.remotelauncher.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.ArrowForward
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.codingassistants.remotelauncher.network.AgentResources
import com.codingassistants.remotelauncher.network.RoleConfig
import com.codingassistants.remotelauncher.viewmodel.AppState

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ModelSelectionScreen(
    state: AppState,
    onUpdateRole: (Int, RoleConfig) -> Unit,
    onAddRole: () -> Unit,
    onRemoveRole: (Int) -> Unit,
    onNext: () -> Unit,
    onBack: () -> Unit,
) {
    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Configure Agents") },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.Default.ArrowBack, "Back")
                    }
                },
            )
        },
        bottomBar = {
            Surface(tonalElevation = 3.dp) {
                Row(
                    modifier =
                        Modifier
                            .fillMaxWidth()
                            .padding(16.dp),
                    horizontalArrangement = Arrangement.SpaceBetween,
                ) {
                    OutlinedButton(
                        onClick = onAddRole,
                        modifier = Modifier.weight(1f),
                    ) {
                        Icon(Icons.Default.Add, contentDescription = null)
                        Spacer(Modifier.width(8.dp))
                        Text("Add Role")
                    }

                    Spacer(Modifier.width(16.dp))

                    Button(
                        onClick = onNext,
                        modifier = Modifier.weight(1f),
                        enabled = state.selectedRoles.isNotEmpty(),
                    ) {
                        Text("Next")
                        Spacer(Modifier.width(8.dp))
                        Icon(Icons.Default.ArrowForward, contentDescription = null)
                    }
                }
            }
        },
    ) { padding ->
        LazyColumn(
            modifier =
                Modifier
                    .fillMaxSize()
                    .padding(padding)
                    .padding(horizontal = 16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            item {
                Spacer(Modifier.height(8.dp))
            }

            itemsIndexed(state.selectedRoles) { index, role ->
                RoleCard(
                    role = role,
                    availableModels = state.availableModels,
                    resources = state.agentResources,
                    onUpdate = { onUpdateRole(index, it) },
                    onRemove = { onRemoveRole(index) },
                )
            }

            item {
                Spacer(Modifier.height(8.dp))
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun RoleCard(
    role: RoleConfig,
    availableModels: Map<String, List<String>>,
    resources: AgentResources,
    onUpdate: (RoleConfig) -> Unit,
    onRemove: () -> Unit,
) {
    var providerExpanded by remember { mutableStateOf(false) }
    var modelExpanded by remember { mutableStateOf(false) }

    val providerNames =
        mapOf(
            "opencode" to "OpenCode Zen",
            "google" to "Google",
            "anthropic" to "Anthropic",
            "openai" to "OpenAI",
            "github_copilot" to "GitHub Copilot",
        )

    Card(
        modifier = Modifier.fillMaxWidth(),
    ) {
        Column(
            modifier = Modifier.padding(16.dp),
        ) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                OutlinedTextField(
                    value = role.name,
                    onValueChange = { onUpdate(role.copy(name = it)) },
                    label = { Text("Role name") },
                    singleLine = true,
                    modifier = Modifier.weight(1f),
                )
                IconButton(onClick = onRemove) {
                    Icon(
                        Icons.Default.Delete,
                        contentDescription = "Remove",
                        tint = MaterialTheme.colorScheme.error,
                    )
                }
            }

            Spacer(Modifier.height(12.dp))

            // Provider selection
            ExposedDropdownMenuBox(
                expanded = providerExpanded,
                onExpandedChange = { providerExpanded = it },
            ) {
                OutlinedTextField(
                    value = providerNames[role.config.provider] ?: role.config.provider,
                    onValueChange = {},
                    readOnly = true,
                    label = { Text("Provider") },
                    trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = providerExpanded) },
                    modifier =
                        Modifier
                            .fillMaxWidth()
                            .menuAnchor(),
                )
                ExposedDropdownMenu(
                    expanded = providerExpanded,
                    onDismissRequest = { providerExpanded = false },
                ) {
                    availableModels.keys.forEach { provider ->
                        DropdownMenuItem(
                            text = { Text(providerNames[provider] ?: provider) },
                            onClick = {
                                val models = availableModels[provider] ?: emptyList()
                                onUpdate(
                                    role.copy(
                                        config =
                                            role.config.copy(
                                                provider = provider,
                                                model = models.firstOrNull() ?: "",
                                            ),
                                    ),
                                )
                                providerExpanded = false
                            },
                        )
                    }
                }
            }

            Spacer(Modifier.height(8.dp))

            // Model selection
            val currentProviderModels = availableModels[role.config.provider] ?: emptyList()
            ExposedDropdownMenuBox(
                expanded = modelExpanded,
                onExpandedChange = { modelExpanded = it },
            ) {
                OutlinedTextField(
                    value = role.config.model,
                    onValueChange = {},
                    readOnly = true,
                    label = { Text("Model") },
                    trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = modelExpanded) },
                    modifier =
                        Modifier
                            .fillMaxWidth()
                            .menuAnchor(),
                )
                ExposedDropdownMenu(
                    expanded = modelExpanded,
                    onDismissRequest = { modelExpanded = false },
                ) {
                    currentProviderModels.forEach { model ->
                        DropdownMenuItem(
                            text = { Text(model) },
                            onClick = {
                                onUpdate(role.copy(config = role.config.copy(model = model)))
                                modelExpanded = false
                            },
                        )
                    }
                }
            }

            Spacer(Modifier.height(8.dp))

            RoleResourcePickers(
                role = role,
                resources = resources,
                onUpdate = onUpdate,
            )
        }
    }
}
