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
    public var appBuild: Int = 0

    public var id: NodeId { nodeId }

    public var displayName: String {
        username.isEmpty ? shortId : username
    }

    public var shortId: String {
        String(nodeId.prefix(8)) + "..." + String(nodeId.suffix(4))
    }

    public init(nodeId: NodeId, username: String = "", source: String = "", discoveredAt: UInt64 = 0, appBuild: Int = 0) {
        self.nodeId = nodeId
        self.username = username
        self.source = source
        self.discoveredAt = discoveredAt
        self.appBuild = appBuild
    }

    enum CodingKeys: String, CodingKey {
        case nodeId = "node_id"
        case username
        case source
        case discoveredAt = "discovered_at"
        case appBuild = "app_build"
    }
}

/// Un message reçu (déchiffré, signature vérifiée côté Rust).
/// Cycle de vie d'un message SORTANT, reflété depuis le tracker Rust
/// (Pending→Sent→Relayed→Delivered/Failed) + purge du store à la livraison.
public enum TomDeliveryStatus: String, Codable {
    case sending          // en cours (pas encore d'ACK)
    case relayed          // relais a accusé réception
    case delivered        // ACK du destinataire (livré)
    case purged           // backup purgé du store expéditeur après livraison
    case failed           // abandonné après retries

    public var label: String {
        switch self {
        case .sending: return "En cours"
        case .relayed: return "Relayé"
        case .delivered: return "Délivré"
        case .purged: return "Délivré · purgé"
        case .failed: return "Échec"
        }
    }

    public var icon: String {
        switch self {
        case .sending: return "clock.arrow.circlepath"
        case .relayed: return "antenna.radiowaves.left.and.right"
        case .delivered: return "checkmark.circle.fill"
        case .purged: return "trash.circle.fill"
        case .failed: return "exclamationmark.triangle.fill"
        }
    }
}

public struct TomMessage: Identifiable, Codable {
    public let id: String // envelope_id
    public let from: NodeId
    public let payload: String // base64-encoded from FFI
    public let timestamp: UInt64
    public let signatureValid: Bool
    public let wasEncrypted: Bool
    public var groupId: GroupId?
    // ── Champs message SORTANT (nil pour un message reçu) ──
    /// Destinataire (nil ⇒ message entrant, `from` est l'expéditeur distant).
    public var to: NodeId? = nil
    /// true ⇒ message automatique (sonde/ping), false ⇒ tapé par l'utilisateur.
    public var isAuto: Bool = false
    /// Statut de livraison courant (messages sortants uniquement).
    public var deliveryStatus: TomDeliveryStatus? = nil

    /// Message sortant ⟺ on a un destinataire.
    public var isOutgoing: Bool { to != nil }

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
        groupId: GroupId? = nil,
        to: NodeId? = nil,
        isAuto: Bool = false,
        deliveryStatus: TomDeliveryStatus? = nil
    ) {
        self.id = id
        self.from = from
        self.payload = payload
        self.timestamp = timestamp
        self.signatureValid = signatureValid
        self.wasEncrypted = wasEncrypted
        self.groupId = groupId
        self.to = to
        self.isAuto = isAuto
        self.deliveryStatus = deliveryStatus
    }

    enum CodingKeys: String, CodingKey {
        case id = "envelope_id"
        case from
        case payload
        case timestamp
        case signatureValid = "signature_valid"
        case wasEncrypted = "was_encrypted"
        case groupId = "group_id"
        case to
        case isAuto
        case deliveryStatus
    }

    /// Décodage EXPLICITE — ne pas supprimer au profit du synthétisé.
    /// Le JSON du FFI (`DeliveredMessageFFI`) ne contient pas les champs
    /// sortants (`to`, `isAuto`, `deliveryStatus`) : le `init(from:)`
    /// synthétisé exige toute clé non-Optional (`isAuto`) → keyNotFound →
    /// batch entier jeté (régression builds 86-95, messages_recus=0 flotte).
    /// Ici : champs FFI requis, champs sortants tolérés absents.
    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        self.id = try c.decode(String.self, forKey: .id)
        self.from = try c.decode(NodeId.self, forKey: .from)
        self.payload = try c.decode(String.self, forKey: .payload)
        self.timestamp = try c.decode(UInt64.self, forKey: .timestamp)
        self.signatureValid = try c.decode(Bool.self, forKey: .signatureValid)
        self.wasEncrypted = try c.decode(Bool.self, forKey: .wasEncrypted)
        self.groupId = try c.decodeIfPresent(GroupId.self, forKey: .groupId)
        self.to = try c.decodeIfPresent(NodeId.self, forKey: .to)
        self.isAuto = try c.decodeIfPresent(Bool.self, forKey: .isAuto) ?? false
        self.deliveryStatus = try c.decodeIfPresent(TomDeliveryStatus.self, forKey: .deliveryStatus)
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

/// Message de groupe reçu (depuis `tom_node_receive_group_messages`).
/// Inclut les messages rattrapés hors-ligne via le gap-fill R13.
public struct TomGroupMessage: Codable, Identifiable {
    public var id: String { "\(groupId)-\(seq)-\(from)" }
    public let groupId: GroupId
    public let from: NodeId
    public let seq: UInt64
    public let text: String
    public let encrypted: Bool

    enum CodingKeys: String, CodingKey {
        case groupId = "group_id"
        case from
        case seq
        case text
        case encrypted
    }
}

/// Per-peer path information
public struct PathInfo: Codable {
    public let kind: String
    public let rtt_ms: UInt64
    public let addr: String
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
    public let pathAddr: String?
    public let clockSkewMs: Int64?
    public let clockSkewSamples: UInt64?
    public let appBuild: UInt32
    /// Per-peer path information: node_id → PathInfo
    public let pathsByPeer: [String: PathInfo]?

    enum CodingKeys: String, CodingKey {
        case nodeId = "node_id"
        case status
        case peersCount = "peers_count"
        case groupsCount = "groups_count"
        case localRole = "local_role"
        case pathKind = "path_kind"
        case pathRttMs = "path_rtt_ms"
        case pathAddr = "path_addr"
        case clockSkewMs = "clock_skew_ms"
        case clockSkewSamples = "clock_skew_samples"
        case appBuild = "app_build"
        case pathsByPeer = "paths_by_peer"
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

/// Protocol event from the Rust runtime (drained from the queue, JSON decoded).
/// Each event carries a type (event name) + optional typed fields depending on the variant.
/// Designed for UI display (activity log, notifications, debugging).
public struct TomProtocolEvent: Codable, Hashable, Identifiable {
    /// Unique identifier for sorting/de-duplication in UI.
    public var id: UUID { UUID() }

    /// Event type name (e.g., "PeerDiscovered", "RolePromoted", "SenderThrottled")
    public let type: String

    /// Timestamp when event was added to the queue (milliseconds since epoch).
    public let ts_ms: UInt64

    // ── Peer/Node IDs (short hex for display) ──
    public let node_id: String?
    public let username: String?
    public let source: String? // DiscoverySource format

    // ── Scores and rates ──
    public let score: Double?
    public let current_rate: Double?

    // ── Strings ──
    public let reason: String?
    public let message_id: String?
    public let group_id: String?
    public let description: String?

    // ── Path/Role info ──
    public let path_kind: String? // "RELAY" | "DIRECT"
    public let rtt_ms: UInt64?
    public let path_addr: String?
    public let new_role: String? // "Client" | "Relay" | "Backup"

    // ── Presence ──
    public let latency_ms: UInt64?

    // ── Delivery/retry ──
    public let attempt: UInt8?
    public let last_status: String? // "Pending" | "Sent" | "Delivered" | ...

    /// Relative timestamp for UI display (e.g., "il y a 12 s").
    public var relativeTime: String {
        let now = UInt64(Date().timeIntervalSince1970 * 1000)
        let elapsedMs = now > ts_ms ? (now - ts_ms) : 0
        let seconds = elapsedMs / 1000
        let minutes = seconds / 60
        let hours = minutes / 60

        if hours > 0 {
            return "il y a \(hours)h"
        } else if minutes > 0 {
            return "il y a \(minutes)m"
        } else if seconds > 0 {
            return "il y a \(seconds)s"
        } else {
            return "à l'instant"
        }
    }

    /// Human-readable summary for activity log display.
    /// Texte affichable, avec résolution optionnelle des noms d'appareils :
    /// si `resolver` est fourni, chaque node_id est remplacé par le nom joli
    /// (« Freebox · 11d5bb11 »). Sinon, ID court seul.
    public func displayLine(resolvingNames resolver: ((String) -> String)? = nil) -> String {
        let shortId = { (full: String?) -> String in
            guard let f = full, !f.isEmpty else { return "?" }
            if let r = resolver { return r(f) }
            return f.count > 8 ? String(f.prefix(8)) : f
        }

        // Nom non vide, sinon nil (un username "" ne doit pas afficher du vide).
        let name = (username?.isEmpty == false) ? username : nil

        switch type {
        case "PeerDiscovered":
            return "👤 Pair découvert : \(name ?? shortId(node_id))"
        case "PeerOnline":
            return "🟢 Pair en ligne : \(shortId(node_id))"
        case "PeerOffline":
            return "🔴 Pair hors ligne : \(shortId(node_id))"
        case "PeerStale":
            return "⚠️ Pair vieilli : \(shortId(node_id))"
        case "RolePromoted":
            return "🎭 Rôle promu → Relais (score \(Int(score ?? 0)))"
        case "RoleDemoted":
            return "⬇️ Rôle rétrogradé (score \(Int(score ?? 0)))"
        case "LocalRoleChanged":
            return "🔄 Mon rôle : \(new_role ?? "?")"
        case "BackupStored":
            return "📥 Backup stocké pour \(shortId(node_id))"
        case "BackupDelivered":
            return "✅ Backup livré à \(shortId(node_id))"
        case "BackupExpired":
            return "⏰ Backup expiré pour \(shortId(node_id))"
        case "SenderThrottled":
            return "🚨 Pair ralenti \(shortId(node_id)) (antispam)"
        case "PathChanged":
            var detail = path_kind ?? "?"
            if let addr = path_addr, !addr.isEmpty {
                detail += " \(addr)"
            }
            return "🔀 Chemin \(shortId(node_id)) → \(detail)"
        case "MessageRejected":
            return "❌ Message rejeté : \(reason ?? "?")"
        case "Forwarded":
            return "➡️ Relayé vers \(shortId(node_id))"
        case "GroupCreated":
            return "👥 Groupe créé : \(username ?? shortId(group_id))"
        case "GroupJoined":
            return "✅ Rejoint groupe : \(username ?? shortId(group_id))"
        case "GroupMemberJoined":
            return "➕ \(username ?? shortId(node_id)) a rejoint"
        case "GroupMemberLeft":
            return "➖ \(username ?? shortId(node_id)) a quitté"
        case "GossipNeighborUp":
            return "🔗 Voisin gossip actif : \(shortId(node_id))"
        case "GossipNeighborDown":
            return "🔌 Voisin gossip inactif : \(shortId(node_id))"
        case "EmbeddedRelayStarted":
            return "🚀 Relai embarqué démarré"
        case "EmbeddedRelayFailed":
            return "💥 Relai embarqué : \(description ?? "erreur")"
        case "MessageStatus":
            let etat: String
            switch last_status {
            case "Pending", "Sent": etat = "📤 en cours"
            case "Relayed": etat = "📡 relayé"
            case "Delivered": etat = "✅ délivré"
            case "Failed": etat = "❌ échec"
            default: etat = last_status ?? "?"
            }
            return "\(etat) → \(shortId(node_id))"
        case "DeliveryRetry":
            return "🔄 Nouvelle tentative \(attempt ?? 0) vers \(shortId(node_id))"
        case "DeliveryTimeout":
            return "⏱️ Livraison échouée : \(shortId(node_id))"
        case "PresenceAttestationReceived":
            return "🔐 Présence attestée (latence \(latency_ms ?? 0)ms)"
        case "DiscoveryTiming":
            // Le détail contient déjà « ⏱️ DISCO t+Xms : … » (instrumentation #26).
            return description ?? "⏱️ DISCO"
        default:
            return "📌 \(type)"
        }
    }

    /// Texte par défaut (ID courts, sans résolution de noms).
    public var displayText: String { displayLine() }

    enum CodingKeys: String, CodingKey {
        case type
        case ts_ms
        case node_id
        case username
        case source
        case score
        case current_rate
        case reason
        case message_id
        case group_id
        case description
        case path_kind
        case rtt_ms
        case path_addr
        case new_role
        case latency_ms
        case attempt
        case last_status
    }
}
