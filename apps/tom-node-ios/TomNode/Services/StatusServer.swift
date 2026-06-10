import Foundation
import Network

/// Minimal HTTP/1.1 TCP server exposing node metrics as JSON.
/// Polled by the infra-web-client dev dashboard every 5 seconds.
final class StatusServer: @unchecked Sendable {
    static let defaultPort: UInt16 = 9091

    private let port: UInt16
    private var listener: NWListener?
    private let snapshot: @Sendable () async -> String

    init(port: UInt16 = defaultPort, snapshot: @Sendable @escaping () async -> String) {
        self.port = port
        self.snapshot = snapshot
    }

    func start() {
        let params = NWParameters.tcp
        params.allowLocalEndpointReuse = true
        guard let nwPort = NWEndpoint.Port(rawValue: port),
              let l = try? NWListener(using: params, on: nwPort) else {
            print("[StatusServer] cannot bind port \(port)")
            return
        }
        l.stateUpdateHandler = { [port] state in
            if case .failed(let err) = state {
                print("[StatusServer] port=\(port) error: \(err)")
            }
        }
        l.newConnectionHandler = { [weak self] conn in
            self?.handle(conn)
        }
        l.start(queue: .global(qos: .utility))
        listener = l
        print("[StatusServer] listening on port \(port)")
    }

    func stop() {
        listener?.cancel()
        listener = nil
    }

    private func handle(_ conn: NWConnection) {
        conn.start(queue: .global(qos: .utility))
        // Read request headers (discarded — always respond with current snapshot)
        conn.receive(minimumIncompleteLength: 1, maximumLength: 4096) { [weak self] _, _, _, _ in
            guard let self else { conn.cancel(); return }
            Task {
                let json = await self.snapshot()
                let body = Data(json.utf8)
                let header = "HTTP/1.1 200 OK\r\nContent-Type: application/json; charset=utf-8\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: \(body.count)\r\nConnection: close\r\n\r\n"
                var response = Data(header.utf8)
                response.append(body)
                conn.send(content: response, completion: .contentProcessed { _ in conn.cancel() })
            }
        }
    }
}
