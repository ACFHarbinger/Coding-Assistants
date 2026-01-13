package com.example.remotelauncher

import io.ktor.network.selector.*
import io.ktor.network.sockets.*
import io.ktor.utils.io.*
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json

class TcpClient {
    private val selectorManager = SelectorManager(Dispatchers.IO)
    private var socket: Socket? = null
    private var receiveChannel: ByteReadChannel? = null
    private var sendChannel: ByteWriteChannel? = null

    var onDisconnected: (() -> Unit)? = null

    private val json = Json {
        ignoreUnknownKeys = true
        encodeDefaults = true
        classDiscriminator = "type"
    }

    private val _events = MutableSharedFlow<ServerResponse>()
    val events = _events.asSharedFlow()

    suspend fun connect(ip: String, port: Int = 5555) {
        withContext(Dispatchers.IO) {
            socket = aSocket(selectorManager).tcp().connect(ip, port)
            receiveChannel = socket?.openReadChannel()
            sendChannel = socket?.openWriteChannel(autoFlush = true)

            launch { listen() }
        }
    }

    private suspend fun listen() {
        try {
            while (true) {
                val line = receiveChannel?.readUTF8Line() ?: break
                if (line.isEmpty()) continue

                try {
                    val response = json.decodeFromString<ServerResponse>(line)
                    _events.emit(response)
                } catch (e: Exception) {
                    println("Failed to decode: $e")
                }
            }
        } catch (e: Exception) {
            println("Listen error: $e")
        } finally {
            disconnect()
            onDisconnected?.invoke()
        }
    }

    suspend fun send(request: ClientRequest) {
        val line = json.encodeToString(request)
        sendChannel?.writeStringUtf8("$line\n")
    }

    fun disconnect() {
        socket?.close()
        socket = null
        receiveChannel = null
        sendChannel = null
    }
}
