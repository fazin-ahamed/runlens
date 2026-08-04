using System;
using System.Threading.Tasks;

namespace RunLens
{
    public class RunLensSession
    {
        private RunLensClient _client;

        public string ActiveSessionId { get; private set; }
        public bool IsRecording => ActiveSessionId != null;

        public RunLensSession(RunLensClient client)
        {
            _client = client;
        }

        public async Task<string[]> ListSessionsAsync(int limit = 20)
        {
            var result = await _client.CallAsync<ListSessionsResult>("session.list",
                new { limit });
            return result?.sessions ?? Array.Empty<string>();
        }

        public async Task<string> StartRecordingAsync(string label = null)
        {
            var result = await _client.CallAsync<StartResult>("record.start",
                new { label });
            ActiveSessionId = result.session_id;
            return ActiveSessionId;
        }

        public async Task StopRecordingAsync()
        {
            if (!IsRecording) return;
            await _client.CallAsync("record.stop");
            ActiveSessionId = null;
        }

        public async Task<string> VerifySessionAsync(string sessionId)
        {
            return await _client.CallAsync<string>("session.verify",
                new { session_id = sessionId });
        }
    }

    [Serializable]
    internal class ListSessionsResult
    {
        public string[] sessions;
    }

    [Serializable]
    internal class StartResult
    {
        public string session_id;
    }
}
