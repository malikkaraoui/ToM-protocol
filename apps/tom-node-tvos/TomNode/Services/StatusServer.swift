import Foundation
import TomProtocolKit
import Network

/// Minimal HTTP/1.1 TCP server exposing node metrics AND a control API as JSON.
///
/// Historiquement read-only (`GET /` → snapshot, polled par le dashboard dev).
/// Généralisé en routeur : le `router` reçoit (méthode, path, query) et rend le
/// corps JSON. Permet de PILOTER le nœud à distance sur le LAN — créer un groupe,
/// envoyer, accepter une invite, lire l'inbox — pour tester R13 sur de vrais
/// appareils, exactement comme l'API de contrôle du CLI `tom-node`.
final class StatusServer: @unchecked Sendable {
    static let defaultPort: UInt16 = 9091

    private let port: UInt16
    private var listener: NWListener?
    private let router: @Sendable (String, String, [String: String]) async -> String

    /// `router(method, path, query)` -> corps JSON de la réponse.
    init(port: UInt16 = defaultPort,
         router: @Sendable @escaping (String, String, [String: String]) async -> String) {
        self.port = port
        self.router = router
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
        conn.receive(minimumIncompleteLength: 1, maximumLength: 8192) { [weak self] data, _, _, _ in
            guard let self else { conn.cancel(); return }
            let requestLine = data
                .flatMap { String(data: $0, encoding: .utf8) }?
                .split(separator: "\r\n", maxSplits: 1).first
                .map(String.init) ?? "GET / HTTP/1.1"

            let (method, path, query) = Self.parseRequestLine(requestLine)
            Task {
                let json = await self.router(method, path, query)
                let body = Data(json.utf8)
                let header = "HTTP/1.1 200 OK\r\nContent-Type: application/json; charset=utf-8\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: \(body.count)\r\nConnection: close\r\n\r\n"
                var response = Data(header.utf8)
                response.append(body)
                conn.send(content: response, completion: .contentProcessed { _ in conn.cancel() })
            }
        }
    }

    /// "METHOD /path?k=v&k2=v2 HTTP/1.1" -> (method, path, [k: v]) avec valeurs
    /// percent-décodées.
    static func parseRequestLine(_ line: String) -> (String, String, [String: String]) {
        let parts = line.split(separator: " ")
        let method = parts.count > 0 ? String(parts[0]) : "GET"
        let target = parts.count > 1 ? String(parts[1]) : "/"
        let split = target.split(separator: "?", maxSplits: 1)
        let path = split.count > 0 ? String(split[0]) : "/"
        var query: [String: String] = [:]
        if split.count > 1 {
            for pair in split[1].split(separator: "&") {
                let kv = pair.split(separator: "=", maxSplits: 1)
                guard kv.count == 2 else { continue }
                let key = String(kv[0])
                let value = String(kv[1]).removingPercentEncoding ?? String(kv[1])
                query[key] = value
            }
        }
        return (method, path, query)
    }
}
