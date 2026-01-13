package com.example.remotelauncher.network

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
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
    data class GetModels(val type: String = "GetModels") : ClientRequest()
    
    @Serializable
    data class StartTask(
        val type: String = "StartTask",
        val config: AgentConfig,
        val task: String
    ) : ClientRequest()
    
    @Serializable
    data class CancelTask(val type: String = "CancelTask") : ClientRequest()
    
    @Serializable
    data class SubmitInput(
        val type: String = "SubmitInput",
        val input: String
    ) : ClientRequest()
    
    @Serializable
    data class GetStatus(val type: String = "GetStatus") : ClientRequest()
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
    data class ModelsList(
        val type: String = "ModelsList",
        val models: Map<String, List<String>>
    ) : ServerResponse()
    
    @Serializable
    data class TaskStarted(val type: String = "TaskStarted") : ServerResponse()
    
    @Serializable
    data class TaskEvent(
        val type: String = "TaskEvent",
        val source: String,
        val event_type: String,
        val content: String
    ) : ServerResponse()
    
    @Serializable
    data class TaskComplete(
        val type: String = "TaskComplete",
        val result: String
    ) : ServerResponse()
    
    @Serializable
    data class Status(
        val type: String = "Status",
        val running: Boolean,
        val message: String
    ) : ServerResponse()
    
    @Serializable
    data class Error(
        val type: String = "Error",
        val message: String
    ) : ServerResponse()
}

class TcpClient(private val host: String, private val port: Int = 5555) {
    private var socket: Socket? = null
    private var writer: PrintWriter? = null
    private var reader: BufferedReader? = null
    
    private val json = Json {
        ignoreUnknownKeys = true
        encodeDefaults = true
    }
    
    suspend fun connect(): Result<Unit> = withContext(Dispatchers.IO) {
        try {
            socket = Socket(host, port)
            writer = PrintWriter(socket!!.getOutputStream(), true)
            reader = BufferedReader(InputStreamReader(socket!!.getInputStream()))
            Result.success(Unit)
        } catch (e: Exception) {
            Result.failure(e)
        }
    }
    
    suspend fun sendRequest(request: ClientRequest): Result<ServerResponse> = withContext(Dispatchers.IO) {
        try {
            val jsonString = when (request) {
                is ClientRequest.GetModels -> json.encodeToString(request)
                is ClientRequest.StartTask -> json.encodeToString(request)
                is ClientRequest.CancelTask -> json.encodeToString(request)
                is ClientRequest.SubmitInput -> json.encodeToString(request)
                is ClientRequest.GetStatus -> json.encodeToString(request)
            }
            
            writer?.println(jsonString)
            
            val response = reader?.readLine()
                ?: return@withContext Result.failure(Exception("No response from server"))
            
            val serverResponse = json.decodeFromString<ServerResponse>(response)
            Result.success(serverResponse)
        } catch (e: Exception) {
            Result.failure(e)
        }
    }
    
    suspend fun getModels(): Result<Map<String, List<String>>> = withContext(Dispatchers.IO) {
        sendRequest(ClientRequest.GetModels()).mapCatching { response ->
            (response as? ServerResponse.ModelsList)?.models
                ?: throw Exception("Unexpected response type")
        }
    }
    
    suspend fun startTask(config: AgentConfig, task: String): Result<Unit> = withContext(Dispatchers.IO) {
        sendRequest(ClientRequest.StartTask(config = config, task = task)).map {}
    }
    
    suspend fun cancelTask(): Result<Unit> = withContext(Dispatchers.IO) {
        sendRequest(ClientRequest.CancelTask()).map {}
    }
    
    suspend fun submitInput(input: String): Result<Unit> = withContext(Dispatchers.IO) {
        sendRequest(ClientRequest.SubmitInput(input = input)).map {}
    }
    
    fun disconnect() {
        try {
            writer?.close()
            reader?.close()
            socket?.close()
        } catch (e: Exception) {
            e.printStackTrace()
        }
    }
    
    fun isConnected(): Boolean = socket?.isConnected == true && socket?.isClosed == false
}
