using UnityEngine;
using UnityEngine.SceneManagement;

namespace RunLens
{
    public class RunLensRecorder : MonoBehaviour
    {
        public RunLensSettings settings;
        private RunLensClient _client;
        private RunLensSession _session;

        void Awake()
        {
            if (settings == null)
                settings = Resources.Load<RunLensSettings>("RunLensSettings");
            _client = new RunLensClient(settings?.daemonUrl ?? "ws://localhost:9876");
            _session = new RunLensSession(_client);
        }

        async void Start()
        {
            if (settings != null && settings.autoConnect)
            {
                await _client.ConnectAsync();
                if (settings.recordOnPlay)
                    await _session.StartRecordingAsync($"unity-scene:{SceneManager.GetActiveScene().name}");
            }
            if (settings == null || settings.captureLogMessages)
                Application.logMessageReceived += OnLogMessage;
            if (settings == null || settings.captureSceneChanges)
                SceneManager.activeSceneChanged += OnSceneChanged;
        }

        void OnLogMessage(string logString, string stackTrace, LogType type)
        {
            var level = type switch
            {
                LogType.Error => "error",
                LogType.Assert => "error",
                LogType.Warning => "warn",
                LogType.Exception => "error",
                _ => "info"
            };
            Debug.unityLogger.Log($"[RunLens] log:{level}:{logString}");
        }

        void OnSceneChanged(Scene prev, Scene next)
        {
            Debug.unityLogger.Log($"[RunLens] scene:{next.name}");
        }

        void OnDestroy()
        {
            Application.logMessageReceived -= OnLogMessage;
            SceneManager.activeSceneChanged -= OnSceneChanged;
            _client?.Disconnect();
        }

        public async void ToggleRecording()
        {
            if (_session.IsRecording)
            {
                await _session.StopRecordingAsync();
                Debug.Log("[RunLens] Recording stopped");
            }
            else
            {
                var id = await _session.StartRecordingAsync(
                    $"unity-scene:{SceneManager.GetActiveScene().name}");
                Debug.Log($"[RunLens] Recording started: {id}");
            }
        }
    }
}
