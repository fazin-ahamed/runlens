using System.Threading.Tasks;
using UnityEditor;
using UnityEngine;

namespace RunLens.Editor
{
    public class RunLensEditorWindow : EditorWindow
    {
        private Runtime.RunLensClient _client;
        private Runtime.RunLensSession _session;
        private string[] _sessions = System.Array.Empty<string>();
        private Vector2 _scroll;
        private bool _connecting;

        [MenuItem("Window/RunLens/Session Manager")]
        public static void Open()
        {
            var w = GetWindow<RunLensEditorWindow>("RunLens");
            w.minSize = new Vector2(350, 250);
            w.Show();
        }

        void OnEnable()
        {
            _client = new Runtime.RunLensClient();
            _session = new Runtime.RunLensSession(_client);
            _ = TryConnectAsync();
        }

        async Task TryConnectAsync()
        {
            _connecting = true;
            Repaint();
            try
            {
                await _client.ConnectAsync();
                await RefreshSessionsAsync();
            }
            catch { }
            _connecting = false;
            Repaint();
        }

        async Task RefreshSessionsAsync()
        {
            if (!_client.IsConnected) return;
            _sessions = await _session.ListSessionsAsync();
            Repaint();
        }

        void OnGUI()
        {
            if (_connecting)
            {
                EditorGUILayout.LabelField("Connecting to RunLens daemon...", EditorStyles.boldLabel);
                return;
            }

            if (!_client.IsConnected)
            {
                EditorGUILayout.HelpBox("RunLens daemon not connected. Start with 'runlens daemon'.", MessageType.Warning);
                if (GUILayout.Button("Retry"))
                    _ = TryConnectAsync();
                return;
            }

            EditorGUILayout.BeginHorizontal();
            EditorGUILayout.LabelField("RunLens Session Manager", EditorStyles.boldLabel);

            if (_session.IsRecording)
            {
                if (GUILayout.Button("Stop Recording", GUILayout.Width(120)))
                    _ = StopRecordingAsync();
            }
            else
            {
                if (GUILayout.Button("Start Recording", GUILayout.Width(120)))
                    _ = StartRecordingAsync();
            }
            EditorGUILayout.EndHorizontal();

            EditorGUILayout.Space();

            if (_session.IsRecording)
            {
                EditorGUILayout.LabelField("Recording...", EditorStyles.miniLabel);
                EditorGUILayout.Space();
            }

            if (GUILayout.Button("Refresh"))
                _ = RefreshSessionsAsync();

            _scroll = EditorGUILayout.BeginScrollView(_scroll);
            foreach (var s in _sessions)
            {
                EditorGUILayout.BeginHorizontal();
                EditorGUILayout.LabelField(s.Length > 8 ? s[..8] : s);
                if (GUILayout.Button("Verify", GUILayout.Width(60)))
                    _ = VerifySessionAsync(s);
                EditorGUILayout.EndHorizontal();
            }
            EditorGUILayout.EndScrollView();
        }

        async Task StartRecordingAsync()
        {
            await _session.StartRecordingAsync($"unity-editor:{Application.productName}");
            Repaint();
        }

        async Task StopRecordingAsync()
        {
            await _session.StopRecordingAsync();
            await RefreshSessionsAsync();
        }

        async Task VerifySessionAsync(string id)
        {
            var result = await _session.VerifySessionAsync(id);
            EditorUtility.DisplayDialog("Verification Result",
                string.IsNullOrEmpty(result) ? "Session verified OK" : result, "OK");
        }

        void OnDisable()
        {
            _client?.Disconnect();
        }
    }
}
