import SwiftUI
import TomProtocolKit

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

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            Text("Settings")
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
                    if !nodeService.connectedPeers.isEmpty {
                        ForEach(nodeService.connectedPeers, id: \.self) { peer in
                            SettingsRow(
                                label: "Connected",
                                value: String(peer.prefix(8)) + "..." + String(peer.suffix(4)),
                                monospaced: true,
                                valueColor: .green
                            )
                        }
                    }
                    if !nodeService.discoveredPeers.isEmpty {
                        ForEach(nodeService.discoveredPeers) { peer in
                            SettingsRow(
                                label: "Discovered",
                                value: peer.displayName,
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

                Section("Profile") {
                    SettingsRow(label: "Username", value: nodeService.username)
                }

                Section("Info") {
                    SettingsRow(label: "Status", value: nodeService.state.rawValue, valueColor: stateColor)
                    SettingsRow(label: "Peers", value: "\(nodeService.peersCount)")
                    SettingsRow(label: "Groups", value: "\(nodeService.groupsCount)")
                    SettingsRow(label: "Messages", value: "\(nodeService.totalMessagesCount)")
                }
            }
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
