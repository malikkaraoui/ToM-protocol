import SwiftUI
import Combine
import TomProtocolKit

struct StatusView: View {
    @EnvironmentObject var nodeService: TomNodeService
    @State private var currentTime = Date()
    @Environment(\.horizontalSizeClass) var sizeClass

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

    // MARK: - tvOS : 10-foot UI avec typo plus grande

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

            // ── En-tête simple (point + nom + rôle) ────────────────────────────────
            simpleHeader
                .padding(.horizontal, 40)
                .padding(.vertical, 20)
                .background(Color.secondary.opacity(0.03))
                .border(Color.secondary.opacity(0.1), width: 1)

            if nodeService.state == .running {
                VStack(spacing: 24) {
                    // Cartes 2×2 sur tvOS
                    VStack(spacing: 24) {
                        HStack(spacing: 24) {
                            cardNetwork.frame(maxWidth: .infinity)
                            cardHealth.frame(maxWidth: .infinity)
                        }
                        HStack(spacing: 24) {
                            cardPeers.frame(maxWidth: .infinity, maxHeight: .infinity)
                            VStack(spacing: 12) {
                                controlsSection
                                if let error = nodeService.errorMessage {
                                    errorView(error)
                                }
                                Spacer()
                            }
                            .padding(16)
                            .background(Color.secondary.opacity(0.04))
                            .cornerRadius(12)
                            .frame(maxWidth: .infinity, maxHeight: .infinity)
                        }
                    }
                    Spacer()
                }
                .padding(40)
            } else {
                VStack(spacing: 20) {
                    Spacer()
                    VStack(spacing: 16) {
                        Text("Nœud inactif").font(.title2).foregroundColor(.secondary)
                        HStack {
                            Spacer()
                            controlsSection
                            Spacer()
                        }
                        if let error = nodeService.errorMessage {
                            HStack {
                                Spacer()
                                errorView(error)
                                Spacer()
                            }
                        }
                    }
                    Spacer()
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
    }

    // MARK: - iOS/iPadOS : responsive avec grille 2 colonnes sur iPad

    private var iosLayout: some View {
        VStack(spacing: 0) {
            // ── Bandeau ATTAQUE ────────────────────────────────
            if hasActiveSecurityAlert {
                securityAlertBanner
                    .padding(.horizontal, 20)
                    .padding(.vertical, 10)
                    .background(Color.red.opacity(0.15))
                    .border(Color.red.opacity(0.3), width: 1)
            }

            // ── En-tête simple ────────────────────────────────
            simpleHeader
                .padding(.horizontal, 20)
                .padding(.vertical, 14)
                .background(Color.secondary.opacity(0.03))
                .border(Color.secondary.opacity(0.1), width: 1)

            if nodeService.state == .running {
                if sizeClass == .regular {
                    // iPad : grille 2 colonnes, remplit hauteur
                    VStack(spacing: 16) {
                        Grid(horizontalSpacing: 16, verticalSpacing: 16) {
                            // Rangée 1 : Réseau + Santé (hauteurs égales)
                            GridRow {
                                cardNetwork
                                    .frame(maxHeight: .infinity)
                                cardHealth
                                    .frame(maxHeight: .infinity)
                            }
                            .frame(maxHeight: .infinity)

                            // Rangée 2 : Pairs (pleine largeur)
                            GridRow {
                                cardPeers
                                    .gridCellColumns(2)
                            }
                        }
                        Spacer()
                    }
                    .padding(20)
                } else {
                    // iPhone : empilé, remplit hauteur
                    ScrollView {
                        VStack(spacing: 16) {
                            cardNetwork
                            cardHealth
                            cardPeers
                        }
                        .padding(20)
                    }
                }
                // Footer contrôles (cohérent avec tvOS) : le bouton Arrêter est
                // désormais accessible en marche sur iPhone/iPad/Mac aussi, pas
                // seulement sur l'Apple TV.
                Divider().opacity(0.3)
                controlsSection
                    .padding(.horizontal, 20)
                    .padding(.vertical, 10)
            } else {
                VStack(spacing: 20) {
                    Spacer()
                    VStack(spacing: 16) {
                        Text("Nœud inactif").font(.title3).foregroundColor(.secondary)
                        HStack {
                            Spacer()
                            controlsSection
                            Spacer()
                        }
                        if let error = nodeService.errorMessage {
                            HStack {
                                Spacer()
                                errorView(error)
                                Spacer()
                            }
                        }
                    }
                    Spacer()
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
    }



    // MARK: - En-tête simple (point + nom + rôle)

    private var simpleHeader: some View {
        HStack(spacing: 12) {
            // Point coloré + nom device + build + état
            HStack(spacing: 8) {
                Circle().fill(statusColor).frame(width: 10, height: 10)
                VStack(alignment: .leading, spacing: 2) {
                    HStack(spacing: 6) {
                        Text(TomNodeService.shortDeviceName(nodeService.username))
                            .font(.headline).fontWeight(.semibold)
                            .lineLimit(1)
                        if nodeService.appBuild > 0 {
                            Text("\(nodeService.appBuild)")
                                .font(.caption2).fontWeight(.semibold)
                                .foregroundColor(.secondary)
                        }
                    }
                    Text(nodeService.state.rawValue.uppercased())
                        .font(.caption2).foregroundColor(.secondary).kerning(0.5)
                        .lineLimit(1)
                }
                Spacer()
            }

            // Rôle en chip
            HStack(spacing: 4) {
                Image(systemName: nodeService.localRole == "Relay"
                      ? "antenna.radiowaves.left.and.right" : "person.fill")
                    .font(.caption2)
                Text(nodeService.localRole)
                    .font(.caption).fontWeight(.semibold)
                    .lineLimit(1)
            }
            .foregroundColor(nodeService.localRole == "Relay" ? .orange : .blue)
            .padding(.horizontal, 8).padding(.vertical, 4)
            .background((nodeService.localRole == "Relay" ? Color.orange : .blue).opacity(0.12))
            .cornerRadius(6)

            // ID court en secondaire discret
            if !nodeService.nodeId.isEmpty {
                Text(shortId(nodeService.nodeId))
                    .font(.system(.caption, design: .monospaced))
                    .foregroundColor(.secondary)
                    .lineLimit(1)
                    .padding(.horizontal, 6).padding(.vertical, 2)
                    .background(Color.secondary.opacity(0.08))
                    .cornerRadius(4)
            }
        }
    }

    // MARK: - Cartes principales (Réseau, Pairs, Santé)

    private var cardNetwork: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                Image(systemName: "bolt.horizontal.fill").foregroundColor(.green)
                Text("Réseau").font(.headline)
                Spacer()
            }

            VStack(alignment: .leading, spacing: 10) {
                // Nombre de pairs connectés en GROS
                VStack(alignment: .leading, spacing: 2) {
                    Text("Pairs connectés")
                        .font(.caption).foregroundColor(.secondary)
                    Text("\(nodeService.connectedPeers.count)")
                        .font(.system(size: 32, weight: .bold))
                        .foregroundColor(.green)
                }

                Divider().opacity(0.3)

                // Rôle
                HStack {
                    Text("Rôle:").font(.caption).foregroundColor(.secondary)
                    Text(nodeService.localRole)
                        .font(.caption).fontWeight(.semibold)
                }

                // Chemin actif
                HStack {
                    Text("Chemin:").font(.caption).foregroundColor(.secondary)
                    if nodeService.pathKind == "DIRECT" {
                        HStack(spacing: 2) {
                            Image(systemName: "bolt.fill").font(.caption2).foregroundColor(.green)
                            Text("Direct \(nodeService.pathRttMs)ms")
                                .font(.caption).fontWeight(.semibold).foregroundColor(.green)
                        }
                    } else {
                        HStack(spacing: 2) {
                            Image(systemName: "cloud.fill").font(.caption2).foregroundColor(.secondary)
                            Text("Relais")
                                .font(.caption).fontWeight(.semibold)
                        }
                    }
                }
            }
        }
        .padding(14)
        .background(Color.green.opacity(0.06))
        .overlay(RoundedRectangle(cornerRadius: 12).stroke(Color.green.opacity(0.2), lineWidth: 1))
        .cornerRadius(12)
    }

    private var cardPeers: some View {
        let allPeers = nodeService.connectedPeers.map { ($0, true) } +
                       nodeService.discoveredPeers
                           .filter { !nodeService.connectedPeers.contains($0.nodeId) }
                           .map { ($0.nodeId, false) }

        return VStack(alignment: .leading, spacing: 12) {
            HStack {
                Image(systemName: "person.3.fill").foregroundColor(.purple)
                Text("Pairs").font(.headline)
                Spacer()
                Text("\(allPeers.count)")
                    .font(.caption).foregroundColor(.secondary)
                    .padding(.horizontal, 8).padding(.vertical, 4)
                    .background(Color.secondary.opacity(0.15))
                    .cornerRadius(4)
            }

            if allPeers.isEmpty {
                Text("Aucun pair découvert").font(.subheadline).foregroundColor(.secondary)
            } else {
                VStack(alignment: .leading, spacing: 6) {
                    ForEach(Array(allPeers.prefix(10)), id: \.0) { peerId, isConnected in
                        HStack(spacing: 8) {
                            Circle().fill(isConnected ? .green : Color.gray.opacity(0.4))
                                .frame(width: 6, height: 6)
                            Text(nodeService.displayName(for: peerId))
                                .font(.subheadline)
                                .fontWeight(isConnected ? .semibold : .regular)
                                .lineLimit(1)
                            Spacer()
                            if isConnected {
                                Text("✓").font(.caption2).foregroundColor(.green)
                            }
                        }
                        .padding(.vertical, 4)
                    }
                    if allPeers.count > 10 {
                        Text("+ \(allPeers.count - 10) autres")
                            .font(.caption2).foregroundColor(.secondary)
                            .padding(.vertical, 4)
                    }
                }
            }
        }
        .padding(14)
        .background(Color.purple.opacity(0.06))
        .overlay(RoundedRectangle(cornerRadius: 12).stroke(Color.purple.opacity(0.15), lineWidth: 1))
        .cornerRadius(12)
    }

    private var cardHealth: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                Image(systemName: "heart.fill").foregroundColor(.red)
                Text("Santé").font(.headline)
                Spacer()
            }

            VStack(alignment: .leading, spacing: 10) {
                // Horloge locale
                HStack {
                    Text("Heure:").font(.caption).foregroundColor(.secondary)
                    Text(formatTime(currentTime))
                        .font(.caption).fontWeight(.semibold)
                }
                .onReceive(Timer.publish(every: 1, on: .main, in: .common).autoconnect()) { _ in
                    currentTime = Date()
                }

                Divider().opacity(0.3)

                // Écart d'horloge avec label honnête
                VStack(alignment: .leading, spacing: 4) {
                    Text("Écart horloge réseau")
                        .font(.caption).foregroundColor(.secondary)
                    if let skew = nodeService.clockSkewMs, nodeService.clockSkewSamples > 0 {
                        Text(formatClockSkew(skew))
                            .font(.caption).fontWeight(.semibold)
                            .foregroundColor(clockSkewColor)
                        Text("(inclut latence + âge relayés)")
                            .font(.caption2).foregroundColor(.secondary)
                    } else {
                        Text("—").font(.caption).foregroundColor(.secondary)
                    }
                }

                Divider().opacity(0.3)

                // Compteur session
                HStack {
                    Text("Messages (session):").font(.caption).foregroundColor(.secondary)
                    Text("\(nodeService.totalMessagesCount)")
                        .font(.caption).fontWeight(.semibold).foregroundColor(.blue)
                }
            }
        }
        .padding(14)
        .background(Color.red.opacity(0.05))
        .overlay(RoundedRectangle(cornerRadius: 12).stroke(Color.red.opacity(0.15), lineWidth: 1))
        .cornerRadius(12)
    }


    // MARK: - Security Alert Banner

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
                Button(action: { nodeService.appendLog(.warning, "👆 BOUTON Démarrer pressé (utilisateur)"); nodeService.start() }) {
                    Label("Démarrer", systemImage: "play.fill")
                }
                .buttonStyle(.borderedProminent).tint(.green)
            }
            if nodeService.state == .running {
                Button(action: { nodeService.appendLog(.warning, "👆 BOUTON Arrêter pressé (utilisateur)"); nodeService.stop() }) {
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

    // MARK: - Time Formatting Helpers

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

    private func formatClockSkew(_ ms: Int64) -> String {
        let absMs = abs(ms)
        let sign = ms >= 0 ? "+" : "−"

        if absMs >= 60000 {
            let minutes = Double(absMs) / 60000.0
            return "\(sign)\(String(format: "%.1f", minutes)) min"
        } else {
            let seconds = Double(absMs) / 1000.0
            return "\(sign)\(String(format: "%.1f", seconds)) s"
        }
    }
}


#Preview {
    StatusView()
        .environmentObject(TomNodeService.shared)
}
