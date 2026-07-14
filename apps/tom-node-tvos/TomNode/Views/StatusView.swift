import SwiftUI
import TomProtocolKit

struct StatusView: View {
    @EnvironmentObject var nodeService: TomNodeService

    var body: some View {
        #if os(tvOS)
        tvOSLayout
        #else
        iosLayout
        #endif
    }

    // MARK: - tvOS : deux colonnes côte à côte (tout visible sans scroll)
    // Contrainte : tout doit tenir en hauteur sans scroll (télécommande Apple TV).
    // → listes cappées à MAX_TV_ROWS, badge overflow si dépassement.

    private static let MAX_TV_ROWS = 5

    private var tvOSLayout: some View {
        HStack(alignment: .top, spacing: 40) {

            // ── Colonne gauche : identité + contrôles ─────────────────
            VStack(alignment: .leading, spacing: 16) {
                deviceHeader
                identityCard
                nodeIdView
                countersRow
                Spacer()
                controlsSection
                if let error = nodeService.errorMessage { errorView(error) }
            }
            .frame(maxWidth: .infinity)

            // ── Colonne droite : réseau + activité ────────────────────
            if nodeService.state == .running {
                VStack(alignment: .leading, spacing: 16) {
                    tvConnectionsSection
                    if !nodeService.discoveredPeers.isEmpty {
                        tvKnownNodesSection
                    }
                    if !nodeService.activityLog.isEmpty {
                        tvActivitySection
                    }
                    Spacer()
                }
                .frame(maxWidth: .infinity)
            }
        }
        .padding(48)
    }

    // ── Connexions actives (cappé MAX_TV_ROWS) ────────────────────────

    private var tvConnectionsSection: some View {
        let peers = nodeService.connectedPeers
        let shown = Array(peers.prefix(Self.MAX_TV_ROWS))
        let overflow = peers.count - shown.count
        return VStack(alignment: .leading, spacing: 10) {
            HStack {
                Image(systemName: "bolt.horizontal.fill").foregroundColor(.green)
                Text("Connexions actives").font(.headline)
                Spacer()
                Text("\(peers.count)").font(.headline).foregroundColor(.green)
            }
            if shown.isEmpty {
                Text("Aucune connexion").font(.subheadline).foregroundColor(.secondary)
            } else {
                ForEach(shown, id: \.self) { peerId in
                    HStack(spacing: 10) {
                        Circle().fill(.green).frame(width: 7, height: 7)
                        Text(nodeService.displayName(for: peerId))
                            .font(.body).fontWeight(.medium)
                        Spacer()
                    }
                    .padding(.vertical, 2)
                }
                if overflow > 0 {
                    Text("+ \(overflow) autre\(overflow > 1 ? "s" : "")")
                        .font(.caption).foregroundColor(.secondary)
                }
            }
        }
        .padding(14)
        .background(Color.green.opacity(0.06))
        .overlay(RoundedRectangle(cornerRadius: 12).stroke(Color.green.opacity(0.2), lineWidth: 1))
        .cornerRadius(12)
    }

    // ── Nœuds connus (cappé MAX_TV_ROWS) ─────────────────────────────

    private var tvKnownNodesSection: some View {
        let peers = nodeService.discoveredPeers
        let shown = Array(peers.prefix(Self.MAX_TV_ROWS))
        let overflow = peers.count - shown.count
        return VStack(alignment: .leading, spacing: 10) {
            HStack {
                Image(systemName: "network")
                Text("Nœuds connus").font(.headline)
                Spacer()
                Text("\(peers.count)").font(.headline).foregroundColor(.secondary)
            }
            ForEach(shown) { peer in
                let connected = nodeService.connectedPeers.contains(peer.nodeId)
                HStack(spacing: 10) {
                    Circle().fill(connected ? .green : Color.gray.opacity(0.4)).frame(width: 7, height: 7)
                    Text(peer.displayName).font(.body).fontWeight(.medium)
                    Text(peer.shortId)
                        .font(.system(.caption2, design: .monospaced))
                        .foregroundColor(.secondary)
                    Spacer()
                    discoveryBadge(peer.source)
                }
                .padding(.vertical, 2)
            }
            if overflow > 0 {
                Text("+ \(overflow) autre\(overflow > 1 ? "s" : "")")
                    .font(.caption).foregroundColor(.secondary)
            }
        }
        .padding(14)
        .background(Color.secondary.opacity(0.06))
        .cornerRadius(12)
    }

    // MARK: - iOS/iPadOS : colonne unique scrollable

    private var iosLayout: some View {
        ScrollView {
            VStack(spacing: 20) {
                deviceHeader
                if nodeService.state == .running {
                    identityCard
                    nodeIdView
                    countersRow
                    connectionsSection
                    if !nodeService.discoveredPeers.isEmpty { knownNodesSection }
                    activitySection
                }
                controlsSection
                if let error = nodeService.errorMessage { errorView(error) }
            }
            .padding(24)
        }
    }

    // MARK: - En-tête

    private var deviceHeader: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(nodeService.username)
                .font(.title)
                .fontWeight(.bold)
            HStack(spacing: 8) {
                Circle().fill(statusColor).frame(width: 12, height: 12)
                Text(nodeService.state.rawValue.uppercased())
                    .font(.subheadline)
                    .foregroundColor(statusColor)
                    .kerning(1)
            }
        }
    }

    // MARK: - Carte identité

    private var identityCard: some View {
        HStack(spacing: 0) {
            VStack(spacing: 6) {
                Text("MON RÔLE")
                    .font(.caption2).foregroundColor(.secondary).kerning(1)
                HStack(spacing: 5) {
                    Image(systemName: nodeService.localRole == "Relay"
                          ? "antenna.radiowaves.left.and.right" : "person.fill")
                    Text(nodeService.localRole).fontWeight(.semibold)
                }
                .font(.callout)
                .foregroundColor(nodeService.localRole == "Relay" ? .orange : .blue)
                .padding(.horizontal, 12).padding(.vertical, 6)
                .background((nodeService.localRole == "Relay" ? Color.orange : .blue).opacity(0.15))
                .cornerRadius(8)
            }
            .frame(maxWidth: .infinity)

            Divider().frame(height: 50).padding(.horizontal, 6)

            VStack(spacing: 6) {
                Text("TRANSPORT")
                    .font(.caption2).foregroundColor(.secondary).kerning(1)
                HStack(spacing: 5) {
                    Image(systemName: nodeService.pathKind == "DIRECT" ? "bolt.fill" : "cloud.fill")
                    Text(nodeService.pathKind == "DIRECT"
                         ? "Direct — \(nodeService.pathRttMs) ms"
                         : "via Relais")
                        .fontWeight(.semibold)
                }
                .font(.callout)
                .foregroundColor(nodeService.pathKind == "DIRECT" ? .green : .secondary)
                .padding(.horizontal, 12).padding(.vertical, 6)
                .background((nodeService.pathKind == "DIRECT" ? Color.green : Color.gray).opacity(0.15))
                .cornerRadius(8)
            }
            .frame(maxWidth: .infinity)
        }
        .padding(16)
        .background(Color.secondary.opacity(0.08))
        .cornerRadius(14)
    }

    // MARK: - Node ID

    private var nodeIdView: some View {
        Group {
            if !nodeService.nodeId.isEmpty {
                Text(nodeService.nodeId)
                    .font(.system(.caption2, design: .monospaced))
                    .foregroundColor(.secondary)
                    .lineLimit(1)
                    .truncationMode(.middle)
                    .padding(.horizontal, 12).padding(.vertical, 6)
                    .background(Color.secondary.opacity(0.07))
                    .cornerRadius(8)
            }
        }
    }

    // MARK: - Compteurs

    private var countersRow: some View {
        HStack(spacing: 20) {
            StatBox(title: "Messages", value: "\(nodeService.totalMessagesCount)", icon: "message.fill")
            StatBox(title: "Groupes",  value: "\(nodeService.groupsCount)",         icon: "person.3.fill")
        }
    }

    // MARK: - Connexions actives

    private var connectionsSection: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Image(systemName: "bolt.horizontal.fill").foregroundColor(.green)
                Text("Connexions actives").font(.headline)
                Spacer()
                Text("\(nodeService.connectedPeers.count)").font(.headline).foregroundColor(.green)
            }
            if nodeService.connectedPeers.isEmpty {
                Text("Aucune connexion").font(.subheadline).foregroundColor(.secondary)
            } else {
                ForEach(nodeService.connectedPeers, id: \.self) { peerId in
                    HStack(spacing: 10) {
                        Circle().fill(.green).frame(width: 7, height: 7)
                        Text(nodeService.displayName(for: peerId))
                            .font(.body).fontWeight(.medium)
                        Spacer()
                    }
                    .padding(.vertical, 2)
                }
            }
        }
        .padding(14)
        .background(Color.green.opacity(0.06))
        .overlay(RoundedRectangle(cornerRadius: 12).stroke(Color.green.opacity(0.2), lineWidth: 1))
        .cornerRadius(12)
    }

    // MARK: - Nœuds connus

    private var knownNodesSection: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Image(systemName: "network")
                Text("Nœuds connus").font(.headline)
                Spacer()
                Text("\(nodeService.discoveredPeers.count)").font(.headline).foregroundColor(.secondary)
            }
            ForEach(nodeService.discoveredPeers) { peer in
                let connected = nodeService.connectedPeers.contains(peer.nodeId)
                HStack(spacing: 10) {
                    Circle().fill(connected ? .green : Color.gray.opacity(0.4)).frame(width: 7, height: 7)
                    Text(peer.displayName).font(.body).fontWeight(.medium)
                    Text(peer.shortId)
                        .font(.system(.caption2, design: .monospaced))
                        .foregroundColor(.secondary)
                    Spacer()
                    discoveryBadge(peer.source)
                }
                .padding(.vertical, 2)
            }
        }
        .padding(14)
        .background(Color.secondary.opacity(0.06))
        .cornerRadius(12)
    }

    // ── Activité (5 derniers événements) ──────────────────────────────

    private var activitySection: some View {
        let recentEvents = Array(nodeService.activityLog.suffix(5).reversed())
        return VStack(alignment: .leading, spacing: 10) {
            HStack {
                Image(systemName: "bell.badge.fill").foregroundColor(.blue)
                Text("Activité").font(.headline)
                Spacer()
                Text("\(nodeService.activityLog.count)")
                    .font(.caption).foregroundColor(.secondary)
                    .padding(.horizontal, 8).padding(.vertical, 4)
                    .background(Color.secondary.opacity(0.2))
                    .cornerRadius(6)
            }
            if recentEvents.isEmpty {
                Text("Aucune activité").font(.subheadline).foregroundColor(.secondary)
            } else {
                ForEach(recentEvents) { entry in
                    VStack(alignment: .leading, spacing: 4) {
                        Text(entry.displayText)
                            .font(.subheadline)
                            .lineLimit(2)
                        Text(entry.relativeTime)
                            .font(.caption2)
                            .foregroundColor(.secondary)
                    }
                    .padding(.vertical, 8)
                    .padding(.horizontal, 10)
                    .background(Color.blue.opacity(0.08))
                    .cornerRadius(8)
                }
            }
        }
        .padding(14)
        .background(Color.blue.opacity(0.06))
        .cornerRadius(12)
    }

    // ── Activité tvOS ─────────────────────────────────────────────────

    private var tvActivitySection: some View {
        let recentEvents = Array(nodeService.activityLog.suffix(Self.MAX_TV_ROWS).reversed())
        let overflow = nodeService.activityLog.count - recentEvents.count
        return VStack(alignment: .leading, spacing: 10) {
            HStack {
                Image(systemName: "bell.badge.fill").foregroundColor(.blue)
                Text("Activité").font(.headline)
                Spacer()
                Text("\(nodeService.activityLog.count)").font(.headline).foregroundColor(.blue)
            }
            if recentEvents.isEmpty {
                Text("Aucune activité").font(.subheadline).foregroundColor(.secondary)
            } else {
                ForEach(recentEvents) { entry in
                    HStack(spacing: 12) {
                        VStack(alignment: .leading, spacing: 2) {
                            Text(entry.displayText).font(.body).lineLimit(2)
                            Text(entry.relativeTime).font(.caption).foregroundColor(.secondary)
                        }
                        Spacer()
                    }
                    .padding(.vertical, 6)
                }
                if overflow > 0 {
                    Text("+ \(overflow) autre\(overflow > 1 ? "s" : "")")
                        .font(.caption).foregroundColor(.secondary)
                }
            }
        }
        .padding(14)
        .background(Color.blue.opacity(0.06))
        .overlay(RoundedRectangle(cornerRadius: 12).stroke(Color.blue.opacity(0.2), lineWidth: 1))
        .cornerRadius(12)
    }

    // MARK: - Contrôles

    private var controlsSection: some View {
        HStack(spacing: 16) {
            if nodeService.state == .stopped || nodeService.state == .error {
                Button(action: { nodeService.start() }) {
                    Label("Démarrer", systemImage: "play.fill").frame(minWidth: 120)
                }
                .buttonStyle(.borderedProminent).tint(.green)
            }
            if nodeService.state == .running {
                Button(action: { nodeService.stop() }) {
                    Label("Arrêter", systemImage: "stop.fill").frame(minWidth: 120)
                }
                .buttonStyle(.bordered).tint(.red)
            }
            if nodeService.state == .starting || nodeService.state == .stopping {
                ProgressView()
            }
        }
    }

    private func errorView(_ error: String) -> some View {
        Text(error)
            .foregroundColor(.red).font(.callout).padding()
            .background(Color.red.opacity(0.1)).cornerRadius(8)
    }

    // MARK: - Helpers

    private var statusColor: Color {
        switch nodeService.state {
        case .running: return .green
        case .starting, .stopping: return .orange
        case .error: return .red
        case .stopped: return .gray
        }
    }

    private func shortId(_ id: String) -> String {
        guard id.count > 12 else { return id }
        return String(id.prefix(8)) + "…" + String(id.suffix(4))
    }

    @ViewBuilder
    private func discoveryBadge(_ source: String) -> some View {
        let label = discoveryLabel(source)
        Text(label)
            .font(.caption2)
            .padding(.horizontal, 7).padding(.vertical, 2)
            .background(discoveryColor(label).opacity(0.12))
            .foregroundColor(discoveryColor(label))
            .cornerRadius(5)
    }

    private func discoveryLabel(_ source: String) -> String {
        switch source.lowercased() {
        case "direct":   return "mDNS"
        case "gossip":   return "Gossip"
        case "announce": return "Announce"
        case "dht":      return "DHT"
        default:         return source.isEmpty ? "?" : source
        }
    }

    private func discoveryColor(_ label: String) -> Color {
        switch label {
        case "mDNS":     return .cyan
        case "Gossip":   return .purple
        case "Announce": return .orange
        case "DHT":      return .blue
        default:         return .secondary
        }
    }
}

// MARK: - StatBox

struct StatBox: View {
    let title: String
    let value: String
    let icon: String

    var body: some View {
        VStack(spacing: 6) {
            Image(systemName: icon).font(.title3).foregroundColor(.accentColor)
            Text(value).font(.title2).fontWeight(.bold)
            Text(title).font(.caption).foregroundColor(.secondary)
        }
        .frame(minWidth: 90)
        .padding(12)
        .background(Color.secondary.opacity(0.1))
        .cornerRadius(12)
    }
}

#Preview {
    StatusView()
        .environmentObject(TomNodeService.shared)
}
