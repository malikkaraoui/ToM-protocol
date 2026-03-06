import SwiftUI

struct StatusView: View {
    @EnvironmentObject var nodeService: TomNodeService

    var body: some View {
        ScrollView {
            VStack(spacing: 24) {
                // Header
                Text("ToM Node")
                    .font(.largeTitle)
                    .fontWeight(.bold)

                // Status indicator
                HStack(spacing: 12) {
                    Circle()
                        .fill(statusColor)
                        .frame(width: 16, height: 16)
                    Text(nodeService.state.rawValue)
                        .font(.title2)
                        .foregroundColor(statusColor)
                }

                // Node ID
                if !nodeService.nodeId.isEmpty {
                    VStack(spacing: 8) {
                        Text("Node ID")
                            .font(.headline)
                            .foregroundColor(.secondary)
                        Text(nodeService.nodeId)
                            .font(.system(.caption, design: .monospaced))
                            .lineLimit(2)
                            .multilineTextAlignment(.center)
                    }
                    .padding()
                    .background(Color.secondary.opacity(0.1))
                    .cornerRadius(12)
                }

                // Stats
                HStack(spacing: 40) {
                    StatBox(title: "Peers", value: "\(nodeService.peersCount)", icon: "person.2")
                    StatBox(title: "Groups", value: "\(nodeService.groupsCount)", icon: "person.3")
                    StatBox(title: "Messages", value: "\(nodeService.messages.count)", icon: "message")
                }

                // Control buttons
                HStack(spacing: 20) {
                    if nodeService.state == .stopped || nodeService.state == .error {
                        Button(action: { nodeService.start() }) {
                            Label("Start", systemImage: "play.fill")
                                .frame(minWidth: 120)
                        }
                        .buttonStyle(.borderedProminent)
                        .tint(.green)
                    }

                    if nodeService.state == .running {
                        Button(action: { nodeService.stop() }) {
                            Label("Stop", systemImage: "stop.fill")
                                .frame(minWidth: 120)
                        }
                        .buttonStyle(.bordered)
                        .tint(.red)
                    }

                    if nodeService.state == .starting || nodeService.state == .stopping {
                        ProgressView()
                    }
                }

                // Error display
                if let error = nodeService.errorMessage {
                    Text(error)
                        .foregroundColor(.red)
                        .font(.callout)
                        .padding()
                        .background(Color.red.opacity(0.1))
                        .cornerRadius(8)
                }
            }
            .padding(40)
        }
    }

    private var statusColor: Color {
        switch nodeService.state {
        case .running: return .green
        case .starting, .stopping: return .orange
        case .error: return .red
        case .stopped: return .gray
        }
    }
}

struct StatBox: View {
    let title: String
    let value: String
    let icon: String

    var body: some View {
        VStack(spacing: 8) {
            Image(systemName: icon)
                .font(.title2)
                .foregroundColor(.accentColor)
            Text(value)
                .font(.title)
                .fontWeight(.bold)
            Text(title)
                .font(.caption)
                .foregroundColor(.secondary)
        }
        .frame(minWidth: 100)
        .padding()
        .background(Color.secondary.opacity(0.1))
        .cornerRadius(12)
    }
}

#Preview {
    StatusView()
        .environmentObject(TomNodeService.shared)
}
