"""E2E integration test: start daemon, all 3 SDKs emit events, verify."""

import json
import os
import socket
import subprocess
import sys
import tempfile
import time

DAEMON_BIN = os.path.join(os.path.dirname(__file__), "..", "..", "..", "target", "debug", "runlens-daemon.exe")


def wait_for_port(port=9876, timeout=10):
    start = time.time()
    while time.time() - start < timeout:
        try:
            s = socket.create_connection(("127.0.0.1", port), timeout=1)
            s.close()
            return True
        except (ConnectionRefusedError, OSError):
            time.sleep(0.5)
    return False


def test_sdk_emit():
    env = os.environ.copy()
    env["RUNLENS_WS_PORT"] = "9876"
    env["RUNLENS_DAEMON_PORT"] = "9877"

    proc = subprocess.Popen(
        [DAEMON_BIN],
        stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        env=env,
    )
    try:
        assert wait_for_port(9876), "daemon did not start on port 9876"

        # Node SDK emit
        node_code = """
        const { DaemonClient } = require("./extensions/sdk-node/dist/index.js");
        const client = new DaemonClient();
        async function main() {
            await client.connect();
            await client.emit({
                eventId: "e1", sessionId: "s1", projectId: "p1",
                sequence: 1, source: "sdk", kind: "test.event",
                severity: "info",
                utcTimestamp: new Date().toISOString(),
                monotonicNs: Date.now() * 1_000_000,
                payload: { msg: "node-test" },
                classification: "public",
            });
            console.log("node_ok");
            await client.disconnect();
        }
        main().catch(console.error);
        """
        result = subprocess.run(["node", "-e", node_code], capture_output=True, text=True, timeout=10)
        assert "node_ok" in result.stdout, f"Node SDK failed: {result.stderr}"

        # Python SDK emit
        py_code = """
import asyncio
import sys
sys.path.insert(0, './extensions/sdk-python')
from runlens_sdk import DaemonClient
async def main():
    client = DaemonClient()
    await client.connect()
    await client.emit({
        "event_id": "e2", "session_id": "s1", "project_id": "p1",
        "sequence": 2, "source": "sdk", "kind": "test.event",
        "severity": "info",
        "utc_timestamp": "2025-01-01T00:00:00Z",
        "monotonic_ns": 0,
        "payload": {"msg": "python-test"},
        "classification": "public",
    })
    print("py_ok")
    await client.disconnect()
asyncio.run(main())
"""
        result = subprocess.run([sys.executable, "-c", py_code], capture_output=True, text=True, timeout=10)
        assert "py_ok" in result.stdout, f"Python SDK failed: {result.stderr}"

        # Go SDK emit
        go_code = """
package main
import (
    "context"
    "fmt"
    "time"
    runlens "github.com/runlens/sdk-go"
)
func main() {
    client := runlens.NewDaemonClient("ws://127.0.0.1:9876")
    ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
    defer cancel()
    if err := client.Connect(ctx); err != nil {
        fmt.Println("connect error:", err)
        return
    }
    defer client.Disconnect()
    now := time.Now().UTC()
    client.Emit(ctx, runlens.EventV2{
        EventId: "e3", SessionId: "s1", ProjectId: "p1",
        Kind: "test.event",
        Severity: "info",
        UtcTimestamp: now.Format(time.RFC3339), MonotonicNs: uint64(now.UnixNano()),
        Payload: map[string]any{"msg": "go-test"},
        Classification: "public",
    })
    fmt.Println("go_ok")
}
"""
        go_dir = tempfile.mkdtemp()
        go_mod = f"""module testemit
go 1.21
require github.com/runlens/sdk-go v0.0.0
replace github.com/runlens/sdk-go => {os.path.abspath("./extensions/sdk-go")}
"""
        with open(os.path.join(go_dir, "go.mod"), "w") as f:
            f.write(go_mod)
        with open(os.path.join(go_dir, "main.go"), "w") as f:
            f.write(go_code)
        subprocess.run(["go", "mod", "tidy"], cwd=go_dir, capture_output=True, text=True, timeout=30)
        result = subprocess.run(["go", "run", "."], cwd=go_dir, capture_output=True, text=True, timeout=30)
        assert "go_ok" in result.stdout, f"Go SDK failed: {result.stderr}"

        print("All 3 SDKs emitted events successfully")
    finally:
        proc.terminate()
        proc.wait()


if __name__ == "__main__":
    test_sdk_emit()
