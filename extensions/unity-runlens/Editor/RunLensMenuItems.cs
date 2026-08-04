using UnityEditor;
using UnityEngine;

namespace RunLens.Editor
{
    public static class RunLensMenuItems
    {
        [MenuItem("Window/RunLens/Session Manager")]
        public static void ShowWindow()
        {
            RunLensEditorWindow.Open();
        }

        [MenuItem("Tools/RunLens/Start Recording", false, 100)]
        public static async void StartRecording()
        {
            var client = new Runtime.RunLensClient();
            await client.ConnectAsync();
            var session = new Runtime.RunLensSession(client);
            var id = await session.StartRecordingAsync($"unity-editor:{Application.productName}");
            Debug.Log($"[RunLens] Recording started: {id}");
        }

        [MenuItem("Tools/RunLens/Start Recording", true)]
        public static bool ValidateStartRecording()
        {
            return !EditorApplication.isPlaying;
        }

        [MenuItem("Tools/RunLens/Create Settings", false, 200)]
        public static void CreateSettings()
        {
            var path = EditorUtility.SaveFilePanelInProject(
                "Save RunLens Settings", "RunLensSettings", "asset", "");
            if (string.IsNullOrEmpty(path)) return;
            var settings = ScriptableObject.CreateInstance<Runtime.RunLensSettings>();
            AssetDatabase.CreateAsset(settings, path);
            AssetDatabase.SaveAssets();
            EditorUtility.FocusProjectWindow();
            Selection.activeObject = settings;
        }
    }
}
