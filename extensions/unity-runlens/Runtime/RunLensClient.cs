using System;
using System.Collections.Concurrent;
using System.Text;
using System.Threading.Tasks;
using UnityEngine;

namespace RunLens
{
    public class RunLensClient : IDisposable
    {
        private DaemonConnection _connection;
        private int _nextId = 1;
        private ConcurrentDictionary<int, TaskCompletionSource<string>> _pending =
            new ConcurrentDictionary<int, TaskCompletionSource<string>>();

        public event Action<string> OnNotification;
        public bool IsConnected => _connection?.IsConnected ?? false;

        public RunLensClient(string url = "ws://localhost:9876")
        {
            _connection = new DaemonConnection(url);
            _connection.OnConnected += () => Debug.Log("[RunLens] Connected to daemon");
            _connection.OnDisconnected += (reason) => Debug.Log($"[RunLens] Disconnected: {reason}");
            _connection.OnError += (err) => Debug.LogError($"[RunLens] Error: {err}");
        }

        public async Task ConnectAsync()
        {
            await _connection.ConnectAsync();
            _ = NotificationLoopAsync();
        }

        public async Task<T> CallAsync<T>(string method, object args = null)
        {
            var id = _nextId++;
            var tcs = new TaskCompletionSource<string>();
            _pending.TryAdd(id, tcs);

            var msg = JsonUtility.ToJson(new JsonRpcRequest
            {
                jsonrpc = "2.0",
                id = id,
                method = method,
                @params = args
            });
            await _connection.SendAsync(Encoding.UTF8.GetBytes(msg));

            var result = await tcs.Task;
            return JsonUtility.FromJson<T>(result);
        }

        public async Task CallAsync(string method, object args = null)
        {
            var id = _nextId++;
            var tcs = new TaskCompletionSource<string>();
            _pending.TryAdd(id, tcs);

            var msg = JsonUtility.ToJson(new JsonRpcRequest
            {
                jsonrpc = "2.0",
                id = id,
                method = method,
                @params = args
            });
            await _connection.SendAsync(Encoding.UTF8.GetBytes(msg));
            await tcs.Task;
        }

        private async Task NotificationLoopAsync()
        {
            while (_connection.IsConnected)
            {
                try
                {
                    var data = await _connection.ReceiveAsync();
                    ProcessMessage(data);
                }
                catch { break; }
            }
        }

        private void ProcessMessage(string data)
        {
            var msg = JsonUtility.FromJson<JsonRpcMessage>(data);
            if (!string.IsNullOrEmpty(msg.method))
            {
                OnNotification?.Invoke(data);
                return;
            }
            if (_pending.TryRemove(msg.id, out var tcs))
            {
                if (!string.IsNullOrEmpty(msg.error))
                    tcs.TrySetException(new Exception(msg.error));
                else
                    tcs.TrySetResult(msg.result);
            }
        }

        public void Disconnect() => _connection?.Disconnect();
        public void Dispose() { Disconnect(); _connection?.Dispose(); }
    }

    [Serializable]
    internal class JsonRpcRequest
    {
        public string jsonrpc = "2.0";
        public int id;
        public string method;
        public object @params;
    }

    [Serializable]
    internal class JsonRpcMessage
    {
        public int id;
        public string method;
        public string result;
        public string error;
        public string @params;
    }
}
