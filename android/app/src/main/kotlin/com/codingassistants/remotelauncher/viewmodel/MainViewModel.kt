package com.codingassistants.remotelauncher.viewmodel

import android.app.Application
import android.content.Context
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.codingassistants.remotelauncher.network.AgentConfig
import com.codingassistants.remotelauncher.network.AgentResources
import com.codingassistants.remotelauncher.network.ModelConfig
import com.codingassistants.remotelauncher.network.ProviderCatalog
import com.codingassistants.remotelauncher.network.RoleConfig
import com.codingassistants.remotelauncher.network.ServerResponse
import com.codingassistants.remotelauncher.network.TcpClient
import com.codingassistants.remotelauncher.network.WakeRecord
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

sealed class Screen {
    object Connection : Screen()

    object Dashboard : Screen()

    object ModelSelection : Screen()

    object TaskExecution : Screen()
}

data class AppState(
    val currentScreen: Screen = Screen.Connection,
    val isConnected: Boolean = false,
    val isConnectionLost: Boolean = false,
    val isReconnecting: Boolean = false,
    val serverAddress: String = "",
    val errorMessage: String? = null,
    val availableModels: Map<String, List<String>> = ProviderCatalog.merge(emptyMap()),
    val selectedRoles: List<RoleConfig> =
        listOf(
            RoleConfig("Planner", ModelConfig("openai", "gpt-4o")),
            RoleConfig("Developer", ModelConfig("openai", "gpt-4o-mini")),
            RoleConfig("Reviewer", ModelConfig("openai", "gpt-4o")),
        ),
    val task: String = "",
    val workDir: String = "./workspace",
    val mcpConfig: String =
        """
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
    val isExecutingTask: Boolean = false,
    val pendingWakes: List<WakeRecord> = emptyList(),
    val activeEvents: List<ServerResponse.TaskEvent> = emptyList(),
    val agentResources: AgentResources = AgentResources(),
    val lastServerIp: String = "",
)

fun parseHostPort(
    input: String,
    defaultPort: Int = 5555,
): Pair<String, Int> {
    val trimmed = input.trim()
    if (trimmed.isEmpty()) return Pair("", defaultPort)
    val colonIdx = trimmed.lastIndexOf(':')
    if (colonIdx > 0 && colonIdx < trimmed.length - 1) {
        val hostPart = trimmed.substring(0, colonIdx)
        val portPart = trimmed.substring(colonIdx + 1).toIntOrNull()
        if (portPart != null && portPart in 1..65535) {
            return Pair(hostPart, portPart)
        }
    }
    return Pair(trimmed, defaultPort)
}

class MainViewModel(
    application: Application,
) : AndroidViewModel(application) {
    private val prefs =
        application.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)

    private val _state =
        MutableStateFlow(AppState(lastServerIp = prefs.getString(PREF_LAST_SERVER_IP, "") ?: ""))
    val state: StateFlow<AppState> = _state.asStateFlow()

    private var tcpClient: TcpClient? = null

    private fun persistServerIp(ipAddress: String) {
        prefs.edit().putString(PREF_LAST_SERVER_IP, ipAddress).apply()
    }

    fun connectToServer(ipAddress: String) {
        viewModelScope.launch {
            val (host, port) = parseHostPort(ipAddress)
            try {
                _state.value =
                    _state.value.copy(
                        errorMessage = null,
                        serverAddress = ipAddress.trim(),
                        isConnectionLost = false,
                        isReconnecting = false,
                    )

                val client = TcpClient(host, port)
                val connectResult = client.connect()

                if (connectResult.isSuccess) {
                    tcpClient = client
                    persistServerIp(ipAddress.trim())
                    _state.value =
                        _state.value.copy(
                            isConnected = true,
                            isConnectionLost = false,
                            isReconnecting = false,
                            lastServerIp = ipAddress.trim(),
                            currentScreen = Screen.Dashboard,
                        )

                    client.setOnConnectionLostListener {
                        viewModelScope.launch {
                            handleConnectionLost()
                        }
                    }

                    // Start listening to messages
                    launch {
                        client.messages.collect { response ->
                            handleResponse(response)
                        }
                    }

                    refreshWakes()
                } else {
                    _state.value =
                        _state.value.copy(
                            isConnected = false,
                            errorMessage = "Connection failed: ${connectResult.exceptionOrNull()?.message}",
                        )
                }
            } catch (e: Exception) {
                _state.value =
                    _state.value.copy(
                        isConnected = false,
                        errorMessage = "Error: ${e.message}",
                    )
            }
        }
    }

    private fun handleConnectionLost() {
        _state.value =
            _state.value.copy(
                isConnected = false,
                isConnectionLost = true,
            )
        attemptAutoReconnect()
    }

    fun reconnect() {
        viewModelScope.launch {
            attemptReconnect(maxAttempts = 1)
        }
    }

    private fun attemptAutoReconnect() {
        viewModelScope.launch {
            attemptReconnect(maxAttempts = 3)
        }
    }

    private suspend fun attemptReconnect(maxAttempts: Int) {
        val address = _state.value.serverAddress
        if (address.isBlank()) return

        _state.value = _state.value.copy(isReconnecting = true)

        for (attempt in 1..maxAttempts) {
            if (_state.value.isConnected && !_state.value.isConnectionLost) {
                _state.value = _state.value.copy(isReconnecting = false)
                return
            }
            if (attempt > 1) {
                delay(2000)
            }
            try {
                val (host, port) = parseHostPort(address)
                val client = TcpClient(host, port)
                val connectResult = client.connect()
                if (connectResult.isSuccess) {
                    tcpClient = client
                    persistServerIp(address.trim())
                    _state.value =
                        _state.value.copy(
                            isConnected = true,
                            isConnectionLost = false,
                            isReconnecting = false,
                            lastServerIp = address.trim(),
                            errorMessage = null,
                        )

                    client.setOnConnectionLostListener {
                        viewModelScope.launch {
                            handleConnectionLost()
                        }
                    }

                    viewModelScope.launch {
                        client.messages.collect { response ->
                            handleResponse(response)
                        }
                    }

                    refreshWakes()
                    return
                }
            } catch (_: Exception) {
            }
        }

        _state.value =
            _state.value.copy(
                isReconnecting = false,
                errorMessage = "Connection to $address lost. Tap Reconnect to try again.",
            )
    }

    private fun handleResponse(response: ServerResponse) {
        when (response) {
            is ServerResponse.ModelsList -> {
                _state.value =
                    _state.value.copy(
                        availableModels = ProviderCatalog.merge(response.models),
                    )
            }
            is ServerResponse.TaskStarted -> {
                _state.value =
                    _state.value.copy(
                        taskResult = "Task Started...\n",
                        isExecutingTask = true,
                    )
            }
            is ServerResponse.TaskEvent -> {
                val formatted = "\n[${response.source}] ${response.event_type}: ${response.content}"
                _state.value =
                    _state.value.copy(
                        taskResult = _state.value.taskResult + formatted,
                        activeEvents = _state.value.activeEvents + response,
                    )
            }
            is ServerResponse.TaskComplete -> {
                val newResult = _state.value.taskResult + "\n\nTask Complete: ${response.result}"
                _state.value =
                    _state.value.copy(
                        taskResult = newResult,
                        isExecutingTask = false,
                    )
            }
            is ServerResponse.PendingWakesList -> {
                _state.value =
                    _state.value.copy(
                        pendingWakes = response.wakes,
                    )
            }
            is ServerResponse.WakeResolved -> {
                refreshWakes()
            }
            is ServerResponse.AgentResourcesList -> {
                val resolved = response.work_dir.trim()
                _state.value =
                    _state.value.copy(
                        agentResources =
                            AgentResources(
                                prompts = response.prompts,
                                rules = response.rules,
                                workflows = response.workflows,
                                skills = response.skills,
                            ),
                        workDir = resolved.ifEmpty { _state.value.workDir },
                    )
            }
            is ServerResponse.Error -> {
                _state.value =
                    _state.value.copy(
                        errorMessage = "Server Error: ${response.message}",
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
        tcpClient = null
        val preservedHost = _state.value.lastServerIp.ifBlank { prefs.getString(PREF_LAST_SERVER_IP, "") ?: "" }
        _state.value = AppState(lastServerIp = preservedHost)
    }

    fun navigateTo(screen: Screen) {
        _state.value = _state.value.copy(currentScreen = screen)
    }

    fun updateRole(
        index: Int,
        role: RoleConfig,
    ) {
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
                ModelConfig("openai", "gpt-4o-mini"),
            ),
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
                _state.value =
                    _state.value.copy(
                        isExecutingTask = true,
                        errorMessage = null,
                        taskResult = "",
                    )

                val config =
                    AgentConfig(
                        roles = _state.value.selectedRoles,
                        work_dir = _state.value.workDir,
                        mcp_config = _state.value.mcpConfig,
                    )

                val result = tcpClient?.startTask(config, _state.value.task)

                if (result?.isFailure == true) {
                    _state.value =
                        _state.value.copy(
                            errorMessage = "Failed to start task: ${result.exceptionOrNull()?.message}",
                            isExecutingTask = false,
                        )
                }
            } catch (e: Exception) {
                _state.value =
                    _state.value.copy(
                        errorMessage = "Error: ${e.message}",
                        isExecutingTask = false,
                    )
            }
        }
    }

    fun cancelTask() {
        viewModelScope.launch {
            try {
                tcpClient?.cancelTask()
                _state.value =
                    _state.value.copy(
                        isExecutingTask = false,
                        taskResult = _state.value.taskResult + "\nCancelled by user.",
                    )
            } catch (e: Exception) {
                _state.value =
                    _state.value.copy(
                        errorMessage = "Failed to cancel: ${e.message}",
                    )
            }
        }
    }

    fun refreshWakes() {
        viewModelScope.launch {
            tcpClient?.getPendingWakes()
        }
    }

    fun resolveWake(
        wakeId: String,
        approve: Boolean,
    ) {
        viewModelScope.launch {
            tcpClient?.resolveWake(wakeId, approve)
        }
    }

    fun fetchModelsAndNavigate() {
        viewModelScope.launch {
            tcpClient?.getModels()
            tcpClient?.getAgentResources()
            _state.value = _state.value.copy(currentScreen = Screen.ModelSelection)
        }
    }

    override fun onCleared() {
        super.onCleared()
        tcpClient?.disconnect()
    }

    companion object {
        private const val PREFS_NAME = "ca_remote_prefs"
        private const val PREF_LAST_SERVER_IP = "last_server_ip"
    }
}
