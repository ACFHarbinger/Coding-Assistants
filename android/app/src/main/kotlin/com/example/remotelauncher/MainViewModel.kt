package com.example.remotelauncher

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.launch

class MainViewModel : ViewModel() {
    private val client = TcpClient()

    var ip by mutableStateOf("")
    var isConnected by mutableStateOf(false)
    var status by mutableStateOf("Not connected")

    var models by mutableStateOf<Map<String, List<String>>>(emptyMap())
    var selectedProvider by mutableStateOf("")
    var selectedModel by mutableStateOf("")

    var taskText by mutableStateOf("")
    var isRunning by mutableStateOf(false)

    val logs = mutableStateListOf<String>()

    init {
        client.onDisconnected = {
            isConnected = false
            status = "Disconnected from server"
            models = emptyMap()
        }
        viewModelScope.launch { client.events.collect { response -> handleResponse(response) } }
    }

    private fun handleResponse(response: ServerResponse) {
        when (response) {
            is ServerResponse.ModelsList -> {
                models = response.models
                if (selectedProvider.isEmpty() && models.isNotEmpty()) {
                    selectedProvider = models.keys.first()
                    selectedModel = models[selectedProvider]?.firstOrNull() ?: ""
                }
            }
            is ServerResponse.TaskStarted -> {
                isRunning = true
                status = "Task started!"
                logs.add("Task started...")
            }
            is ServerResponse.TaskEvent -> {
                logs.add("[${response.source}] ${response.content}")
                // Auto-scroll logic would be in UI
            }
            is ServerResponse.TaskComplete -> {
                isRunning = false
                status = "Task complete!"
                logs.add("Result: ${response.result}")
            }
            is ServerResponse.Status -> {
                status = response.message
            }
            is ServerResponse.Error -> {
                status = "Error: ${response.message}"
                logs.add("Error: ${response.message}")
            }
        }
    }

    fun connect() {
        if (ip.isBlank()) return
        viewModelScope.launch {
            try {
                status = "Connecting..."
                client.connect(ip)
                isConnected = true
                status = "Connected to $ip"
                fetchModels()
            } catch (e: Exception) {
                status = "Connection failed: ${e.message}"
            }
        }
    }

    private fun fetchModels() {
        viewModelScope.launch { client.send(ClientRequest.GetModels) }
    }

    fun launchTask() {
        if (taskText.isBlank()) return
        viewModelScope.launch {
            val config =
                    AgentConfig(
                            roles =
                                    listOf(
                                            RoleConfig(
                                                    "Planner",
                                                    ModelConfig(selectedProvider, selectedModel)
                                            ),
                                            RoleConfig(
                                                    "Developer",
                                                    ModelConfig(selectedProvider, selectedModel)
                                            ),
                                            RoleConfig(
                                                    "Reviewer",
                                                    ModelConfig(selectedProvider, selectedModel)
                                            )
                                    )
                    )
            client.send(ClientRequest.StartTask(config, taskText))
        }
    }

    fun cancelTask() {
        viewModelScope.launch {
            client.send(ClientRequest.CancelTask)
            isRunning = false
        }
    }

    fun disconnect() {
        client.disconnect()
        isConnected = false
        status = "Disconnected"
    }
}
