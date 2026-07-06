import Foundation

/// Identité d'un nœud (clé publique Ed25519, hex) — c'est l'adresse réseau.
public typealias NodeId = String
/// Identifiant de groupe.
public typealias GroupId = String
/// Identifiant de message (envelope id).
public typealias MessageId = String

/// Un pair découvert sur le réseau.
public struct TomPeer: Identifiable, Codable {
    public let nodeId: NodeId
    public var username: String = ""
    public var source: String = ""
    public var discoveredAt: UInt64 = 0

    public var id: NodeId { nodeId }

    public var displayName: String {
        username.isEmpty ? shortId : username
    }

    public var shortId: String {
        String(nodeId.prefix(8)) + "..." + String(nodeId.suffix(4))
    }

    public init(nodeId: NodeId, username: String = "", source: String = "", discoveredAt: UInt64 = 0) {
        self.nodeId = nodeId
        self.username = username
        self.source = source
        self.discoveredAt = discoveredAt
    }

    enum CodingKeys: String, CodingKey {
        case nodeId = "node_id"
        case username
        case source
        case discoveredAt = "discovered_at"
    }
}

/// Un message reçu (déchiffré, signature vérifiée côté Rust).
public struct TomMessage: Identifiable, Codable {
    public let id: String // envelope_id
    public let from: NodeId
    public let payload: String // base64-encoded from FFI
    public let timestamp: UInt64
    public let signatureValid: Bool
    public let wasEncrypted: Bool
    public var groupId: GroupId?

    public var payloadData: Data {
        Data(base64Encoded: payload) ?? Data()
    }

    public var text: String {
        String(data: payloadData, encoding: .utf8) ?? "<binary \(payloadData.count) bytes>"
    }

    public var date: Date {
        Date(timeIntervalSince1970: Double(timestamp) / 1000.0)
    }

    public var senderShortId: String {
        String(from.prefix(8)) + "..."
    }

    public init(
        id: String,
        from: NodeId,
        payload: String,
        timestamp: UInt64,
        signatureValid: Bool,
        wasEncrypted: Bool,
        groupId: GroupId? = nil
    ) {
        self.id = id
        self.from = from
        self.payload = payload
        self.timestamp = timestamp
        self.signatureValid = signatureValid
        self.wasEncrypted = wasEncrypted
        self.groupId = groupId
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

/// Un groupe et son fil de messages local.
public struct TomGroup: Identifiable, Codable {
    public let id: GroupId
    public var name: String
    public var members: [NodeId]
    public var messages: [TomMessage] = []

    public init(id: GroupId, name: String, members: [NodeId], messages: [TomMessage] = []) {
        self.id = id
        self.name = name
        self.members = members
        self.messages = messages
    }

    enum CodingKeys: String, CodingKey {
        case id, name, members
    }
}

/// Instantané d'état du nœud (depuis `tom_node_status`).
public struct TomNodeStatus: Codable {
    public let nodeId: String
    public let status: String
    public let peersCount: Int
    public let groupsCount: Int
    public let localRole: String?
    public let pathKind: String?
    public let pathRttMs: UInt64?

    enum CodingKeys: String, CodingKey {
        case nodeId = "node_id"
        case status
        case peersCount = "peers_count"
        case groupsCount = "groups_count"
        case localRole = "local_role"
        case pathKind = "path_kind"
        case pathRttMs = "path_rtt_ms"
    }
}

/// Stats L1-001 presence (depuis `tom_node_presence_stats`).
public struct TomPresenceStats: Codable {
    /// Attestations acceptées depuis le démarrage (monotone).
    public let acceptedTotal: UInt64
    /// Dernier attesteur accepté ("" si aucun).
    public let lastAttester: String
    /// Aller-retour du dernier accepté, horloge locale (ms).
    public let lastLatencyMs: UInt64
    /// Attestations dans la fenêtre d'agrégation 30s.
    public let windowCount: UInt64
    /// 8 premiers hex du seed d'entropie courant.
    public let seedPrefix: String

    enum CodingKeys: String, CodingKey {
        case acceptedTotal = "accepted_total"
        case lastAttester = "last_attester"
        case lastLatencyMs = "last_latency_ms"
        case windowCount = "window_count"
        case seedPrefix = "seed_prefix"
    }
}

/// Compteurs par-issue L1-001 (depuis `tom_node_presence_metrics`).
public struct TomPresenceMetrics: Codable {
    public let issued: UInt64
    public let accepted: UInt64
    public let dropUnknownChallenge: UInt64
    public let dropStale: UInt64
    public let dropWrongAttester: UInt64
    public let dropBadSignature: UInt64
    public let dropIncoherent: UInt64
    public let dropGate: UInt64
    public let dropStoreFull: UInt64
    public let challengesReceived: UInt64
    public let signed: UInt64
    public let refusedBadSignature: UInt64
    public let refusedIncoherent: UInt64
    public let refusedBudget: UInt64
    public let latencyMinMs: UInt64
    public let latencyMaxMs: UInt64
    public let latencyMeanMs: UInt64

    enum CodingKeys: String, CodingKey {
        case issued, accepted, signed
        case dropUnknownChallenge = "drop_unknown_challenge"
        case dropStale = "drop_stale"
        case dropWrongAttester = "drop_wrong_attester"
        case dropBadSignature = "drop_bad_signature"
        case dropIncoherent = "drop_incoherent"
        case dropGate = "drop_gate"
        case dropStoreFull = "drop_store_full"
        case challengesReceived = "challenges_received"
        case refusedBadSignature = "refused_bad_signature"
        case refusedIncoherent = "refused_incoherent"
        case refusedBudget = "refused_budget"
        case latencyMinMs = "latency_min_ms"
        case latencyMaxMs = "latency_max_ms"
        case latencyMeanMs = "latency_mean_ms"
    }
}

/// Cycle de vie du nœud côté app.
public enum TomNodeState: String {
    case stopped = "Stopped"
    case starting = "Starting"
    case running = "Running"
    case stopping = "Stopping"
    case error = "Error"
}
