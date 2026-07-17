import Foundation

/// Erreurs du SDK Apple ToM.
public enum TomError: LocalizedError {
    case notStarted
    case alreadyRunning
    case ffiReturnedNull(function: String)
    case jsonParseFailed(String)
    case sendFailed(String)
    case groupCreateFailed(String)
    /// `tom_node_start` a rendu -2 : budget de démarrage expiré (réseau
    /// indisponible/dégradé). Le handle reste réutilisable — réessayer avec
    /// backoff au lieu d'afficher une erreur terminale.
    case startTimeout(String)
    case unknown(String)

    public var errorDescription: String? {
        switch self {
        case .notStarted:
            return "Node is not started"
        case .alreadyRunning:
            return "Node is already running"
        case .ffiReturnedNull(let fn):
            return "FFI function \(fn) returned NULL"
        case .jsonParseFailed(let detail):
            return "JSON parse failed: \(detail)"
        case .sendFailed(let detail):
            return "Send failed: \(detail)"
        case .groupCreateFailed(let detail):
            return "Group create failed: \(detail)"
        case .startTimeout(let detail):
            return "Start timed out: \(detail)"
        case .unknown(let detail):
            return "Unknown error: \(detail)"
        }
    }
}
