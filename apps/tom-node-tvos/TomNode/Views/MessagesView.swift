import SwiftUI
import TomProtocolKit

struct MessagesView: View {
    @EnvironmentObject var nodeService: TomNodeService
    @State private var showSendSheet = false
    @State private var targetPeerId = ""
    @State private var messageText = ""

    /// Resolve a NodeId to a display name via the service's central mapping
    private func displayName(for nodeId: NodeId) -> String {
        nodeService.displayName(for: nodeId)
    }

    var body: some View {
        NavigationStack {
            Group {
                if nodeService.messages.isEmpty {
                    VStack(spacing: 16) {
                        Image(systemName: "message")
                            .font(.system(size: 48))
                            .foregroundColor(.secondary)
                        Text("No messages yet")
                            .font(.title3)
                            .foregroundColor(.secondary)
                        if nodeService.state == .running {
                            Text("Messages will appear here when received")
                                .font(.callout)
                                .foregroundColor(.secondary)
                        } else {
                            Text("Start the node to send and receive messages")
                                .font(.callout)
                                .foregroundColor(.secondary)
                        }
                    }
                } else {
                    List(nodeService.messages.sorted { $0.date > $1.date }) { message in
                        MessageRow(
                            message: message,
                            senderName: displayName(for: message.from),
                            recipientName: message.to.map { displayName(for: $0) } ?? ""
                        )
                    }
                }
            }
            .navigationTitle("Messages")
            .toolbar {
                if nodeService.state == .running {
                    Button(action: { showSendSheet = true }) {
                        Image(systemName: "square.and.pencil")
                    }
                }
            }
            .sheet(isPresented: $showSendSheet) {
                SendMessageSheet(
                    targetPeerId: $targetPeerId,
                    messageText: $messageText,
                    discoveredPeers: nodeService.discoveredPeers,
                    onSend: {
                        nodeService.sendMessage(to: targetPeerId, text: messageText)
                        messageText = ""
                        showSendSheet = false
                    }
                )
            }
        }
    }
}

struct MessageRow: View {
    let message: TomMessage
    var senderName: String = ""
    var recipientName: String = ""

    /// « moi » pour le nœud local, sinon le nom résolu ou l'ID court.
    private var senderLabel: String {
        message.isOutgoing ? "moi" : (senderName.isEmpty ? message.senderShortId : senderName)
    }
    private var recipientLabel: String {
        message.isOutgoing ? (recipientName.isEmpty ? "?" : recipientName) : "moi"
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(spacing: 6) {
                // expéditeur ≫ destinataire
                Text(senderLabel)
                    .font(.system(.caption, design: .monospaced))
                    .foregroundColor(.accentColor)
                Image(systemName: "arrow.right")
                    .font(.caption2)
                    .foregroundColor(.secondary)
                Text(recipientLabel)
                    .font(.system(.caption, design: .monospaced))
                    .foregroundColor(.primary)
                Spacer()
                // Badge AUTO vs MANUEL (messages sortants uniquement)
                if message.isOutgoing {
                    Text(message.isAuto ? "AUTO" : "MANUEL")
                        .font(.caption2.weight(.bold))
                        .padding(.horizontal, 6).padding(.vertical, 2)
                        .background((message.isAuto ? Color.orange : Color.blue).opacity(0.18))
                        .foregroundColor(message.isAuto ? .orange : .blue)
                        .clipShape(Capsule())
                }
                Text(message.date, style: .time)
                    .font(.caption2)
                    .foregroundColor(.secondary)
            }
            // Défense : borner la chaîne passée au typographe (un payload de
            // plusieurs Mo dans un Text bloque le main thread → watchdog kill).
            Text(String(message.text.prefix(500)))
                .font(.body)
            HStack(spacing: 8) {
                // Statut de livraison (messages sortants) : en cours → relayé →
                // délivré → purgé (ou échec).
                if let st = message.deliveryStatus {
                    Label(st.label, systemImage: st.icon)
                        .font(.caption2)
                        .foregroundColor(statusColor(st))
                }
                if message.wasEncrypted {
                    Label("Chiffré", systemImage: "lock.fill")
                        .font(.caption2)
                        .foregroundColor(.green)
                }
                if message.signatureValid {
                    Label("Signé", systemImage: "checkmark.seal.fill")
                        .font(.caption2)
                        .foregroundColor(.blue)
                }
            }
        }
        .padding(.vertical, 4)
    }

    private func statusColor(_ s: TomDeliveryStatus) -> Color {
        switch s {
        case .sending: return .secondary
        case .relayed: return .orange
        case .delivered, .purged: return .green
        case .failed: return .red
        }
    }
}

struct SendMessageSheet: View {
    @Binding var targetPeerId: String
    @Binding var messageText: String
    let discoveredPeers: [TomPeer]
    let onSend: () -> Void

    var body: some View {
        NavigationStack {
            Form {
                // Peer picker — select from discovered peers (with username)
                if !discoveredPeers.isEmpty {
                    Section("Discovered Peers") {
                        ForEach(discoveredPeers) { peer in
                            Button(action: { targetPeerId = peer.nodeId }) {
                                HStack {
                                    VStack(alignment: .leading) {
                                        Text(peer.displayName)
                                            .font(.body)
                                        Text(peer.shortId)
                                            .font(.system(.caption2, design: .monospaced))
                                            .foregroundColor(.secondary)
                                    }
                                    Spacer()
                                    if targetPeerId == peer.nodeId {
                                        Image(systemName: "checkmark.circle.fill")
                                            .foregroundColor(.green)
                                    }
                                }
                            }
                        }
                    }
                }

                // Manual entry fallback
                Section("Or enter manually") {
                    TextField("Peer Node ID (hex)", text: $targetPeerId)
                        .font(.system(.body, design: .monospaced))
                }

                Section("Message") {
                    TextField("Type your message...", text: $messageText)
                }
            }
            .navigationTitle("Send Message")
            .toolbar {
                ToolbarItem(placement: .confirmationAction) {
                    Button("Send", action: onSend)
                        .disabled(targetPeerId.isEmpty || messageText.isEmpty)
                }
            }
        }
    }
}

#Preview {
    MessagesView()
        .environmentObject(TomNodeService.shared)
}
