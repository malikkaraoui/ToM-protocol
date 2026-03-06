import SwiftUI

struct SettingsView: View {
    @EnvironmentObject var nodeService: TomNodeService

    var body: some View {
        NavigationStack {
            Form {
                Section("Node Identity") {
                    if !nodeService.nodeId.isEmpty {
                        LabeledContent("Node ID") {
                            Text(nodeService.nodeId)
                                .font(.system(.caption2, design: .monospaced))
                                .lineLimit(1)
                                .truncationMode(.middle)
                        }
                    } else {
                        Text("Node not started")
                            .foregroundColor(.secondary)
                    }
                }

                Section("Network") {
                    LabeledContent("Relay URL") {
                        Text(nodeService.relayUrl)
                            .font(.system(.body, design: .monospaced))
                    }

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
                    LabeledContent("Username") {
                        Text(nodeService.username)
                    }
                }

                Section("Info") {
                    LabeledContent("Status") {
                        Text(nodeService.state.rawValue)
                            .foregroundColor(stateColor)
                    }
                    LabeledContent("Peers") {
                        Text("\(nodeService.peersCount)")
                    }
                    LabeledContent("Groups") {
                        Text("\(nodeService.groupsCount)")
                    }
                    LabeledContent("Messages") {
                        Text("\(nodeService.messages.count)")
                    }
                }
            }
            .navigationTitle("Settings")
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
