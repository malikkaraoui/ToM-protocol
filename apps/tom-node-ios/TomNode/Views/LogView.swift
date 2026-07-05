import SwiftUI
import TomProtocolKit

struct LogView: View {
    @EnvironmentObject var nodeService: TomNodeService

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Text("Live Log")
                    .font(.headline)
                Spacer()
                Text("\(nodeService.logEntries.count) events")
                    .font(.caption)
                    .foregroundColor(.secondary)
                Button("Clear") {
                    nodeService.logEntries.removeAll()
                }
                .font(.caption)
            }
            .padding(.horizontal, 40)
            .padding(.top, 20)

            ScrollViewReader { proxy in
                ScrollView {
                    LazyVStack(alignment: .leading, spacing: 2) {
                        ForEach(nodeService.logEntries) { entry in
                            HStack(alignment: .top, spacing: 8) {
                                Text(entry.timestamp)
                                    .font(.system(.caption2, design: .monospaced))
                                    .foregroundColor(.secondary)
                                    .frame(width: 100, alignment: .leading)

                                Circle()
                                    .fill(entry.levelColor)
                                    .frame(width: 8, height: 8)
                                    .padding(.top, 4)

                                Text(String(entry.message.prefix(500)))
                                    .font(.system(.caption, design: .monospaced))
                                    .foregroundColor(entry.levelColor)
                                    .lineLimit(3)
                            }
                            .id(entry.id)
                        }
                    }
                    .padding(.horizontal, 40)
                }
                .onChange(of: nodeService.logEntries.count) { _ in
                    if let last = nodeService.logEntries.last {
                        withAnimation {
                            proxy.scrollTo(last.id, anchor: .bottom)
                        }
                    }
                }
            }
        }
    }
}

struct LogEntry: Identifiable {
    let id = UUID()
    let date: Date
    let level: LogLevel
    let message: String

    var timestamp: String {
        let f = DateFormatter()
        f.dateFormat = "HH:mm:ss.SS"
        return f.string(from: date)
    }

    var levelColor: Color {
        switch level {
        case .info: return .white
        case .success: return .green
        case .warning: return .orange
        case .error: return .red
        case .network: return .cyan
        case .echo: return .purple
        }
    }
}

enum LogLevel {
    case info, success, warning, error, network, echo
}

#Preview {
    LogView()
        .environmentObject(TomNodeService.shared)
}
