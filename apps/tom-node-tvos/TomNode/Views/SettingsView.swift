import SwiftUI

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
                    SettingsRow(label: "Relay URL", value: nodeService.relayUrl, monospaced: true)

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

                Section("Profile") {
                    SettingsRow(label: "Username", value: nodeService.username)
                }

                Section("Info") {
                    SettingsRow(label: "Status", value: nodeService.state.rawValue, valueColor: stateColor)
                    SettingsRow(label: "Peers", value: "\(nodeService.peersCount)")
                    SettingsRow(label: "Groups", value: "\(nodeService.groupsCount)")
                    SettingsRow(label: "Messages", value: "\(nodeService.messages.count)")
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
