import SwiftUI
import TomProtocolKit
#if os(iOS)
import UIKit
#endif

struct SettingsRow: View {
    let label: String
    let value: String
    var monospaced: Bool = false
    var valueColor: Color = .primary

    var body: some View {
        Button(action: {}) {
            HStack {
                Text(label)
                    .foregroundColor(.secondary)
                Spacer()
                Text(value)
                    .foregroundColor(valueColor)
                    .font(monospaced ? .system(.body, design: .monospaced) : .body)
                    .lineLimit(1)
                    .truncationMode(.middle)
            }
            .padding(.vertical, 4)
        }
    }
}

struct SettingsView: View {
    @EnvironmentObject var nodeService: TomNodeService
    @State private var confirmNetworkReset = false
    @State private var confirmFactoryReset = false
    /// Feedback des resets : l'effacement va trop vite (<1 s) pour être vu →
    /// sans spinner ni confirmation, les boutons semblaient inertes.
    @State private var resetInProgress = false
    @State private var resetDone: String?

    /// Reset autorisé uniquement nœud arrêté (invariant sûreté #4 : les .db
    /// SQLite sont ouvertes tant que le runtime tourne).
    private var canReset: Bool {
        nodeService.state == .stopped || nodeService.state == .error
    }

    /// Exécute le reset avec retour visible : spinner pendant l'effacement,
    /// haptique (iPhone/iPad seulement — tvOS/macOS n'ont pas de moteur) puis
    /// toast ✓ qui s'efface seul.
    private func performReset(level: TomNodeService.ResetLevel) async {
        resetInProgress = true
        resetDone = nil
        _ = await nodeService.resetNode(level: level, source: .button)
        resetInProgress = false
        let msg = level == .factory
            ? "Réinitialisation usine faite — node_id neuf au prochain démarrage"
            : "Réseau oublié — le nœud est amnésique"
        resetDone = msg
        #if os(iOS)
        UINotificationFeedbackGenerator().notificationOccurred(.success)
        #endif
        try? await Task.sleep(nanoseconds: 4_000_000_000)
        if resetDone == msg { resetDone = nil }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            Text("Réglages")
                .font(.title2)
                .fontWeight(.bold)
                .padding(.horizontal, 48)
                .padding(.top, 20)
                .padding(.bottom, 10)

            List {
                Section("Node Identity") {
                    if !nodeService.nodeId.isEmpty {
                        SettingsRow(label: "Node ID", value: nodeService.nodeId, monospaced: true)
                    } else {
                        SettingsRow(label: "Node ID", value: "Not started", valueColor: .secondary)
                    }
                }

                Section("Network") {
                    SettingsRow(label: "Relay Mode", value: nodeService.relayStatusLabel, monospaced: true)
                    SettingsRow(label: "Bootstrap", value: nodeService.bootstrapStatusLabel, monospaced: true)

                    TextField("Relay URL (optional live seed)", text: Binding(
                        get: { nodeService.relayUrl },
                        set: { nodeService.relayUrl = $0.trimmingCharacters(in: .whitespacesAndNewlines) }
                    ))
                    .disabled(nodeService.state == .running)

                    TextField("Bootstrap Peer ID (optional live seed)", text: Binding(
                        get: { nodeService.bootstrapPeerId },
                        set: { nodeService.bootstrapPeerId = $0.trimmingCharacters(in: .whitespacesAndNewlines) }
                    ))
                    .font(.system(.body, design: .monospaced))
                    .disabled(nodeService.state == .running)

                    Toggle("N0 Discovery", isOn: Binding(
                        get: { nodeService.n0Discovery },
                        set: { nodeService.n0Discovery = $0 }
                    ))
                    .disabled(nodeService.state == .running)

                    Toggle("DHT", isOn: Binding(
                        get: { nodeService.enableDht },
                        set: { nodeService.enableDht = $0 }
                    ))
                    .disabled(nodeService.state == .running)

                    Toggle("Encryption", isOn: Binding(
                        get: { nodeService.encryption },
                        set: { nodeService.encryption = $0 }
                    ))
                    .disabled(nodeService.state == .running)
                }

                Section("Observability") {
                    Toggle("UDP Log Export", isOn: Binding(
                        get: { nodeService.udpLogExportEnabled },
                        set: { nodeService.udpLogExportEnabled = $0 }
                    ))
                    .disabled(nodeService.state == .running)

                    TextField("IPv4 host (optional)", text: Binding(
                        get: { nodeService.udpLogHost },
                        set: { nodeService.udpLogHost = $0.trimmingCharacters(in: .whitespacesAndNewlines) }
                    ))
                    .disabled(nodeService.state == .running || !nodeService.udpLogExportEnabled)

                    TextField("Port", text: Binding(
                        get: { nodeService.udpLogPort },
                        set: { nodeService.udpLogPort = String($0.filter(\.isNumber).prefix(5)) }
                    ))
                    .disabled(nodeService.state == .running || !nodeService.udpLogExportEnabled)

                    SettingsRow(
                        label: "UDP Status",
                        value: nodeService.udpLogExportEnabled ? "Enabled on next start" : "Disabled",
                        valueColor: nodeService.udpLogExportEnabled ? .orange : .secondary
                    )
                }

                Section("Peers") {
                    // id: \.offset (index) et non \.self : un [String] avec des
                    // doublons transitoires faisait délirer SwiftUI (lignes
                    // fantômes + Discovered déplacé au milieu). L'index est
                    // toujours unique → rendu stable.
                    if !nodeService.connectedPeers.isEmpty {
                        ForEach(Array(nodeService.connectedPeers.enumerated()), id: \.offset) { _, peerId in
                            if let peer = nodeService.discoveredPeers.first(where: { $0.nodeId == peerId }) {
                                let displayValue = nodeService.displayName(for: peerId) + (peer.appBuild > 0 ? " \(peer.appBuild)" : "")
                                SettingsRow(
                                    label: "Connected",
                                    value: displayValue,
                                    valueColor: .green
                                )
                            } else {
                                SettingsRow(
                                    label: "Connected",
                                    value: nodeService.displayName(for: peerId),
                                    valueColor: .green
                                )
                            }
                        }
                    }
                    // Découverts NON déjà connectés (évite de lister un pair deux fois).
                    let discoveredOnly = nodeService.discoveredPeers.filter {
                        !nodeService.connectedPeers.contains($0.nodeId)
                    }
                    if !discoveredOnly.isEmpty {
                        ForEach(Array(discoveredOnly.enumerated()), id: \.offset) { _, peer in
                            let displayValue = nodeService.displayName(for: peer.nodeId) + (peer.appBuild > 0 ? " \(peer.appBuild)" : "")
                            SettingsRow(
                                label: "Discovered",
                                value: displayValue,
                                valueColor: .blue
                            )
                        }
                    }
                    if nodeService.connectedPeers.isEmpty && nodeService.discoveredPeers.isEmpty {
                        SettingsRow(label: "Peers", value: "None yet", valueColor: .secondary)
                    }
                }

                Section("Stress Testing") {
                    Toggle("Auto-Echo (respond to all messages)", isOn: Binding(
                        get: { nodeService.autoEchoEnabled },
                        set: { nodeService.autoEchoEnabled = $0 }
                    ))

                    SettingsRow(label: "Echo Count", value: "\(nodeService.echoCount)")
                }

                // Reset de cache (« sortir du labo », doc reset-cache-app §5).
                // Boutons ACTIFS uniquement nœud arrêté/en erreur (sinon .db
                // ouvertes → corruption). Chacun derrière une confirmation ;
                // l'usine change le node_id (destructif).
                Section("Reset (sortie labo)") {
                    Button(role: .destructive) {
                        confirmNetworkReset = true
                    } label: {
                        Label("Oublier le réseau", systemImage: "arrow.counterclockwise")
                    }
                    .disabled(!canReset || resetInProgress)

                    Button(role: .destructive) {
                        confirmFactoryReset = true
                    } label: {
                        Label("Réinitialisation usine (nouveau node_id)", systemImage: "trash")
                    }
                    .disabled(!canReset || resetInProgress)

                    if resetInProgress {
                        HStack(spacing: 8) {
                            ProgressView()
                            Text("Effacement en cours…")
                                .font(.caption2)
                                .foregroundColor(.secondary)
                        }
                    } else if let done = resetDone {
                        Label(done, systemImage: "checkmark.circle.fill")
                            .font(.caption2)
                            .foregroundColor(.green)
                    } else if !canReset {
                        Text("Arrêter le nœud pour activer le reset.")
                            .font(.caption2)
                            .foregroundColor(.secondary)
                    }
                }

                Section("Profile") {
                    SettingsRow(label: "Username", value: nodeService.username)
                }

                Section("Info") {
                    SettingsRow(label: "Version", value: TomVersion.display, monospaced: true)
                    SettingsRow(label: "Status", value: nodeService.state.rawValue, valueColor: stateColor)
                    SettingsRow(label: "Peers", value: "\(nodeService.peersCount)")
                    SettingsRow(label: "Groups", value: "\(nodeService.groupsCount)")
                    SettingsRow(label: "Messages", value: "\(nodeService.totalMessagesCount)")
                }
            }
        }
        .confirmationDialog("Oublier le réseau ?", isPresented: $confirmNetworkReset, titleVisibility: .visible) {
            Button("Oublier le réseau", role: .destructive) {
                Task { await performReset(level: .network) }
            }
            Button("Annuler", role: .cancel) {}
        } message: {
            Text("Efface topologie, groupes, backup, noms connus. GARDE l'identité et les compteurs. Le nœud reste le même, amnésique.")
        }
        .confirmationDialog("Réinitialisation usine ?", isPresented: $confirmFactoryReset, titleVisibility: .visible) {
            Button("Tout effacer (nouveau node_id)", role: .destructive) {
                Task { await performReset(level: .factory) }
            }
            Button("Annuler", role: .cancel) {}
        } message: {
            Text("Efface AUSSI l'identité et les compteurs. Au prochain démarrage, le nœud aura un node_id NEUF (nouveau contact jamais vu).")
        }
    }

    private var stateColor: Color {
        switch nodeService.state {
        case .running: return .green
        case .starting, .stopping: return .orange
        case .error: return .red
        case .stopped: return .gray
        }
    }
}

#Preview {
    SettingsView()
        .environmentObject(TomNodeService.shared)
}
