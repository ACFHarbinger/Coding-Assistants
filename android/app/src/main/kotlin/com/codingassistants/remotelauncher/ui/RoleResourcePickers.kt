package com.codingassistants.remotelauncher.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.codingassistants.remotelauncher.network.AgentResources
import com.codingassistants.remotelauncher.network.RoleConfig

@Composable
fun RoleResourcePickers(
    role: RoleConfig,
    resources: AgentResources,
    onUpdate: (RoleConfig) -> Unit,
) {
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        ResourceDropdown(
            label = "Prompt",
            noneLabel = "Default",
            value = role.config.prompt_file,
            options = resources.prompts,
            onSelect = { onUpdate(role.copy(config = role.config.copy(prompt_file = it))) },
        )
        ResourceDropdown(
            label = "Rule",
            noneLabel = "None",
            value = role.config.rule_file,
            options = resources.rules,
            onSelect = { onUpdate(role.copy(config = role.config.copy(rule_file = it))) },
        )
        ResourceDropdown(
            label = "Workflow",
            noneLabel = "None",
            value = role.config.workflow_file,
            options = resources.workflows,
            onSelect = { onUpdate(role.copy(config = role.config.copy(workflow_file = it))) },
        )
        ResourceDropdown(
            label = "Skill",
            noneLabel = "None",
            value = role.config.skill_file,
            options = resources.skills,
            onSelect = { onUpdate(role.copy(config = role.config.copy(skill_file = it))) },
        )
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun ResourceDropdown(
    label: String,
    noneLabel: String,
    value: String?,
    options: List<String>,
    onSelect: (String?) -> Unit,
) {
    var expanded by remember { mutableStateOf(false) }
    val display = value?.substringAfterLast('/') ?: noneLabel

    ExposedDropdownMenuBox(
        expanded = expanded,
        onExpandedChange = { expanded = it },
    ) {
        OutlinedTextField(
            value = display,
            onValueChange = {},
            readOnly = true,
            label = { Text(label) },
            trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = expanded) },
            modifier =
                Modifier
                    .fillMaxWidth()
                    .menuAnchor(),
        )
        ExposedDropdownMenu(
            expanded = expanded,
            onDismissRequest = { expanded = false },
        ) {
            DropdownMenuItem(
                text = { Text(noneLabel) },
                onClick = {
                    onSelect(null)
                    expanded = false
                },
            )
            options.forEach { path ->
                DropdownMenuItem(
                    text = { Text(path.substringAfterLast('/')) },
                    onClick = {
                        onSelect(path)
                        expanded = false
                    },
                )
            }
        }
    }
}
