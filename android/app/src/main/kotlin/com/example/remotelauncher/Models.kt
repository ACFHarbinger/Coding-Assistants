package com.example.remotelauncher

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable class RoleConfig(val name: String, val config: ModelConfig)

@Serializable
class ModelConfig(
        val provider: String,
        val model: String,
        val prompt_file: String? = null,
        val rule_file: String? = null,
        val workflow_file: String? = null
)

@Serializable
class AgentConfig(
        val roles: List<RoleConfig>,
        val work_dir: String = "./workspace",
        val mcp_config: String = ""
)

@Serializable
sealed class ClientRequest {
    @Serializable @SerialName("GetModels") object GetModels : ClientRequest()

    @Serializable
    @SerialName("StartTask")
    data class StartTask(val config: AgentConfig, val task: String) : ClientRequest()

    @Serializable @SerialName("CancelTask") object CancelTask : ClientRequest()

    @Serializable
    @SerialName("SubmitInput")
    data class SubmitInput(val input: String) : ClientRequest()

    @Serializable @SerialName("GetStatus") object GetStatus : ClientRequest()
}

@Serializable
sealed class ServerResponse {
    @Serializable
    @SerialName("ModelsList")
    data class ModelsList(val models: Map<String, List<String>>) : ServerResponse()

    @Serializable @SerialName("TaskStarted") object TaskStarted : ServerResponse()

    @Serializable
    @SerialName("TaskEvent")
    data class TaskEvent(val source: String, val event_type: String, val content: String) :
            ServerResponse()

    @Serializable
    @SerialName("TaskComplete")
    data class TaskComplete(val result: String) : ServerResponse()

    @Serializable
    @SerialName("Status")
    data class Status(val running: Boolean, val message: String) : ServerResponse()

    @Serializable @SerialName("Error") data class Error(val message: String) : ServerResponse()
}
