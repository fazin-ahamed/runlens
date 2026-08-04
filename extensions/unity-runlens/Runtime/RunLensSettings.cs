using UnityEngine;

namespace RunLens
{
    [CreateAssetMenu(fileName = "RunLensSettings", menuName = "RunLens/Settings")]
    public class RunLensSettings : ScriptableObject
    {
        [Header("Daemon")]
        public string daemonUrl = "ws://localhost:9876";
        public bool autoConnect = true;
        public int maxReconnectAttempts = 10;

        [Header("Recording")]
        public bool recordOnPlay = false;
        public bool captureLogMessages = true;
        public bool captureSceneChanges = true;

        [Header("Session")]
        public int listLimit = 20;
    }
}
