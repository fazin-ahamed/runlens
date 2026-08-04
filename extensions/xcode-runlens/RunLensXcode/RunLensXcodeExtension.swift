import Foundation
import XcodeKit

class RunLensXcodeExtension: NSObject, XCSourceEditorExtension {
    func extensionDidFinishLaunching() { }
}

class RecordSessionCommand: NSObject, XCSourceEditorCommand {
    private var isRecording = false

    func perform(with invocation: XCSourceEditorCommandInvocation,
                 completionHandler: @escaping (Error?) -> Void) {
        Task {
            do {
                let client = DaemonClient()
                try await client.connect(timeout: 1.0)

                if isRecording {
                    try await client.call(method: "record.stop")
                    isRecording = false
                    showAlert(title: "RunLens", message: "Recording stopped")
                } else {
                    let result = try await client.call(method: "record.start",
                        params: ["label": "xcode:\(invocation.buffer.displayName ?? "unknown")"])
                    isRecording = true
                    if let dict = result as? [String: Any],
                       let id = dict["session_id"] as? String {
                        showAlert(title: "RunLens", message: "Recording: \(id.prefix(8))")
                    }
                }
                client.disconnect()
                completionHandler(nil)
            } catch {
                fallbackToCLI(command: isRecording ? "stop" : "start")
                completionHandler(nil)
            }
        }
    }

    private func fallbackToCLI(command: String) {
        let task = Process()
        task.launchPath = "/usr/bin/env"
        task.arguments = ["runlens", "record", "--label", "xcode-cli"]
        try? task.run()
        task.waitUntilExit()
        showAlert(title: "RunLens (CLI)", message: "Recording \(command) via CLI")
    }

    private func showAlert(title: String, message: String) {
        let alert = NSAlert()
        alert.messageText = title
        alert.informativeText = message
        alert.runModal()
    }
}

class ListSessionsCommand: NSObject, XCSourceEditorCommand {
    func perform(with invocation: XCSourceEditorCommandInvocation,
                 completionHandler: @escaping (Error?) -> Void) {
        Task {
            do {
                let client = DaemonClient()
                try await client.connect(timeout: 1.0)
                let result = try await client.call(method: "session.list",
                    params: ["limit": 10])
                client.disconnect()

                if let dict = result as? [String: Any],
                   let sessions = dict["sessions"] as? [[String: Any]] {
                    let summary = sessions.map { s in
                        "\(String(describing: s["id"]).prefix(8)) - \(s["event_count"] ?? 0) events"
                    }.joined(separator: "\n")
                    showAlert(title: "RunLens Sessions",
                              message: sessions.isEmpty ? "No sessions" : summary)
                } else {
                    showAlert(title: "RunLens Sessions", message: "No sessions")
                }
                completionHandler(nil)
            } catch {
                let proc = Process()
                proc.launchPath = "/usr/bin/env"
                proc.arguments = ["runlens", "list", "--limit", "10"]
                let pipe = Pipe()
                proc.standardOutput = pipe
                try? proc.run()
                proc.waitUntilExit()
                let data = pipe.fileHandleForReading.readDataToEndOfFile()
                let output = String(data: data, encoding: .utf8) ?? "No output"
                showAlert(title: "RunLens Sessions (CLI)", message: output)
                completionHandler(nil)
            }
        }
    }

    private func showAlert(title: String, message: String) {
        let alert = NSAlert()
        alert.messageText = title
        alert.informativeText = message
        alert.runModal()
    }
}
