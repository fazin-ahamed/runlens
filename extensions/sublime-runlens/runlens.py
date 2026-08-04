import json
import os
import subprocess
import sys
import threading
import time

import sublime
import sublime_plugin

try:
    import websocket
    HAS_WS = True
except ImportError:
    HAS_WS = False

DAEMON_HOST = "127.0.0.1"
DAEMON_PORT = 9876


def runlens_binary():
    return sublime.load_settings("RunLens.sublime-settings").get(
        "runlens_binary", "runlens"
    )


def cli_call(*args):
    bin = runlens_binary()
    try:
        r = subprocess.run([bin, *args], capture_output=True, text=True, timeout=10)
        if r.returncode == 0 and r.stdout.strip():
            return json.loads(r.stdout)
        return None
    except (FileNotFoundError, subprocess.TimeoutExpired, json.JSONDecodeError) as e:
        sublime.error_message(f"RunLens CLI error: {e}")
        return None


class DaemonClient(threading.Thread):
    def __init__(self):
        super().__init__(daemon=True)
        self.ws = None
        self.connected = False
        self._lock = threading.Lock()
        self._id = 0
        self._callbacks = {}

    def run(self):
        url = f"ws://{DAEMON_HOST}:{DAEMON_PORT}"
        try:
            self.ws = websocket.WebSocketApp(
                url,
                on_open=lambda ws: self._set_connected(True),
                on_close=lambda ws, *a: self._set_connected(False),
                on_message=self._on_message,
                on_error=lambda ws, e: self._set_connected(False),
            )
            self.ws.run_forever()
        except Exception:
            self._set_connected(False)

    def _set_connected(self, val):
        with self._lock:
            self.connected = val

    def _on_message(self, ws, msg):
        try:
            data = json.loads(msg)
        except json.JSONDecodeError:
            return
        msg_id = data.get("id")
        if msg_id is not None:
            callback = self._callbacks.pop(msg_id, None)
            if callback:
                if "error" in data:
                    callback(None, data["error"])
                else:
                    callback(data.get("result"))
        elif data.get("method"):
            method = data["method"]
            params = data.get("params", {})

    def call(self, method, params=None, callback=None):
        if params is None:
            params = {}
        self._id += 1
        msg_id = self._id
        if callback:
            self._callbacks[msg_id] = callback
        payload = json.dumps({
            "jsonrpc": "2.0",
            "id": msg_id,
            "method": method,
            "params": params,
        })
        try:
            self.ws.send(payload)
        except Exception:
            if callback:
                callback(None, {"message": "send failed"})

    def stop(self):
        if self.ws:
            self.ws.close()


class RunLensListCommand(sublime_plugin.WindowCommand):
    def run(self):
        if HAS_WS:
            client = DaemonClient()
            client.start()
            time.sleep(0.1)
            if client.connected:
                client.call("session.list", {"limit": 50}, self._show_list)
                return
            client.stop()
        result = cli_call("list", "--limit", "50", "--json")
        self._show_list(result)

    def _show_list(self, result, error=None):
        if error or not result:
            sublime.message_dialog("RunLens: no sessions or daemon not running")
            return
        sessions = result if isinstance(result, list) else result.get("sessions", [])
        if not sessions:
            sublime.message_dialog("RunLens: no sessions")
            return
        lines = []
        for s in sessions:
            sid = (s.get("id", "?") or "?")[:8]
            ev = s.get("event_count", 0)
            dur = s.get("duration_ms", 0)
            lines.append(f"{sid}  {ev} events  {dur}ms")
        self.window.show_quick_panel(lines, lambda i: None)


class RunLensRecordCommand(sublime_plugin.WindowCommand):
    def run(self):
        if HAS_WS:
            client = DaemonClient()
            client.start()
            time.sleep(0.1)
            if client.connected:
                client.call("record.start", {}, lambda r, e: self._done(r, e))
                return
            client.stop()
        result = cli_call("record", "start", "--json")
        self._done(result)

    def _done(self, result, error=None):
        if error or not result:
            sublime.message_dialog("RunLens: recording failed")
            return
        sid = (result.get("session_id", "") or "")[:8]
        sublime.message_dialog(f"RunLens: recording session {sid}")


class RunLensGraphCommand(sublime_plugin.WindowCommand):
    def run(self):
        if HAS_WS:
            client = DaemonClient()
            client.start()
            time.sleep(0.1)
            if client.connected:
                client.call("session.list", {"limit": 1}, self._on_list)
                return
            client.stop()
        sessions = cli_call("list", "--limit", "1", "--json")
        self._show_graph(sessions)

    def _on_list(self, result, error=None):
        if error or not result:
            self._show_graph(None)
            return
        self._show_graph(result)

    def _show_graph(self, sessions, error=None):
        if error or not sessions:
            sublime.message_dialog("RunLens: no sessions for critical path")
            return
        session_list = sessions if isinstance(sessions, list) else sessions.get("sessions", [])
        if not session_list:
            sublime.message_dialog("RunLens: no sessions for critical path")
            return
        sid = session_list[0].get("id", "")
        result = cli_call("graph", "critical", sid, "--json")
        path = result.get("critical_path", []) if result else []
        count = len(path)
        sublime.message_dialog(f"RunLens critical path: {count} nodes")


class RunLensStatusCommand(sublime_plugin.WindowCommand):
    def run(self):
        result = cli_call("daemon", "status")
        if result:
            sublime.message_dialog("RunLens: daemon running")
        else:
            sublime.message_dialog("RunLens: daemon not running (start with: runlens daemon)")