package runlens

import com.google.gson.Gson
import com.google.gson.JsonElement
import com.google.gson.JsonObject
import com.google.gson.JsonParser
import java.net.URI
import java.net.http.HttpClient
import java.net.http.WebSocket
import java.util.concurrent.CancellationException
import java.util.concurrent.CompletableFuture
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.CopyOnWriteArrayList

class DaemonClient(private val url: String = "ws://localhost:9876") {
    private val gson = Gson()
    private var ws: WebSocket? = null
    private val pending = ConcurrentHashMap<Int, PendingRequest>()
    private var nextId = 1
    private val listeners = CopyOnWriteArrayList<DaemonListener>()
    private var destroyed = false

    data class PendingRequest(val resolve: (JsonElement?) -> Unit, val reject: (Throwable) -> Unit)

    interface DaemonListener {
        fun onConnected() {}
        fun onDisconnected() {}
        fun onEvent(method: String, params: JsonObject) {}
    }

    fun connect(): CompletableFuture<Void> {
        val future = CompletableFuture<Void>()
        destroyed = false
        val client = HttpClient.newBuilder().build()
        client.newWebSocketBuilder()
            .buildAsync(URI.create(url), object : WebSocket.Listener {
                override fun onOpen(webSocket: WebSocket) {
                    ws = webSocket
                    listeners.forEach { it.onConnected() }
                    future.complete(null)
                    WebSocket.Listener.super.onOpen(webSocket)
                }

                override fun onText(webSocket: WebSocket, data: CharSequence, last: Boolean) {
                    if (destroyed) return
                    val message = data.toString()
                    val json = JsonParser.parseString(message).asJsonObject
                    val id = json.get("id")?.asInt
                    if (id != null) {
                        val pending = pending.remove(id) ?: return
                        val error = json.get("error")
                        if (error != null) {
                            pending.reject(RuntimeException(error.toString()))
                        } else {
                            pending.resolve(json.get("result"))
                        }
                    } else {
                        listeners.forEach { it.onEvent(
                            json.get("method")?.asString ?: "",
                            json.getAsJsonObject("params") ?: JsonObject()
                        ) }
                    }
                }

                override fun onError(webSocket: WebSocket, error: Throwable) {
                    if (!destroyed) future.completeExceptionally(error)
                }

                override fun onClose(webSocket: WebSocket, statusCode: Int, reason: String) {
                    ws = null
                    listeners.forEach { it.onDisconnected() }
                }
            })
        return future
    }

    fun disconnect() {
        destroyed = true
        ws?.sendClose(1000, "client shutdown")
        ws = null
    }

    fun call(method: String, params: Any = mapOf<String, Any>()): CompletableFuture<JsonElement?> {
        val id = nextId++
        val future = CompletableFuture<JsonElement?>()
        pending[id] = PendingRequest({ future.complete(it) }, { future.completeExceptionally(it) })
        val request = mapOf(
            "jsonrpc" to "2.0",
            "id" to id,
            "method" to method,
            "params" to params
        )
        try {
            ws?.sendText(gson.toJson(request), true)
        } catch (e: Exception) {
            pending.remove(id)
            future.completeExceptionally(e)
        }
        return future
    }

    fun addListener(listener: DaemonListener) {
        listeners.add(listener)
    }

    fun removeListener(listener: DaemonListener) {
        listeners.remove(listener)
    }
}