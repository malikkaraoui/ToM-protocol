import SwiftUI
import Combine
import TomProtocolKit

struct StatusView: View {
    @EnvironmentObject var nodeService: TomNodeService
    @State private var currentTime = Date()

    var body: some View {
        #if os(tvOS)
        tvOSLayout
        #else
        iosLayout
        #endif
    }

    // MARK: - Detect active security alert (< 60 seconds)
    private var hasActiveSecurityAlert: Bool {
        guard let lastAt = nodeService.lastSecurityEventAt else { return false }
        return Date().timeIntervalSince(lastAt) < 60
    }

    // MARK: - tvOS : activité au centre, en-tête compact, layout 70/30

    private var tvOSLayout: some View {
        VStack(spacing: 0) {
            // ── Bandeau ATTAQUE (si alerte de sécurité active) ────────────────────────────────
            if hasActiveSecurityAlert {
                securityAlertBanner
                    .padding(.horizontal, 40)
                    .padding(.vertical, 12)
                    .background(Color.red.opacity(0.15))
                    .border(Color.red.opacity(0.3), width: 1)
            }

            // ── En-tête compact (haut) ────────────────────────────────
            compactHeader
                .padding(.horizontal, 40)
                .padding(.vertical, 20)
                .background(Color.secondary.opacity(0.03))
                .border(Color.secondary.opacity(0.1), width: 1)

            // ── Contenu principal : Activité (gauche, flex) + Réseau (droite, fixed) ──
            if nodeService.state == .running {
                HStack(alignment: .top, spacing: 32) {
                    // Activité large (priorité)
                    VStack(alignment: .leading, spacing: 10) {
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
                        if nodeService.activityLog.isEmpty {
                            Text("Aucune activité").font(.subheadline).foregroundColor(.secondary)
                        } else {
                            ScrollView {
                                LazyVStack(alignment: .leading, spacing: 8) {
                                    ForEach(nodeService.activityLog.reversed()) { entry in
                                        HStack(spacing: 12) {
                                            Circle().fill(activityCategoryColor(entry)).frame(width: 6, height: 6)
                                            VStack(alignment: .leading, spacing: 2) {
                                                Text(entry.displayText).font(.body).lineLimit(2)
                                                Text(entry.relativeTime).font(.caption).foregroundColor(.secondary)
                                            }
                                            Spacer()
                                        }
                                        .padding(.vertical, 6)
                                        .padding(.horizontal, 10)
                                        .background(Color.blue.opacity(0.06))
                                        .cornerRadius(8)
                                    }
                                }
                            }
                        }
                    }
                    .frame(maxWidth: .infinity)
                    .padding(14)
                    .background(Color.blue.opacity(0.04))
                    .overlay(RoundedRectangle(cornerRadius: 12).stroke(Color.blue.opacity(0.15), lineWidth: 1))
                    .cornerRadius(12)

                    // Réseau compact (droite, fixed width)
                    VStack(alignment: .leading, spacing: 16) {
                        // Connexions
                        VStack(alignment: .leading, spacing: 8) {
                            HStack {
                                Image(systemName: "bolt.horizontal.fill").foregroundColor(.green)
                                Text("Actives").font(.headline)
                                Spacer()
                                Text("\(nodeService.connectedPeers.count)").font(.headline).foregroundColor(.green)
                            }
                            if nodeService.connectedPeers.isEmpty {
                                Text("—").font(.caption).foregroundColor(.secondary)
                            } else {
                                ForEach(nodeService.connectedPeers.prefix(8), id: \.self) { peerId in
                                    HStack(spacing: 6) {
                                        Circle().fill(.green).frame(width: 5, height: 5)
                                        Text(nodeService.displayName(for: peerId))
                                            .font(.caption).lineLimit(1)
                                    }
                                }
                                if nodeService.connectedPeers.count > 8 {
                                    Text("+ \(nodeService.connectedPeers.count - 8)")
                                        .font(.caption2).foregroundColor(.secondary)
                                }
                            }
                        }
                        .padding(10)
                        .background(Color.green.opacity(0.04))
                        .overlay(RoundedRectangle(cornerRadius: 10).stroke(Color.green.opacity(0.15), lineWidth: 1))
                        .cornerRadius(10)

                        // Nœuds connus
                        if !nodeService.discoveredPeers.isEmpty {
                            VStack(alignment: .leading, spacing: 8) {
                                HStack {
                                    Image(systemName: "network").font(.caption)
                                    Text("Découverts").font(.headline)
                                    Spacer()
                                    Text("\(nodeService.discoveredPeers.count)").font(.headline).foregroundColor(.secondary)
                                }
                                ForEach(nodeService.discoveredPeers.prefix(6) ) { peer in
                                    HStack(spacing: 6) {
                                        Circle().fill(nodeService.connectedPeers.contains(peer.nodeId) ? .green : Color.gray.opacity(0.3))
                                            .frame(width: 5, height: 5)
                                        Text(nodeService.displayName(for: peer.nodeId))
                                            .font(.caption).lineLimit(1)
                                    }
                                }
                                if nodeService.discoveredPeers.count > 6 {
                                    Text("+ \(nodeService.discoveredPeers.count - 6)")
                                        .font(.caption2).foregroundColor(.secondary)
                                }
                            }
                            .padding(10)
                            .background(Color.secondary.opacity(0.04))
                            .cornerRadius(10)
                        }

                        Spacer()

                        // Contrôles
                        VStack(spacing: 8) {
                            controlsSection
                            if let error = nodeService.errorMessage {
                                errorView(error)
                            }
                        }
                    }
                    .frame(minWidth: 220, maxWidth: 280)
                }
                .padding(32)
            } else {
                // Mode arrêté
                VStack(spacing: 20) {
                    Spacer()
                    Text("Nœud inactif").font(.title3).foregroundColor(.secondary)
                    controlsSection
                    if let error = nodeService.errorMessage {
                        errorView(error)
                    }
                    Spacer()
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
    }


    // MARK: - iOS/iPadOS : en-tête compact + activité + sections secondaires repliables

    private var iosLayout: some View {
        VStack(spacing: 0) {
            // ── Bandeau ATTAQUE (si alerte de sécurité active) ────────────────────────────────
            if hasActiveSecurityAlert {
                securityAlertBanner
                    .padding(.horizontal, 20)
                    .padding(.vertical, 10)
                    .background(Color.red.opacity(0.15))
                    .border(Color.red.opacity(0.3), width: 1)
            }

            // ── En-tête compact (haut) ────────────────────────────────
            compactHeader
                .padding(.horizontal, 20)
                .padding(.vertical, 16)
                .background(Color.secondary.opacity(0.03))
                .border(Color.secondary.opacity(0.1), width: 1)

            // ── Contenu scrollable ────────────────────────────────────
            if nodeService.state == .running {
                ScrollView {
                    VStack(spacing: 16) {
                        // Activité (priorité)
                        VStack(alignment: .leading, spacing: 10) {
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
                            if nodeService.activityLog.isEmpty {
                                Text("Aucune activité").font(.subheadline).foregroundColor(.secondary)
                                    .padding(.vertical, 20)
                            } else {
                                VStack(alignment: .leading, spacing: 8) {
                                    ForEach(nodeService.activityLog.reversed()) { entry in
                                        HStack(spacing: 10) {
                                            Circle().fill(activityCategoryColor(entry)).frame(width: 6, height: 6)
                                            VStack(alignment: .leading, spacing: 2) {
                                                Text(entry.displayText).font(.subheadline).lineLimit(2)
                                                Text(entry.relativeTime).font(.caption2).foregroundColor(.secondary)
                                            }
                                            Spacer()
                                        }
                                        .padding(.vertical, 8)
                                        .padding(.horizontal, 10)
                                        .background(Color.blue.opacity(0.06))
                                        .cornerRadius(8)
                                    }
                                }
                            }
                        }
                        .padding(14)
                        .background(Color.blue.opacity(0.04))
                        .overlay(RoundedRectangle(cornerRadius: 12).stroke(Color.blue.opacity(0.15), lineWidth: 1))
                        .cornerRadius(12)

                        // Connexions actives
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

                        // Nœuds connus
                        if !nodeService.discoveredPeers.isEmpty {
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
                                        Text(nodeService.displayName(for: peer.nodeId)).font(.body).fontWeight(.medium)
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

                        // Contrôles + Erreur
                        VStack(spacing: 12) {
                            controlsSection
                            if let error = nodeService.errorMessage {
                                errorView(error)
                            }
                        }
                    }
                    .padding(20)
                }
            } else {
                // Mode arrêté
                VStack(spacing: 20) {
                    Spacer()
                    Text("Nœud inactif").font(.title3).foregroundColor(.secondary)
                    controlsSection
                    if let error = nodeService.errorMessage {
                        errorView(error)
                    }
                    Spacer()
                }
                .frame(maxHeight: .infinity)
            }
        }
    }

    // MARK: - En-tête compact (une ligne ou deux max)

    private var compactHeader: some View {
        VStack(alignment: .leading, spacing: 8) {
            // Ligne 1 : Nom device + état (gauche) · ID court (droite)
            HStack(spacing: 10) {
                Text(nodeService.localDisplayName)
                    .font(.headline).fontWeight(.semibold)
                    .lineLimit(1).fixedSize(horizontal: true, vertical: false)
                Circle().fill(statusColor).frame(width: 8, height: 8)
                Text(nodeService.state.rawValue.uppercased())
                    .font(.caption2).foregroundColor(statusColor).kerning(0.5)
                    .lineLimit(1).fixedSize(horizontal: true, vertical: false)
                Spacer(minLength: 8)

                // Compteur d'alertes de sécurité (badge rouge si > 0 ET actif)
                if nodeService.securityAlerts > 0 {
                    HStack(spacing: 3) {
                        Image(systemName: "exclamationmark.triangle.fill")
                            .font(.caption2)
                        Text("\(nodeService.securityAlerts)")
                            .font(.caption2).fontWeight(.semibold)
                    }
                    .foregroundColor(.white)
                    .padding(.horizontal, 6).padding(.vertical, 3)
                    .background(hasActiveSecurityAlert ? Color.red : Color.orange)
                    .cornerRadius(4)
                }

                if !nodeService.nodeId.isEmpty {
                    Text(shortId(nodeService.nodeId))
                        .font(.system(.caption, design: .monospaced))
                        .foregroundColor(.secondary)
                        .lineLimit(1).fixedSize(horizontal: true, vertical: false)
                        .padding(.horizontal, 8).padding(.vertical, 4)
                        .background(Color.secondary.opacity(0.08))
                        .cornerRadius(6)
                }
            }

            // Ligne 2 : Rôle · Transport (gauche) · Compteurs lisibles (droite)
            HStack(spacing: 8) {
                // Chip Rôle
                HStack(spacing: 3) {
                    Image(systemName: nodeService.localRole == "Relay"
                          ? "antenna.radiowaves.left.and.right" : "person.fill")
                        .font(.caption2)
                    Text(nodeService.localRole).font(.caption).fontWeight(.semibold)
                        .lineLimit(1).fixedSize(horizontal: true, vertical: false)
                }
                .foregroundColor(nodeService.localRole == "Relay" ? .orange : .blue)
                .padding(.horizontal, 8).padding(.vertical, 4)
                .background((nodeService.localRole == "Relay" ? Color.orange : .blue).opacity(0.12))
                .cornerRadius(6)

                // Chip Transport
                HStack(spacing: 3) {
                    Image(systemName: nodeService.pathKind == "DIRECT" ? "bolt.fill" : "cloud.fill")
                        .font(.caption2)
                    Text(nodeService.pathKind == "DIRECT"
                         ? "Direct \(nodeService.pathRttMs)ms"
                         : "Relais")
                        .font(.caption).fontWeight(.semibold)
                        .lineLimit(1).fixedSize(horizontal: true, vertical: false)
                }
                .foregroundColor(nodeService.pathKind == "DIRECT" ? .green : .secondary)
                .padding(.horizontal, 8).padding(.vertical, 4)
                .background((nodeService.pathKind == "DIRECT" ? Color.green : Color.gray).opacity(0.12))
                .cornerRadius(6)

                Spacer(minLength: 8)

                // Horloge locale
                localClockChip

                // Écart réseau (clock skew)
                clockSkewChip

                // Compteurs — jamais tronqués/verticaux (fixedSize + lineLimit)
                HStack(spacing: 4) {
                    Image(systemName: "message.fill").font(.caption2).foregroundColor(.blue)
                    Text("\(nodeService.totalMessagesCount)")
                        .font(.subheadline).fontWeight(.bold).foregroundColor(.blue)
                        .lineLimit(1).fixedSize(horizontal: true, vertical: false)
                }
                HStack(spacing: 4) {
                    Image(systemName: "person.3.fill").font(.caption2).foregroundColor(.green)
                    Text("\(nodeService.connectedPeers.count)")
                        .font(.subheadline).fontWeight(.bold).foregroundColor(.green)
                        .lineLimit(1).fixedSize(horizontal: true, vertical: false)
                }
            }
        }
    }

    // MARK: - Indicateurs ATTAQUE & HORLOGE (Phase 2 Sprint 3)

    private var securityAlertBanner: some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack(spacing: 8) {
                Image(systemName: "exclamationmark.triangle.fill")
                    .foregroundColor(.red)
                    .font(.title3)
                Text("🚨 Anomalie sécurité détectée")
                    .font(.headline)
                    .foregroundColor(.red)
                Spacer()
                if let lastAt = nodeService.lastSecurityEventAt {
                    Text(formatSecurityEventTime(lastAt))
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }
            if !nodeService.lastSecurityEventText.isEmpty {
                Text(nodeService.lastSecurityEventText)
                    .font(.subheadline)
                    .foregroundColor(.primary)
                    .lineLimit(2)
            }
        }
        .padding(12)
        .background(Color.red.opacity(0.08))
        .cornerRadius(8)
    }

    private var localClockChip: some View {
        VStack(spacing: 0) {
            HStack(spacing: 4) {
                Image(systemName: "clock.fill")
                    .font(.caption2)
                Text(formatTime(currentTime))
                    .font(.caption).fontWeight(.semibold)
                    .lineLimit(1).fixedSize(horizontal: true, vertical: false)
            }
            .foregroundColor(.secondary)
            .padding(.horizontal, 8).padding(.vertical, 4)
            .background(Color.secondary.opacity(0.12))
            .cornerRadius(6)
        }
        .onReceive(Timer.publish(every: 1, on: .main, in: .common).autoconnect()) { _ in
            currentTime = Date()
        }
    }

    private var clockSkewChip: some View {
        VStack(spacing: 0) {
            HStack(spacing: 4) {
                Image(systemName: "antenna.radiowaves.left.and.right")
                    .font(.caption2)
                if let skew = nodeService.clockSkewMs, nodeService.clockSkewSamples > 0 {
                    let skewSeconds = Double(skew) / 1000.0
                    let sign = skew >= 0 ? "+" : ""
                    Text("\(sign)\(String(format: "%.1f", skewSeconds))s")
                        .font(.caption).fontWeight(.semibold)
                        .lineLimit(1).fixedSize(horizontal: true, vertical: false)
                } else {
                    Text("—")
                        .font(.caption).fontWeight(.semibold)
                        .foregroundColor(.secondary)
                }
            }
            .foregroundColor(clockSkewColor)
            .padding(.horizontal, 8).padding(.vertical, 4)
            .background(clockSkewColor.opacity(0.12))
            .cornerRadius(6)
        }
    }

    private var clockSkewColor: Color {
        guard let skew = nodeService.clockSkewMs else { return .gray }
        let absSkew = abs(skew)
        if absSkew < 2000 { return .green }
        if absSkew < 30000 { return .orange }
        return .red
    }

    // MARK: - Contrôles

    private var controlsSection: some View {
        HStack(spacing: 12) {
            if nodeService.state == .stopped || nodeService.state == .error {
                Button(action: { nodeService.start() }) {
                    Label("Démarrer", systemImage: "play.fill")
                }
                .buttonStyle(.borderedProminent).tint(.green)
            }
            if nodeService.state == .running {
                Button(action: { nodeService.stop() }) {
                    Label("Arrêter", systemImage: "stop.fill")
                }
                .buttonStyle(.bordered).tint(.red)
            }
            if nodeService.state == .starting || nodeService.state == .stopping {
                ProgressView().scaleEffect(0.8, anchor: .center)
            }
            Spacer()
        }
    }

    private func errorView(_ error: String) -> some View {
        Text(error)
            .foregroundColor(.red).font(.caption).lineLimit(3)
            .padding(10)
            .background(Color.red.opacity(0.1))
            .cornerRadius(8)
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
        guard id.count >= 8 else { return id }
        return String(id.prefix(8))
    }

    private func activityCategoryColor(_ entry: ActivityEntry) -> Color {
        let text = entry.displayText.lowercased()
        if text.contains("connex") || text.contains("connected") { return .green }
        if text.contains("disconnect") || text.contains("error") { return .red }
        if text.contains("message") || text.contains("sent") { return .blue }
        if text.contains("discover") || text.contains("peer") { return .purple }
        return .secondary
    }

    @ViewBuilder
    private func discoveryBadge(_ source: String) -> some View {
        let label = discoveryLabel(source)
        Text(label)
            .font(.caption2)
            .padding(.horizontal, 6).padding(.vertical, 2)
            .background(discoveryColor(label).opacity(0.12))
            .foregroundColor(discoveryColor(label))
            .cornerRadius(4)
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

    // MARK: - Time Formatting Helpers (Phase 2 Sprint 3)

    private func formatTime(_ date: Date) -> String {
        let formatter = DateFormatter()
        formatter.dateFormat = "HH:mm:ss"
        return formatter.string(from: date)
    }

    private func formatSecurityEventTime(_ date: Date) -> String {
        let elapsed = Date().timeIntervalSince(date)
        let seconds = Int(elapsed)
        let minutes = seconds / 60
        let hours = minutes / 60

        if hours > 0 {
            return "il y a \(hours)h"
        } else if minutes > 0 {
            return "il y a \(minutes)m"
        } else {
            return "il y a \(seconds)s"
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
