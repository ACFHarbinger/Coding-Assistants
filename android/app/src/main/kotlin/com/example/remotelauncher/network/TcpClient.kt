package com.example.remotelauncher.network

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch
import kotlinx.coroutines.isActive
import kotlinx.coroutines.cancelChildren
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import java.io.BufferedReader
import java.io.InputStreamReader
import java.io.PrintWriter
import java.net.Socket

@Serializable
sealed class ClientRequest {
    @Serializable
    @SerialName("GetModels")
    class GetModels : ClientRequest()
    
    @Serializable
    @SerialName("StartTask")
    data class StartTask(
        val config: AgentConfig,
        val task: String
    ) : ClientRequest()
    
    @Serializable
    @SerialName("CancelTask")
    class CancelTask : ClientRequest()
    
    @Serializable
    @SerialName("SubmitInput")
    data class SubmitInput(
        val input: String
    ) : ClientRequest()
    
    @Serializable
    @SerialName("GetStatus")
    class GetStatus : ClientRequest()
}

@Serializable
data class AgentConfig(
    val roles: List<RoleConfig>,
    val work_dir: String,
    val mcp_config: String
)

@Serializable
data class RoleConfig(
    val name: String,
    val config: ModelConfig
)

@Serializable
data class ModelConfig(
    val provider: String,
    val model: String,
    val prompt_file: String? = null,
    val rule_file: String? = null,
    val workflow_file: String? = null
)

@Serializable
sealed class ServerResponse {
    @Serializable
    @SerialName("ModelsList")
    data class ModelsList(
        val models: Map<String, List<String>>
    ) : ServerResponse()
    
    @Serializable
    @SerialName("TaskStarted")
    class TaskStarted : ServerResponse()
    
    @Serializable
    @SerialName("TaskEvent")
    data class TaskEvent(
        val source: String,
        val event_type: String,
        val content: String
    ) : ServerResponse()
    
    @Serializable
    @SerialName("TaskComplete")
    data class TaskComplete(
        val result: String
    ) : ServerResponse()
    
    @Serializable
    @SerialName("Status")
    data class Status(
        val running: Boolean,
        val message: String
    ) : ServerResponse()
    
    @Serializable
    @SerialName("Error")
    data class Error(
        val message: String
    ) : ServerResponse()
}

class TcpClient(private val host: String, private val port: Int = 5555) {
    private var socket: Socket? = null
    private var writer: PrintWriter? = null
    private var reader: BufferedReader? = null
    
    private val scope = CoroutineScope(Dispatchers.IO + SupervisorJob())
    
    private val _messages = MutableSharedFlow<ServerResponse>()
    val messages = _messages.asSharedFlow()
    
    private val json = Json {
        ignoreUnknownKeys = true
        encodeDefaults = true
    }
    
    suspend fun connect(): Result<Unit> = withContext(Dispatchers.IO) {
        try {
            socket = Socket(host, port)
            writer = PrintWriter(socket!!.getOutputStream(), true)
            reader = BufferedReader(InputStreamReader(socket!!.getInputStream()))
            
            startListening()
            
            Result.success(Unit)
        } catch (e: Exception) {
            Result.failure(e)
        }
    }
    
    private fun startListening() {
        scope.launch {
            try {
                while (isActive) {
                    val line = reader?.readLine() ?: break
                    if (line.isBlank()) continue
                    
                    try {
                        val response = json.decodeFromString<ServerResponse>(line)
                        _messages.emit(response)
                    } catch (e: Exception) {
                        e.printStackTrace()
                    }
                }
            } catch (e: Exception) {
                e.printStackTrace()
            } finally {
                disconnect()
            }
        }
    }
    
    suspend fun sendRequest(request: ClientRequest): Result<Unit> = withContext(Dispatchers.IO) {
        try {
            val jsonString = json.encodeToString(request)
            writer?.println(jsonString)
            Result.success(Unit)
        } catch (e: Exception) {
            Result.failure(e)
        }
    }
    
    suspend fun getModels(): Result<Unit> = sendRequest(ClientRequest.GetModels())
    
    suspend fun startTask(config: AgentConfig, task: String): Result<Unit> = 
        sendRequest(ClientRequest.StartTask(config = config, task = task))
    
    suspend fun cancelTask(): Result<Unit> = sendRequest(ClientRequest.CancelTask())
    
    suspend fun submitInput(input: String): Result<Unit> = sendRequest(ClientRequest.SubmitInput(input = input))
    
    fun disconnect() {
        try {
            scope.coroutineContext.cancelChildren()
            writer?.close()
            reader?.close()
            socket?.close()
        } catch (e: Exception) {
            e.printStackTrace()
        }
    }
    
    fun isConnected(): Boolean = socket?.isConnected == true && socket?.isClosed == false
}
