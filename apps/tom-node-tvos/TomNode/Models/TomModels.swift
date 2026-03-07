import Foundation

typealias NodeId = String
typealias GroupId = String
typealias MessageId = String

struct TomPeer: Identifiable, Codable {
    let id: NodeId
    var isOnline: Bool = false
    var lastSeen: Date?

    var shortId: String {
        String(id.prefix(8)) + "..." + String(id.suffix(4))
    }
}

struct TomMessage: Identifiable, Codable {
    let id: String // envelope_id
    let from: NodeId
    let payload: String // base64-encoded from FFI
    let timestamp: UInt64
    let signatureValid: Bool
    let wasEncrypted: Bool
    var groupId: GroupId?

    var payloadData: Data {
        Data(base64Encoded: payload) ?? Data()
    }

    var text: String {
        String(data: payloadData, encoding: .utf8) ?? "<binary \(payloadData.count) bytes>"
    }

    var date: Date {
        Date(timeIntervalSince1970: Double(timestamp) / 1000.0)
    }

    var senderShortId: String {
        String(from.prefix(8)) + "..."
    }

    enum CodingKeys: String, CodingKey {
        case id = "envelope_id"
        case from
        case payload
        case timestamp
        case signatureValid = "signature_valid"
        case wasEncrypted = "was_encrypted"
        case groupId = "group_id"
    }
}

struct TomGroup: Identifiable, Codable {
    let id: GroupId
    var name: String
    var members: [NodeId]
    var messages: [TomMessage] = []

    enum CodingKeys: String, CodingKey {
        case id, name, members
    }
}

struct TomNodeStatus: Codable {
    let nodeId: String
    let status: String
    let peersCount: Int
    let groupsCount: Int

    enum CodingKeys: String, CodingKey {
        case nodeId = "node_id"
        case status
        case peersCount = "peers_count"
        case groupsCount = "groups_count"
    }
}

enum TomNodeState: String {
    case stopped = "Stopped"
    case starting = "Starting"
    case running = "Running"
    case stopping = "Stopping"
    case error = "Error"
}
