import Foundation

class DaemonClient {
    private let url: URL
    private var session: URLSession?
    private var task: URLSessionWebSocketTask?
    private var nextId = 1
    private var pending: [Int: (Result<Any, Error>) -> Void] = [:]
    private var queue = DispatchQueue(label: "com.runlens.daemon")

    init(url: String = "ws://localhost:9876") {
        self.url = URL(string: url)!
    }

    func connect(timeout: TimeInterval = 2.0) async throws {
        let config = URLSessionConfiguration.default
        config.timeoutIntervalForRequest = timeout
        session = URLSession(configuration: config)
        task = session!.webSocketTask(with: url)
        task!.resume()
        receiveLoop()
    }

    func disconnect() {
        task?.cancel(with: .normalClosure, reason: nil)
        task = nil
        session = nil
    }

    @discardableResult
    func call(method: String, params: Any? = nil) async throws -> Any {
        let id = nextId
        nextId += 1

        var request: [String: Any] = [
            "jsonrpc": "2.0",
            "id": id,
            "method": method
        ]
        if let params = params { request["params"] = params }

        let data = try JSONSerialization.data(withJSONObject: request)
        let message = URLSessionWebSocketTask.Message.data(data)

        return try await withCheckedThrowingContinuation { continuation in
            queue.sync { pending[id] = { result in continuation.resume(with: result) } }
            task?.send(message) { error in
                if let error = error {
                    self.queue.sync { self.pending.removeValue(forKey: id) }
                    continuation.resume(throwing: error)
                }
            }
        }
    }

    private func receiveLoop() {
        task?.receive { [weak self] result in
            guard let self = self else { return }
            switch result {
            case .success(let message):
                self.handle(message)
                self.receiveLoop()
            case .failure:
                break
            }
        }
    }

    private func handle(_ message: URLSessionWebSocketTask.Message) {
        guard case .data(let data) = message,
              let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else { return }
        if let id = json["id"] as? Int {
            queue.sync {
                if let handler = pending.removeValue(forKey: id) {
                    if let error = json["error"] as? [String: Any],
                       let message = error["message"] as? String {
                        handler(.failure(NSError(domain: "runlens", code: -1,
                            userInfo: [NSLocalizedDescriptionKey: message])))
                    } else {
                        handler(.success(json["result"] as Any))
                    }
                }
            }
        }
    }
}
