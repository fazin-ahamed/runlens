using System;
using System.Net.WebSockets;
using System.Threading;
using System.Threading.Tasks;
using UnityEngine;

namespace RunLens
{
    public class DaemonConnection : IDisposable
    {
        public event Action OnConnected;
        public event Action<string> OnDisconnected;
        public event Action<string> OnError;

        private ClientWebSocket _ws;
        private CancellationTokenSource _cts;
        private string _url;
        private bool _destroyed;
        private int _reconnectAttempts;
        private int _maxReconnectAttempts = 10;
        private float _reconnectDelay = 1f;
        private float _maxReconnectDelay = 30f;

        public bool IsConnected =>
            _ws?.State == WebSocketState.Open;

        public DaemonConnection(string url = "ws://localhost:9876")
        {
            _url = url;
        }

        public async Task ConnectAsync()
        {
            if (_destroyed) return;
            _cts = new CancellationTokenSource();
            _ws = new ClientWebSocket();
            try
            {
                await _ws.ConnectAsync(new Uri(_url), _cts.Token);
                _reconnectAttempts = 0;
                _reconnectDelay = 1f;
                OnConnected?.Invoke();
                _ = ReceiveLoopAsync();
            }
            catch (Exception ex)
            {
                OnError?.Invoke(ex.Message);
                _ws?.Dispose();
                _ws = null;
                ScheduleReconnect();
            }
        }

        public async Task SendAsync(byte[] data)
        {
            if (_ws?.State != WebSocketState.Open) return;
            await _ws.SendAsync(new ArraySegment<byte>(data),
                WebSocketMessageType.Text, true,
                _cts?.Token ?? CancellationToken.None);
        }

        public async Task<string> ReceiveAsync()
        {
            var buffer = new byte[8192];
            var result = await _ws.ReceiveAsync(
                new ArraySegment<byte>(buffer),
                _cts?.Token ?? CancellationToken.None);
            return System.Text.Encoding.UTF8.GetString(buffer, 0, result.Count);
        }

        private async Task ReceiveLoopAsync()
        {
            var buffer = new byte[65536];
            try
            {
                while (_ws?.State == WebSocketState.Open && !_cts.IsCancellationRequested)
                {
                    var result = await _ws.ReceiveAsync(
                        new ArraySegment<byte>(buffer), _cts.Token);
                    if (result.MessageType == WebSocketMessageType.Close)
                        break;
                }
            }
            catch (OperationCanceledException) { }
            catch (Exception ex)
            {
                OnError?.Invoke(ex.Message);
            }
            finally
            {
                if (!_destroyed)
                {
                    OnDisconnected?.Invoke("Connection closed");
                    ScheduleReconnect();
                }
            }
        }

        private async void ScheduleReconnect()
        {
            if (_destroyed) return;
            _reconnectAttempts++;
            if (_reconnectAttempts > _maxReconnectAttempts) return;
            await Task.Delay((int)(_reconnectDelay * 1000));
            _reconnectDelay = Math.Min(_reconnectDelay * 2, _maxReconnectDelay);
            if (!_destroyed) await ConnectAsync();
        }

        public void Disconnect()
        {
            _destroyed = true;
            _cts?.Cancel();
            _ws?.CloseAsync(WebSocketCloseStatus.NormalClosure, "", CancellationToken.None);
            _ws?.Dispose();
            _ws = null;
        }

        public void Dispose()
        {
            Disconnect();
            _cts?.Dispose();
        }
    }
}
