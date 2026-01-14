package com.example.remotelauncher.viewmodel

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.example.remotelauncher.network.*
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

sealed class Screen {
    object Connection : Screen()
    object ModelSelection : Screen()
    object TaskExecution : Screen()
}

data class AppState(
    val currentScreen: Screen = Screen.Connection,
    val isConnected: Boolean = false,
    val serverAddress: String = "",
    val errorMessage: String? = null,
    val availableModels: Map<String, List<String>> = emptyMap(),
    val selectedRoles: List<RoleConfig> = listOf(
        RoleConfig("Planner", ModelConfig("openai", "gpt-4o")),
        RoleConfig("Developer", ModelConfig("openai", "gpt-4o-mini")),
        RoleConfig("Reviewer", ModelConfig("openai", "gpt-4o"))
    ),
    val task: String = "",
    val workDir: String = "./workspace",
    val mcpConfig: String = """
{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "/home/pkhunter/Repositories/Coding-Assistants"],
      "disabledTools": ["read_file"]
    }
  }
}
""".trimIndent(),
    val taskResult: String = "",
    val isExecutingTask: Boolean = false
)

class MainViewModel : ViewModel() {
    private val _state = MutableStateFlow(AppState())
    val state: StateFlow<AppState> = _state.asStateFlow()
    
    private var tcpClient: TcpClient? = null
    
    fun connectToServer(ipAddress: String) {
        viewModelScope.launch {
            try {
                _state.value = _state.value.copy(
                    errorMessage = null,
                    serverAddress = ipAddress
                )
                
                tcpClient = TcpClient(ipAddress)
                val connectResult = tcpClient?.connect()
                
                if (connectResult?.isSuccess == true) {
                    _state.value = _state.value.copy(
                        isConnected = true
                    )
                    
                    // Start listening to messages
                    launch {
                        tcpClient?.messages?.collect { response ->
                            handleResponse(response)
                        }
                    }
                    
                    // Fetch available models
                    tcpClient?.getModels()
                } else {
                    _state.value = _state.value.copy(
                        errorMessage = "Connection failed: ${connectResult?.exceptionOrNull()?.message}"
                    )
                }
            } catch (e: Exception) {
                _state.value = _state.value.copy(
                    errorMessage = "Error: ${e.message}"
                )
            }
        }
    }
    
    private fun handleResponse(response: ServerResponse) {
        when (response) {
            is ServerResponse.ModelsList -> {
                _state.value = _state.value.copy(
                    currentScreen = Screen.ModelSelection,
                    availableModels = response.models
                )
            }
            is ServerResponse.TaskStarted -> {
                 _state.value = _state.value.copy(
                    taskResult = "Task Started...\n",
                    isExecutingTask = true
                )
            }
            is ServerResponse.TaskEvent -> {
                 val newResult = _state.value.taskResult + "\n[${response.source}] ${response.event_type}: ${response.content}"
                 _state.value = _state.value.copy(
                    taskResult = newResult
                )
            }
            is ServerResponse.TaskComplete -> {
                 val newResult = _state.value.taskResult + "\n\nTask Complete: ${response.result}"
                 _state.value = _state.value.copy(
                    taskResult = newResult,
                    isExecutingTask = false
                )
            }
            is ServerResponse.Error -> {
                _state.value = _state.value.copy(
                    errorMessage = "Server Error: ${response.message}"
                )
            }
            is ServerResponse.Status -> {
                 if (response.running) {
                     // Maybe update something?
                 } else {
                     // Maybe task cancelled?
                 }
            }
        }
    }
    
    fun disconnect() {
        tcpClient?.disconnect()
        _state.value = AppState()
    }
    
    fun navigateTo(screen: Screen) {
        _state.value = _state.value.copy(currentScreen = screen)
    }
    
    fun updateRole(index: Int, role: RoleConfig) {
        val newRoles = _state.value.selectedRoles.toMutableList()
        if (index < newRoles.size) {
            newRoles[index] = role
            _state.value = _state.value.copy(selectedRoles = newRoles)
        }
    }
    
    fun addRole() {
        val newRoles = _state.value.selectedRoles.toMutableList()
        newRoles.add(
            RoleConfig(
                "New Role ${newRoles.size + 1}",
                ModelConfig("openai", "gpt-4o-mini")
            )
        )
        _state.value = _state.value.copy(selectedRoles = newRoles)
    }
    
    fun removeRole(index: Int) {
        val newRoles = _state.value.selectedRoles.toMutableList()
        if (index < newRoles.size) {
            newRoles.removeAt(index)
            _state.value = _state.value.copy(selectedRoles = newRoles)
        }
    }
    
    fun updateTask(task: String) {
        _state.value = _state.value.copy(task = task)
    }
    
    fun updateWorkDir(workDir: String) {
        _state.value = _state.value.copy(workDir = workDir)
    }
    
    fun updateMcpConfig(mcpConfig: String) {
        _state.value = _state.value.copy(mcpConfig = mcpConfig)
    }
    
    fun executeTask() {
        viewModelScope.launch {
            try {
                _state.value = _state.value.copy(
                    isExecutingTask = true,
                    errorMessage = null,
                    taskResult = ""
                )
                
                val config = AgentConfig(
                    roles = _state.value.selectedRoles,
                    work_dir = _state.value.workDir,
                    mcp_config = _state.value.mcpConfig
                )
                
                val result = tcpClient?.startTask(config, _state.value.task)
                
                if (result?.isFailure == true) {
                     _state.value = _state.value.copy(
                        errorMessage = "Failed to start task: ${result.exceptionOrNull()?.message}",
                        isExecutingTask = false
                    )
                }
            } catch (e: Exception) {
                _state.value = _state.value.copy(
                    errorMessage = "Error: ${e.message}",
                    isExecutingTask = false
                )
            }
        }
    }
    
    fun cancelTask() {
        viewModelScope.launch {
            try {
                tcpClient?.cancelTask()
                _state.value = _state.value.copy(
                    isExecutingTask = false,
                    taskResult = _state.value.taskResult + "\nCancelled by user."
                )
            } catch (e: Exception) {
                _state.value = _state.value.copy(
                    errorMessage = "Failed to cancel: ${e.message}"
                )
            }
        }
    }
    
    override fun onCleared() {
        super.onCleared()
        tcpClient?.disconnect()
    }
}
