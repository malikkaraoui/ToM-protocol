import Foundation
import os.log
import UIKit
import AVFoundation
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
    @Published var discoveredPeers: [TomPeer] = []
    @Published var errorMessage: String?

    // Live log
    @Published var logEntries: [LogEntry] = []

    // Auto-echo for stress testing
    @Published var autoEchoEnabled: Bool = true
    @Published var echoCount: Int = 0

    // Anti-sleep audio player
    private var silentPlayer: AVAudioPlayer?

    // Config
    @Published var relayUrl: String = "http://82.67.95.8:3340"
    @Published var username: String = "AppleTV"
    @Published var encryption: Bool = true
    @Published var enableDht: Bool = true
    @Published var n0Discovery: Bool = true

    /// Bootstrap peer for gossip discovery (Freebox NAS — network seed node)
    /// This is only needed while the network is young; once enough peers exist,
    /// gossip propagates organically and no fixed bootstrap is required.
    @Published var bootstrapPeerId: String = "4e28f4706e0dcb01f13d74a9ea00d3bdfc62490c2f4a91f7cb8b14bed6a45814"

    /// Track if the node was running before the app went to background
    private var wasRunningBeforeSleep = false

    private init() {}

    // MARK: - Logging

    func appendLog(_ level: LogLevel, _ message: String) {
        let entry = LogEntry(date: Date(), level: level, message: message)
        logEntries.append(entry)
        // Keep last 1000 entries
        if logEntries.count > 1000 {
            logEntries.removeFirst(logEntries.count - 1000)
        }
    }

    // MARK: - Anti-sleep

    func startAntiSleep() {
        // Disable idle timer (official API to prevent screen dimming/sleep)
        UIApplication.shared.isIdleTimerDisabled = true

        // Play a silent audio loop to prevent tvOS from sleeping
        let silenceData = Data(count: 44100 * 2) // 1s of silence (16-bit mono 44.1kHz)
        var wavHeader = Data()
        // WAV header for 1s silence
        let dataSize = UInt32(silenceData.count)
        let fileSize = UInt32(36 + silenceData.count)
        wavHeader.append(contentsOf: [0x52, 0x49, 0x46, 0x46]) // RIFF
        wavHeader.append(contentsOf: withUnsafeBytes(of: fileSize.littleEndian) { Array($0) })
        wavHeader.append(contentsOf: [0x57, 0x41, 0x56, 0x45]) // WAVE
        wavHeader.append(contentsOf: [0x66, 0x6D, 0x74, 0x20]) // fmt
        wavHeader.append(contentsOf: withUnsafeBytes(of: UInt32(16).littleEndian) { Array($0) })
        wavHeader.append(contentsOf: withUnsafeBytes(of: UInt16(1).littleEndian) { Array($0) }) // PCM
        wavHeader.append(contentsOf: withUnsafeBytes(of: UInt16(1).littleEndian) { Array($0) }) // mono
        wavHeader.append(contentsOf: withUnsafeBytes(of: UInt32(44100).littleEndian) { Array($0) }) // sample rate
        wavHeader.append(contentsOf: withUnsafeBytes(of: UInt32(88200).littleEndian) { Array($0) }) // byte rate
        wavHeader.append(contentsOf: withUnsafeBytes(of: UInt16(2).littleEndian) { Array($0) }) // block align
        wavHeader.append(contentsOf: withUnsafeBytes(of: UInt16(16).littleEndian) { Array($0) }) // bits per sample
        wavHeader.append(contentsOf: [0x64, 0x61, 0x74, 0x61]) // data
        wavHeader.append(contentsOf: withUnsafeBytes(of: dataSize.littleEndian) { Array($0) })
        wavHeader.append(silenceData)

        do {
            try AVAudioSession.sharedInstance().setCategory(.playback, mode: .default)
            try AVAudioSession.sharedInstance().setActive(true)
            silentPlayer = try AVAudioPlayer(data: wavHeader)
            silentPlayer?.numberOfLoops = -1 // infinite loop
            silentPlayer?.volume = 0.01 // near-silent
            silentPlayer?.play()
            appendLog(.info, "Anti-sleep: silent audio loop started")
        } catch {
            appendLog(.warning, "Anti-sleep failed: \(error.localizedDescription)")
        }
    }

    func stopAntiSleep() {
        UIApplication.shared.isIdleTimerDisabled = false
        silentPlayer?.stop()
        silentPlayer = nil
        try? AVAudioSession.sharedInstance().setActive(false)
    }

    func start() {
        guard state == .stopped || state == .error else { return }
        state = .starting
        errorMessage = nil
        echoCount = 0

        appendLog(.info, "Starting node...")
        appendLog(.info, "Relay: \(relayUrl)")
        appendLog(.info, "Username: \(username), Encryption: \(encryption)")
        appendLog(.info, "DHT: \(enableDht), n0Discovery: \(n0Discovery)")

        Task {
            do {
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
                appendLog(.success, "Node handle created")

                let bootstrapPeers = bootstrapPeerId.isEmpty ? [] : [bootstrapPeerId]
                try await node.start(
                    username: username,
                    encryption: encryption,
                    enableDht: enableDht,
                    relayUrl: relayUrl,
                    identityPath: identityPath,
                    n0Discovery: n0Discovery,
                    dataDir: dataDir,
                    gossipBootstrapPeers: bootstrapPeers
                )

                state = .running
                appendLog(.success, "Runtime started")
                if !bootstrapPeers.isEmpty {
                    appendLog(.network, "Bootstrap peers: \(bootstrapPeers.map { String($0.prefix(8)) })")
                }

                startAntiSleep()
                startPolling()
                log.info("Node started — identity: \(self.identityPath ?? "ephemeral"), data: \(self.dataDir ?? "none")")

            } catch {
                log.error("Failed to start node: \(error.localizedDescription)")
                appendLog(.error, "START FAILED: \(error.localizedDescription)")
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
        stopAntiSleep()

        appendLog(.info, "Stopping node...")

        Task {
            await node.stop()
            state = .stopped
            nodeId = ""
            peersCount = 0
            groupsCount = 0
            appendLog(.info, "Node stopped. Echo count: \(echoCount)")
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

    // MARK: - Lifecycle

    /// Called when the app returns to foreground (after tvOS sleep).
    /// The old tokio runtime is dead — force-reset and auto-restart if needed.
    func handleReturnToForeground() {
        guard state == .running else { return }

        appendLog(.warning, "FOREGROUND RETURN — restarting node (connections lost)")
        log.info("Returning to foreground — restarting node (connections lost during sleep)")
        pollTask?.cancel()
        pollTask = nil
        stopAntiSleep()

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
        let dir = FileManager.default.urls(for: .cachesDirectory, in: .userDomainMask).first
        return dir?.appendingPathComponent("tom_identity.key").path
    }

    private var dataDir: String? {
        let dir = FileManager.default.urls(for: .cachesDirectory, in: .userDomainMask).first
        return dir?.appendingPathComponent("tom_data").path
    }

    private func startPolling() {
        var knownPeerIds = Set<String>()
        var lastPeerCount = 0

        pollTask = Task { [weak self] in
            while !Task.isCancelled {
                guard let self = self else { break }

                // Poll messages
                let newMessages = await self.node.receiveMessages()
                for msg in newMessages {
                    self.messages.append(msg)
                    let senderShort = String(msg.from.prefix(8))
                    let textPreview = String(msg.text.prefix(80))
                    self.appendLog(.network, "MSG from \(senderShort): \(textPreview)")

                    // Auto-echo: reply to incoming messages for stress testing
                    if self.autoEchoEnabled {
                        do {
                            let echoPayload = "echo:\(msg.text)".data(using: .utf8) ?? Data()
                            try await self.node.sendMessage(to: msg.from, payload: echoPayload)
                            self.echoCount += 1
                            if self.echoCount <= 10 || self.echoCount % 100 == 0 {
                                self.appendLog(.echo, "ECHO #\(self.echoCount) → \(senderShort)")
                            }
                        } catch {
                            self.appendLog(.error, "Echo failed → \(senderShort): \(error.localizedDescription)")
                        }
                    }
                }
                // Keep last 500 messages
                if self.messages.count > 500 {
                    self.messages = Array(self.messages.suffix(500))
                }

                // Poll status + peers
                if let status = await self.node.status() {
                    self.nodeId = status.nodeId
                    self.peersCount = status.peersCount
                    self.groupsCount = status.groupsCount
                }
                let currentConnected = await self.node.connectedPeers()
                self.connectedPeers = currentConnected

                let currentDiscovered = await self.node.discoveredPeers()
                // Log new peer discoveries
                for peer in currentDiscovered {
                    if !knownPeerIds.contains(peer.nodeId) {
                        knownPeerIds.insert(peer.nodeId)
                        let name = peer.username.isEmpty ? peer.shortId : "\(peer.username) (\(peer.shortId))"
                        self.appendLog(.success, "PEER DISCOVERED: \(name) via \(peer.source)")
                    }
                }
                self.discoveredPeers = currentDiscovered

                // Log peer count changes
                let peerCount = currentDiscovered.count
                if peerCount != lastPeerCount {
                    self.appendLog(.info, "Peers: \(lastPeerCount) → \(peerCount)")
                    lastPeerCount = peerCount
                }

                try? await Task.sleep(nanoseconds: 250_000_000) // 250ms (faster polling)
            }
        }
    }
}
