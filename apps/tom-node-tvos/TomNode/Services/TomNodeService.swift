import Foundation
import os.log
import UIKit
import AVFoundation
import Combine

@MainActor
final class TomNodeService: ObservableObject {
    static let shared = TomNodeService()

    private static let autoStartDelayNanoseconds: UInt64 = 5_000_000_000
    private static let autoMessageRetryInterval: TimeInterval = 5

    private let log = Logger(subsystem: "org.tom-protocol.tom-node", category: "TomNodeService")
    private let node = TomNodeWrapper()
    private var pollTask: Task<Void, Never>?
    private var autoStartTask: Task<Void, Never>?
    private var hasScheduledInitialAutoStart = false
    private var autoMessagedPeerIds = Set<String>()
    private var autoMessageAttemptedAt: [String: Date] = [:]
    private var seededPeerIds = Set<String>()

    @Published var state: TomNodeState = .stopped
    @Published var nodeId: String = ""
    @Published var peersCount: Int = 0
    @Published var groupsCount: Int = 0
    @Published var messages: [TomMessage] = []
    @Published var totalMessagesCount: Int = 0
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

    // Config — defaults work out of the box, zero manual configuration
    // Default relay = NAS Freebox (always running tom-relay --dev on port 3340)
    @Published var relayUrl: String = "http://192.168.0.83:3340"
    @Published var username: String = TomNodeService.defaultUsername()
    @Published var encryption: Bool = true
    @Published var enableDht: Bool = true
    @Published var n0Discovery: Bool = true
    @Published var udpLogExportEnabled: Bool = true
    @Published var udpLogHost: String = TomNodeService.defaultCollectorHost()
    @Published var udpLogPort: String = "9999"

    /// Device-aware username: "iPad", "iPhone", or "AppleTV"
    private static func defaultUsername() -> String {
        #if os(iOS)
        return UIDevice.current.userInterfaceIdiom == .pad ? "iPad" : "iPhone"
        #else
        return "AppleTV"
        #endif
    }

    /// Detect the subnet broadcast address from the device's Wi-Fi IP.
    /// On a 192.168.0.x network → returns 192.168.0.255.
    /// Falls back to 255.255.255.255 if detection fails.
    private static func defaultCollectorHost() -> String {
        if let localIP = getLocalIPv4() {
            let parts = localIP.split(separator: ".")
            if parts.count == 4 {
                return "\(parts[0]).\(parts[1]).\(parts[2]).255"
            }
        }
        return "255.255.255.255"
    }

    /// Get the device's local IPv4 address (Wi-Fi preferred).
    private static func getLocalIPv4() -> String? {
        var ifaddr: UnsafeMutablePointer<ifaddrs>?
        guard getifaddrs(&ifaddr) == 0, let firstAddr = ifaddr else { return nil }
        defer { freeifaddrs(ifaddr) }

        var bestIP: String?

        for ptr in sequence(first: firstAddr, next: { $0.pointee.ifa_next }) {
            guard let addr = ptr.pointee.ifa_addr else { continue }
            let flags = Int32(ptr.pointee.ifa_flags)

            guard addr.pointee.sa_family == UInt8(AF_INET) else { continue }
            guard (flags & (IFF_UP | IFF_RUNNING)) != 0 else { continue }
            guard (flags & IFF_LOOPBACK) == 0 else { continue }

            var hostname = [CChar](repeating: 0, count: Int(NI_MAXHOST))
            if getnameinfo(addr, socklen_t(addr.pointee.sa_len),
                           &hostname, socklen_t(hostname.count),
                           nil, 0, NI_NUMERICHOST) == 0 {
                let ip = String(cString: hostname)
                let name = String(cString: ptr.pointee.ifa_name)
                // en0 = Wi-Fi on iOS/tvOS
                if name == "en0" { return ip }
                if bestIP == nil { bestIP = ip }
            }
        }

        return bestIP
    }

    /// Optional bootstrap peer for early-network gossip seeding.
    /// Leave empty to rely on organic discovery (n0/DHT/gossip learned peers).
    @Published var bootstrapPeerId: String = ""

    /// Track if the node was running before the app went to background
    private var wasRunningBeforeSleep = false

    private init() {}

    private var normalizedRelayUrl: String? {
        let trimmed = relayUrl.trimmingCharacters(in: .whitespacesAndNewlines)
        return trimmed.isEmpty ? nil : trimmed
    }

    private var normalizedBootstrapPeers: [String] {
        let trimmed = bootstrapPeerId.trimmingCharacters(in: .whitespacesAndNewlines)
        return trimmed.isEmpty ? [] : [trimmed]
    }

    var relayStatusLabel: String {
        normalizedRelayUrl ?? "Automatic discovery / public fallback"
    }

    var bootstrapStatusLabel: String {
        normalizedBootstrapPeers.isEmpty ? "Organic discovery only" : String(normalizedBootstrapPeers[0].prefix(12)) + "…"
    }

    // MARK: - Logging

    func appendLog(_ level: LogLevel, _ message: String) {
        let entry = LogEntry(date: Date(), level: level, message: message)
        logEntries.append(entry)
        // Keep last 1000 entries
        if logEntries.count > 1000 {
            logEntries.removeFirst(logEntries.count - 1000)
        }
        // Broadcast structured JSON over UDP for remote monitoring
        let escaped = message.replacingOccurrences(of: "\"", with: "\\\"")
            .replacingOccurrences(of: "\n", with: " ")
        let json = """
        {"ts":"\(entry.timestamp)","node":"\(username)","appareil":"tvos","event":"\(level)","detail":"\(escaped)","phase":"\(state == .running ? "connecte" : "arret")","taille_reseau":\(peersCount),"role":"participant","msgs_recv":\(totalMessagesCount)}
        """
        sendLogUDP(json.trimmingCharacters(in: .whitespacesAndNewlines))
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
        autoStartTask?.cancel()
        autoStartTask = nil
        autoMessagedPeerIds.removeAll()
        autoMessageAttemptedAt.removeAll()
        seededPeerIds.removeAll()
        stopNetworkLogExport()
        startNetworkLogExportIfNeeded()

        appendLog(.info, "Starting node...")
        appendLog(.info, "Relay mode: \(relayStatusLabel)")
        appendLog(.info, "Username: \(username), Encryption: \(encryption)")
        appendLog(.info, "DHT: \(enableDht), n0Discovery: \(n0Discovery)")
        appendLog(.info, "Bootstrap mode: \(bootstrapStatusLabel)")

        Task {
            do {
                if let dir = dataDir {
                    try? FileManager.default.createDirectory(
                        atPath: dir,
                        withIntermediateDirectories: true
                    )
                }

                try await node.create(
                    relayUrl: normalizedRelayUrl,
                    identityPath: identityPath,
                    n0Discovery: n0Discovery
                )
                appendLog(.success, "Node handle created")

                let bootstrapPeers = normalizedBootstrapPeers
                let localDiscovery = true  // Always enable local discovery (mDNS) — finds peers on same LAN
                try await node.start(
                    username: username,
                    encryption: encryption,
                    enableDht: enableDht,
                    relayUrl: normalizedRelayUrl,
                    identityPath: identityPath,
                    n0Discovery: n0Discovery,
                    localDiscovery: localDiscovery,
                    dataDir: dataDir,
                    gossipBootstrapPeers: bootstrapPeers
                )

                state = .running
                appendLog(.success, "Runtime started")
                appendLog(.network, "Local discovery: \(localDiscovery ? "enabled" : "disabled")")
                if !bootstrapPeers.isEmpty {
                    appendLog(.network, "Bootstrap peers: \(bootstrapPeers.map { String($0.prefix(8)) })")
                } else {
                    appendLog(.network, "Bootstrap peers: none (organic discovery)")
                }

                if let relayUrl = self.normalizedRelayUrl {
                    for peerId in bootstrapPeers {
                        await self.seedPeerRoute(nodeId: peerId, relayUrl: relayUrl, source: "bootstrap")
                    }
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
        autoMessagedPeerIds.removeAll()
        autoMessageAttemptedAt.removeAll()
        seededPeerIds.removeAll()
        stopAntiSleep()

        appendLog(.info, "Stopping node...")

        Task {
            await node.stop()
            state = .stopped
            nodeId = ""
            peersCount = 0
            groupsCount = 0
            appendLog(.info, "Node stopped. Echo count: \(echoCount)")
            stopNetworkLogExport()
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
                totalMessagesCount += 1
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
        stopNetworkLogExport()

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

    func scheduleInitialAutoStart() {
        guard !hasScheduledInitialAutoStart else { return }
        hasScheduledInitialAutoStart = true

        appendLog(.info, "Auto-start scheduled in 5 seconds")

        autoStartTask = Task { [weak self] in
            try? await Task.sleep(nanoseconds: Self.autoStartDelayNanoseconds)
            guard let self = self, !Task.isCancelled else { return }
            self.autoStartTask = nil

            guard self.state == .stopped || self.state == .error else { return }

            self.appendLog(.info, "Auto-start trigger fired")
            self.start()
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
                    self.totalMessagesCount += 1
                    let senderShort = String(msg.from.prefix(8))
                    let textPreview = String(msg.text.prefix(80))
                    self.appendLog(.network, "MSG from \(senderShort): \(textPreview)")

                    // Auto-echo: reply to incoming messages for stress testing
                    // Format matches tom-stress responder exactly:
                    //   PING:<seq> → PONG:<seq>
                    //   BURST:<seq> → BURST-ACK:<seq>
                    //   other → ECHO:<text>
                    if self.autoEchoEnabled {
                        do {
                            let reply = Self.buildStressReply(msg.text)
                            let replyData = reply.data(using: .utf8) ?? Data()
                            try await self.node.sendMessage(to: msg.from, payload: replyData)
                            self.echoCount += 1
                            if self.echoCount <= 10 || self.echoCount % 100 == 0 {
                                self.appendLog(.echo, "ECHO #\(self.echoCount) → \(senderShort): \(reply.prefix(40))")
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
                await self.maybeAutoMessageDiscoveredPeers(currentDiscovered)

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

    private func maybeAutoMessageDiscoveredPeers(_ peers: [TomPeer]) async {
        guard state == .running else { return }

        let now = Date()

        for peer in peers {
            guard peer.nodeId != nodeId else { continue }
            guard !autoMessagedPeerIds.contains(peer.nodeId) else { continue }

            let lastAttempt = autoMessageAttemptedAt[peer.nodeId] ?? .distantPast
            guard now.timeIntervalSince(lastAttempt) >= Self.autoMessageRetryInterval else { continue }

            autoMessageAttemptedAt[peer.nodeId] = now

            let targetLabel = peer.username.isEmpty ? peer.shortId : peer.username
            let probe = Self.buildAutoProbeMessage(username: username)

            do {
                if let relayUrl = normalizedRelayUrl {
                    await seedPeerRoute(nodeId: peer.nodeId, relayUrl: relayUrl, source: "auto-discovery")
                }
                let payload = Data(probe.utf8)
                try await node.sendMessage(to: peer.nodeId, payload: payload)
                autoMessagedPeerIds.insert(peer.nodeId)
                appendLog(.network, "AUTO-PING → \(targetLabel): \(probe)")

                let sent = TomMessage(
                    id: UUID().uuidString,
                    from: nodeId,
                    payload: payload.base64EncodedString(),
                    timestamp: UInt64(Date().timeIntervalSince1970 * 1000),
                    signatureValid: true,
                    wasEncrypted: true
                )
                messages.append(sent)
                totalMessagesCount += 1
            } catch {
                appendLog(.warning, "AUTO-PING failed → \(targetLabel): \(error.localizedDescription)")
            }
        }
    }

    private func seedPeerRoute(nodeId: NodeId, relayUrl: String, source: String) async {
        guard !nodeId.isEmpty else { return }
        guard nodeId != self.nodeId else { return }
        guard !seededPeerIds.contains(nodeId) else { return }

        do {
            try await node.addPeerAddr(nodeId: nodeId, relayUrl: relayUrl)
            seededPeerIds.insert(nodeId)
            appendLog(.network, "SEEDED ROUTE → \(String(nodeId.prefix(8))) via relay (\(source))")
        } catch {
            appendLog(.warning, "SEED ROUTE failed → \(String(nodeId.prefix(8))): \(error.localizedDescription)")
        }
    }

    // MARK: - Stress Reply (matches tom-stress responder exactly)

    static func buildStressReply(_ text: String) -> String {
        if text.hasPrefix("PING:") {
            return "PONG:" + text.dropFirst(5)
        } else if text.hasPrefix("BURST:") {
            return "BURST-ACK:" + text.dropFirst(6)
        } else {
            return "ECHO:" + text
        }
    }

    static func buildAutoProbeMessage(username: String) -> String {
        let sanitized = username
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .replacingOccurrences(of: " ", with: "-")
        let suffix = UInt64(Date().timeIntervalSince1970)
        return "PING:\(sanitized.isEmpty ? "appletv" : sanitized)-\(suffix)"
    }

    // MARK: - Network Log Export (UDP to Mac)

    private var udpLogSocket: Int32 = -1
    private var udpLogAddr: sockaddr_in?

    private func startNetworkLogExportIfNeeded() {
        guard udpLogExportEnabled else { return }

        let host = udpLogHost.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !host.isEmpty else {
            appendLog(.warning, "UDP log export skipped: host is empty")
            return
        }

        guard let port = UInt16(udpLogPort), port > 0 else {
            appendLog(.warning, "UDP log export skipped: invalid port '\(udpLogPort)'")
            return
        }

        startNetworkLogExport(host: host, port: port)
    }

    /// Start broadcasting logs over UDP for remote monitoring.
    private func startNetworkLogExport(host: String, port: UInt16) {
        udpLogSocket = socket(AF_INET, SOCK_DGRAM, IPPROTO_UDP)
        guard udpLogSocket >= 0 else {
            appendLog(.warning, "UDP log socket failed")
            return
        }

        // Enable broadcast so 255.255.255.255 works
        var broadcastEnable: Int32 = 1
        setsockopt(udpLogSocket, SOL_SOCKET, SO_BROADCAST, &broadcastEnable, socklen_t(MemoryLayout<Int32>.size))

        var addr = sockaddr_in()
        addr.sin_family = sa_family_t(AF_INET)
        addr.sin_port = port.bigEndian

        let parseResult = host.withCString { hostPtr in
            inet_pton(AF_INET, hostPtr, &addr.sin_addr)
        }
        guard parseResult == 1 else {
            appendLog(.warning, "UDP log export skipped: invalid IPv4 host \(host)")
            close(udpLogSocket)
            udpLogSocket = -1
            udpLogAddr = nil
            return
        }

        udpLogAddr = addr

        appendLog(.info, "UDP log export → \(host):\(port)")
    }

    private func stopNetworkLogExport() {
        if udpLogSocket >= 0 {
            close(udpLogSocket)
            udpLogSocket = -1
        }
        udpLogAddr = nil
    }

    /// Send a JSON log line over UDP (fire-and-forget)
    private func sendLogUDP(_ message: String) {
        guard udpLogSocket >= 0, var addr = udpLogAddr else { return }
        let line = "\(message)\n"
        line.withCString { ptr in
            withUnsafePointer(to: &addr) { addrPtr in
                addrPtr.withMemoryRebound(to: sockaddr.self, capacity: 1) { sa in
                    sendto(udpLogSocket, ptr, strlen(ptr), 0, sa, socklen_t(MemoryLayout<sockaddr_in>.size))
                }
            }
        }
    }
}
