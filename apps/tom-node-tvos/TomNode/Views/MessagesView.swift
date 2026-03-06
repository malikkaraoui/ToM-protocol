import SwiftUI

struct MessagesView: View {
    @EnvironmentObject var nodeService: TomNodeService
    @State private var showSendSheet = false
    @State private var targetPeerId = ""
    @State private var messageText = ""

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
                    List(nodeService.messages.reversed()) { message in
                        MessageRow(message: message)
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

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack {
                Text(message.senderShortId)
                    .font(.system(.caption, design: .monospaced))
                    .foregroundColor(.accentColor)
                Spacer()
                Text(message.date, style: .time)
                    .font(.caption2)
                    .foregroundColor(.secondary)
            }
            Text(message.text)
                .font(.body)
            HStack(spacing: 8) {
                if message.wasEncrypted {
                    Label("Encrypted", systemImage: "lock.fill")
                        .font(.caption2)
                        .foregroundColor(.green)
                }
                if message.signatureValid {
                    Label("Signed", systemImage: "checkmark.seal.fill")
                        .font(.caption2)
                        .foregroundColor(.blue)
                }
            }
        }
        .padding(.vertical, 4)
    }
}

struct SendMessageSheet: View {
    @Binding var targetPeerId: String
    @Binding var messageText: String
    let onSend: () -> Void

    var body: some View {
        NavigationStack {
            Form {
                Section("Recipient") {
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
