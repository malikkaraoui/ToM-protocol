import Foundation
import os.log
import Combine

@MainActor
final class TomNodeService: ObservableObject {
    static let shared = TomNodeService()

    private let log = Logger(subsystem: "org.tom-protocol.tom-node", category: "TomNodeService")
    private let node = TomNodeWrapper()
    private var pollTask: Task<Void, Never>?

    @Published var state: TomNodeState = .stopped
    @Published var nodeId: String = ""
    @Published var peersCount: Int = 0
    @Published var groupsCount: Int = 0
    @Published var messages: [TomMessage] = []
    @Published var connectedPeers: [NodeId] = []
    @Published var errorMessage: String?

    // Config
    @Published var relayUrl: String = "http://82.67.95.8:3340"
    @Published var username: String = "AppleTV"
    @Published var encryption: Bool = true
    @Published var enableDht: Bool = true
    @Published var n0Discovery: Bool = true
    @Published var nasPeerNodeId: String = "90cbf4dee5d41a524107b7fa6980590b92bd2d6586900f722b95e51c7eb60ec6"

    /// Track if the node was running before the app went to background
    private var wasRunningBeforeSleep = false

    private init() {}

    func start() {
        guard state == .stopped || state == .error else { return }
        state = .starting
        errorMessage = nil

        Task {
            do {
                // Ensure data directory exists for SQLite persistence
                if let dir = dataDir {
                    try? FileManager.default.createDirectory(
                        atPath: dir,
                        withIntermediateDirectories: true
                    )
                }

                try await node.create(
                    relayUrl: relayUrl,
                    identityPath: identityPath,
                    n0Discovery: n0Discovery
                )

                try await node.start(
                    username: username,
                    encryption: encryption,
                    enableDht: enableDht,
                    relayUrl: relayUrl,
                    identityPath: identityPath,
                    n0Discovery: n0Discovery,
                    dataDir: dataDir
                )

                state = .running
                startPolling()
                log.info("Node started — identity: \(self.identityPath ?? "ephemeral"), data: \(self.dataDir ?? "none")")

                // Auto-add NAS responder peer via relay
                await addNasPeer()
            } catch {
                log.error("Failed to start node: \(error.localizedDescription)")
                state = .error
                errorMessage = error.localizedDescription
            }
        }
    }

    func stop() {
        guard state == .running else { return }
        state = .stopping
        pollTask?.cancel()
        pollTask = nil

        Task {
            await node.stop()
            state = .stopped
            nodeId = ""
            peersCount = 0
            groupsCount = 0
            log.info("Node stopped")
        }
    }

    func sendMessage(to target: NodeId, text: String) {
        guard state == .running else { return }
        Task {
            do {
                guard let data = text.data(using: .utf8) else { return }
                try await node.sendMessage(to: target, payload: data)
                log.info("Message sent to \(target.prefix(8))...")

                // Add sent message to local list
                let sent = TomMessage(
                    id: UUID().uuidString,
                    from: nodeId,
                    payload: data.base64EncodedString(),
                    timestamp: UInt64(Date().timeIntervalSince1970 * 1000),
                    signatureValid: true,
                    wasEncrypted: true
                )
                messages.append(sent)
            } catch {
                log.error("Send failed: \(error.localizedDescription)")
                errorMessage = error.localizedDescription
            }
        }
    }

    func createGroup(name: String, members: [NodeId]) {
        guard state == .running else { return }
        Task {
            do {
                try await node.createGroup(name: name, members: members, inviteOnly: false)
                log.info("Group create command sent: \(name)")
            } catch {
                log.error("Group create failed: \(error.localizedDescription)")
                errorMessage = error.localizedDescription
            }
        }
    }

    func sendGroupMessage(groupId: GroupId, text: String) {
        guard state == .running else { return }
        Task {
            do {
                try await node.sendGroupMessage(groupId: groupId, text: text)
                log.info("Group message sent to \(groupId.prefix(8))...")
            } catch {
                log.error("Group send failed: \(error.localizedDescription)")
                errorMessage = error.localizedDescription
            }
        }
    }

    func addPeer(nodeId: NodeId, relayUrl: String? = nil) {
        guard state == .running else { return }
        Task {
            do {
                try await node.addPeerAddr(nodeId: nodeId, relayUrl: relayUrl)
                log.info("Added peer: \(nodeId.prefix(8))...")
            } catch {
                log.error("Add peer failed: \(error.localizedDescription)")
            }
        }
    }

    /// Auto-add NAS responder (Freebox) via the relay
    private func addNasPeer() async {
        // NAS responder node ID — set by tom-stress on NAS
        // This is populated dynamically; check Settings for the current value
        guard !nasPeerNodeId.isEmpty else {
            log.info("No NAS peer configured — skipping auto-add")
            return
        }
        do {
            try await node.addPeerAddr(
                nodeId: self.nasPeerNodeId,
                relayUrl: self.relayUrl
            )
            log.info("Auto-added NAS peer: \(self.nasPeerNodeId.prefix(8))...")
        } catch {
            log.error("Failed to add NAS peer: \(error.localizedDescription)")
        }
    }

    // MARK: - Lifecycle

    /// Called when the app returns to foreground (after tvOS sleep).
    /// The old tokio runtime is dead — force-reset and auto-restart if needed.
    func handleReturnToForeground() {
        guard state == .running else { return }

        log.info("Returning to foreground — restarting node (connections lost during sleep)")
        pollTask?.cancel()
        pollTask = nil

        Task {
            await node.forceReset()
            state = .stopped
            nodeId = ""
            peersCount = 0
            groupsCount = 0

            // Auto-restart
            start()
        }
    }

    // MARK: - Private

    private var identityPath: String? {
        let dir = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask).first
        return dir?.appendingPathComponent("tom_identity.key").path
    }

    private var dataDir: String? {
        let dir = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask).first
        return dir?.appendingPathComponent("tom_data").path
    }

    private func startPolling() {
        pollTask = Task { [weak self] in
            while !Task.isCancelled {
                guard let self = self else { break }

                // Poll messages
                let newMessages = await self.node.receiveMessages()
                if !newMessages.isEmpty {
                    self.messages.append(contentsOf: newMessages)
                    // Keep last 500 messages
                    if self.messages.count > 500 {
                        self.messages = Array(self.messages.suffix(500))
                    }
                }

                // Poll status + peers
                if let status = await self.node.status() {
                    self.nodeId = status.nodeId
                    self.peersCount = status.peersCount
                    self.groupsCount = status.groupsCount
                }
                self.connectedPeers = await self.node.connectedPeers()

                try? await Task.sleep(nanoseconds: 500_000_000) // 500ms
            }
        }
    }
}
