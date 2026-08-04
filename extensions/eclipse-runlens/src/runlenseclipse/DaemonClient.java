package runlenseclipse;

import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.WebSocket;
import java.util.Map;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.CopyOnWriteArrayList;
import java.util.concurrent.atomic.AtomicInteger;

import com.google.gson.Gson;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;

public class DaemonClient {
    private final String url;
    private final HttpClient client = HttpClient.newHttpClient();
    private final Gson gson = new Gson();
    private final AtomicInteger nextId = new AtomicInteger(1);
    private final ConcurrentHashMap<Integer, PendingRequest> pending = new ConcurrentHashMap<>();
    private final CopyOnWriteArrayList<DaemonListener> listeners = new CopyOnWriteArrayList<>();
    private volatile WebSocket ws;

    public interface DaemonListener {
        default void onConnected() {}
        default void onDisconnected() {}
        default void onEvent(String method, JsonObject params) {}
    }

    private static class PendingRequest {
        final CompletableFuture<JsonElement> future;
        PendingRequest(CompletableFuture<JsonElement> future) { this.future = future; }
    }

    public DaemonClient(String url) { this.url = url; }

    public CompletableFuture<Void> connect() {
        CompletableFuture<Void> future = new CompletableFuture<>();
        client.newWebSocketBuilder()
            .buildAsync(URI.create(url), new WebSocket.Listener() {
                @Override
                public void onOpen(WebSocket webSocket) {
                    ws = webSocket;
                    listeners.forEach(DaemonListener::onConnected);
                    future.complete(null);
                }

                @Override
                public void onText(WebSocket webSocket, CharSequence data, boolean last) {
                    String msg = data.toString();
                    JsonObject json = JsonParser.parseString(msg).getAsJsonObject();
                    Integer id = json.has("id") ? json.get("id").getAsInt() : null;
                    if (id != null) {
                        PendingRequest pr = pending.remove(id);
                        if (pr != null) {
                            if (json.has("error")) {
                                pr.future.completeExceptionally(
                                    new RuntimeException(json.get("error").toString()));
                            } else {
                                pr.future.complete(json.get("result"));
                            }
                        }
                    } else {
                        String method = json.has("method") ? json.get("method").getAsString() : "";
                        JsonObject params = json.has("params") ? json.getAsJsonObject("params") : new JsonObject();
                        for (DaemonListener l : listeners) l.onEvent(method, params);
                    }
                }

                @Override
                public void onError(WebSocket webSocket, Throwable error) {
                    future.completeExceptionally(error);
                }

                @Override
                public void onClose(WebSocket webSocket, int statusCode, String reason) {
                    ws = null;
                    listeners.forEach(DaemonListener::onDisconnected);
                }
            });
        return future;
    }

    public void disconnect() {
        if (ws != null) ws.sendClose(1000, "client shutdown");
        ws = null;
    }

    public CompletableFuture<JsonElement> call(String method) {
        return call(method, Map.of());
    }

    public CompletableFuture<JsonElement> call(String method, Object params) {
        int id = nextId.getAndIncrement();
        CompletableFuture<JsonElement> future = new CompletableFuture<>();
        pending.put(id, new PendingRequest(future));
        Map<String, Object> req = Map.of(
            "jsonrpc", "2.0", "id", id, "method", method, "params", params);
        try {
            ws.sendText(gson.toJson(req), true);
        } catch (Exception e) {
            pending.remove(id);
            future.completeExceptionally(e);
        }
        return future;
    }

    public void addListener(DaemonListener listener) { listeners.add(listener); }
    public void removeListener(DaemonListener listener) { listeners.remove(listener); }
}