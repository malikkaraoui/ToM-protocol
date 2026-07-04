import Foundation
import TomProtocolKit
import os.log
#if !os(macOS)
import UIKit
import AVFoundation
#endif
import Combine

@MainActor
final class TomNodeService: ObservableObject {
    static let shared = TomNodeService()

    private static let autoStartDelayNanoseconds: UInt64 = 5_000_000_000
    private static let autoMessageRetryInterval: TimeInterval = 5

    private let log = Logger(subsystem: "org.tom-protocol.tom-node", category: "TomNodeService")
    private let node = TomNodeWrapper()
    private var pollTask: Task<Void, Never>?
    private var startTask: Task<Void, Never>?
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
    @Published var localRole: String = "Peer"
    @Published var pathKind: String = "RELAY"
    @Published var pathRttMs: UInt64 = 0

    // Live log
    @Published var logEntries: [LogEntry] = []

    // Auto-echo for stress testing
    @Published var autoEchoEnabled: Bool = true
    @Published var echoCount: Int = 0

    // Anti-sleep audio player (iOS/tvOS only)
    #if !os(macOS)
    private var silentPlayer: AVAudioPlayer?
    /// Cached silent WAV so the keepalive can be rebuilt after an interruption.
    private var antiSleepWav: Data?
    /// Guard so audio-session observers are installed only once.
    private var audioObserversInstalled = false
    #endif

    // Relay vide = full-node mode, relay embarqué dans chaque nœud.
    @Published var relayUrl: String = ""
    /// Best relay known at runtime: configured relay first, then gossip-discovered.
    /// Updated after start and on each poll cycle. Use as relay when seeding peer routes.
    @Published var activeRelayUrl: String = ""
    @Published var username: String = TomNodeService.defaultUsername()
    @Published var encryption: Bool = true
    @Published var enableDht: Bool = true
    @Published var n0Discovery: Bool = true
    @Published var udpLogExportEnabled: Bool = true
    @Published var udpLogHost: String = TomNodeService.defaultCollectorHost()
    @Published var udpLogPort: String = "9999"

    private static func defaultUsername() -> String {
        #if os(iOS)
        return UIDevice.current.userInterfaceIdiom == .pad ? "iPad" : "iPhone"
        #elseif os(macOS)
        return "Mac"
        #else
        return "AppleTV"
        #endif
    }

    static let appareil: String = {
        #if os(iOS)
        return UIDevice.current.userInterfaceIdiom == .pad ? "ipad" : "iphone"
        #elseif os(macOS)
        return "macos"
        #else
        return "tvos"
        #endif
    }()

    /// Timestamp when the node entered .running state (nil when stopped).
    private var nodeStartTime: Date?
    /// Count of messages sent by this node (used in structured logs).
    private var totalMessagesSentCount: Int = 0
    /// First bootstrap source observed — persisted across all subsequent log lines.
    private var firstBootstrapSource: String? = nil

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
    private static let cachedPeerKey = "tom_bg_peers"

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
        if let configured = normalizedRelayUrl { return configured }
        if !activeRelayUrl.isEmpty { return "\(activeRelayUrl) (auto)" }
        return "Automatic discovery / public fallback"
    }

    var bootstrapStatusLabel: String {
        normalizedBootstrapPeers.isEmpty ? "Organic discovery only" : String(normalizedBootstrapPeers[0].prefix(12)) + "…"
    }

    // MARK: - Logging

    func appendLog(_ level: LogLevel, _ message: String, sourceAmorcage: String? = nil) {
        let entry = LogEntry(date: Date(), level: level, message: message)
        logEntries.append(entry)
        // Keep last 1000 entries
        if logEntries.count > 1000 {
            logEntries.removeFirst(logEntries.count - 1000)
        }
        // Broadcast structured JSON over UDP for remote monitoring
        let escaped = message.replacingOccurrences(of: "\"", with: "\\\"")
            .replacingOccurrences(of: "\n", with: " ")
        let uptimeS = nodeStartTime.map { Int(-$0.timeIntervalSinceNow) } ?? 0
        let msgsSent = totalMessagesSentCount
        let msgsRecv = totalMessagesCount - totalMessagesSentCount
        let shortId = String(nodeId.prefix(8))
        let effectiveSource = sourceAmorcage ?? firstBootstrapSource
        let srcField = effectiveSource.map { ",\"source_amorcage\":\"\($0)\"" } ?? ""
        let json = """
        {"ts":\(Int(Date().timeIntervalSince1970 * 1000)),"node":"\(username)","node_id":"\(shortId)","appareil":"\(Self.appareil)","event":"\(level)","detail":"\(escaped)"\(srcField),"phase":"\(state == .running ? "connecte" : "arret")","taille_reseau":\(peersCount),"number_peers":\(connectedPeers.count),"discovered_peers":\(discoveredPeers.count),"role":"\(localRole)","path":"\(pathKind)","rtt_ms":\(pathRttMs),"msgs_sent":\(msgsSent),"msgs_recv":\(msgsRecv),"groups":\(groupsCount),"uptime_s":\(uptimeS)}
        """
        sendLogUDP(json.trimmingCharacters(in: .whitespacesAndNewlines))
    }

    // MARK: - Anti-sleep

    func startAntiSleep() {
        #if !os(macOS)
        UIApplication.shared.isIdleTimerDisabled = true

        // Silent audio loop prevents tvOS/iOS from sleeping
        let silenceData = Data(count: 44100 * 2)
        var wavHeader = Data()
        let dataSize = UInt32(silenceData.count)
        let fileSize = UInt32(36 + silenceData.count)
        wavHeader.append(contentsOf: [0x52, 0x49, 0x46, 0x46]) // RIFF
        wavHeader.append(contentsOf: withUnsafeBytes(of: fileSize.littleEndian) { Array($0) })
        wavHeader.append(contentsOf: [0x57, 0x41, 0x56, 0x45]) // WAVE
        wavHeader.append(contentsOf: [0x66, 0x6D, 0x74, 0x20]) // fmt
        wavHeader.append(contentsOf: withUnsafeBytes(of: UInt32(16).littleEndian) { Array($0) })
        wavHeader.append(contentsOf: withUnsafeBytes(of: UInt16(1).littleEndian) { Array($0) })
        wavHeader.append(contentsOf: withUnsafeBytes(of: UInt16(1).littleEndian) { Array($0) })
        wavHeader.append(contentsOf: withUnsafeBytes(of: UInt32(44100).littleEndian) { Array($0) })
        wavHeader.append(contentsOf: withUnsafeBytes(of: UInt32(88200).littleEndian) { Array($0) })
        wavHeader.append(contentsOf: withUnsafeBytes(of: UInt16(2).littleEndian) { Array($0) })
        wavHeader.append(contentsOf: withUnsafeBytes(of: UInt16(16).littleEndian) { Array($0) })
        wavHeader.append(contentsOf: [0x64, 0x61, 0x74, 0x61]) // data
        wavHeader.append(contentsOf: withUnsafeBytes(of: dataSize.littleEndian) { Array($0) })
        wavHeader.append(silenceData)

        antiSleepWav = wavHeader
        activateSilentAudio()
        installAudioObserversIfNeeded()
        #else
        appendLog(.info, "Anti-sleep: not required on macOS")
        #endif
    }

    #if !os(macOS)
    /// (Re)activate the silent-audio keepalive. Safe to call repeatedly — used
    /// both at startup and to RESUME after an audio-session interruption.
    private func activateSilentAudio() {
        guard let wav = antiSleepWav else { return }
        do {
            // .mixWithOthers: coexist with any other audio so we are far less
            // likely to be stopped, and can resume cleanly after interruptions.
            try AVAudioSession.sharedInstance().setCategory(.playback, mode: .default, options: [.mixWithOthers])
            try AVAudioSession.sharedInstance().setActive(true)
            if silentPlayer == nil {
                silentPlayer = try AVAudioPlayer(data: wav)
            }
            silentPlayer?.numberOfLoops = -1
            silentPlayer?.volume = 0.01
            if silentPlayer?.isPlaying != true {
                silentPlayer?.play()
            }
        } catch {
            appendLog(.warning, "Anti-sleep activate failed: \(error.localizedDescription)")
        }
    }

    /// Without this, ANY audio interruption (call, Siri, another app, the system
    /// reclaiming the session) silently stops the keepalive — iOS then suspends
    /// the whole node within ~30s and it does nothing until the next foreground.
    /// These observers resume the keepalive so the node survives in background.
    private func installAudioObserversIfNeeded() {
        guard !audioObserversInstalled else { return }
        audioObserversInstalled = true
        let nc = NotificationCenter.default

        nc.addObserver(
            forName: AVAudioSession.interruptionNotification,
            object: nil,
            queue: .main
        ) { [weak self] note in
            guard let self,
                  let raw = note.userInfo?[AVAudioSessionInterruptionTypeKey] as? UInt,
                  let type = AVAudioSession.InterruptionType(rawValue: raw)
            else { return }
            if type == .ended {
                self.appendLog(.info, "Anti-sleep: interruption ended — resuming keepalive")
                self.activateSilentAudio()
            }
        }

        nc.addObserver(
            forName: AVAudioSession.mediaServicesWereResetNotification,
            object: nil,
            queue: .main
        ) { [weak self] _ in
            guard let self else { return }
            self.appendLog(.warning, "Anti-sleep: media services reset — rebuilding keepalive")
            self.silentPlayer = nil
            self.activateSilentAudio()
        }
    }
    #endif

    func stopAntiSleep() {
        #if !os(macOS)
        UIApplication.shared.isIdleTimerDisabled = false
        silentPlayer?.stop()
        silentPlayer = nil
        try? AVAudioSession.sharedInstance().setActive(false)
        #endif
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

        startTask = Task {
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

                // Stop demandé pendant le create → libérer le handle et sortir.
                // Sans ce point de sortie, un Stop pendant "Starting…" était
                // silencieusement ignoré (guard .running) et l'UI moulinait.
                if Task.isCancelled {
                    await node.forceReset()
                    appendLog(.info, "Démarrage annulé par Stop (create)")
                    startTask = nil
                    return
                }

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

                if Task.isCancelled {
                    await node.forceReset()
                    appendLog(.info, "Démarrage annulé par Stop (start)")
                    startTask = nil
                    return
                }

                state = .running
                nodeStartTime = Date()

                // Resolve best relay: configured first, then gossip-discovered
                let discoveredRelay = await self.node.getDiscoveredRelay()
                let effectiveRelay = self.normalizedRelayUrl ?? discoveredRelay
                if let relay = effectiveRelay { self.activeRelayUrl = relay }

                appendLog(.success, "Runtime started")
                appendLog(.network, "Local discovery: \(localDiscovery ? "enabled" : "disabled")")
                appendLog(.network, "Active relay: \(effectiveRelay ?? "none (organic)")")
                if !bootstrapPeers.isEmpty {
                    appendLog(.network, "Bootstrap peers: \(bootstrapPeers.map { String($0.prefix(8)) })")
                } else {
                    appendLog(.network, "Bootstrap peers: none (organic discovery)")
                }

                if let relayUrl = effectiveRelay {
                    for peerId in bootstrapPeers {
                        await self.seedPeerRoute(nodeId: peerId, relayUrl: relayUrl, source: "bootstrap")
                    }
                    // Reconnect vers les peers connus avant la mise en background
                    let cachedIds = UserDefaults.standard.stringArray(forKey: Self.cachedPeerKey) ?? []
                    let freshIds = Set(cachedIds).subtracting(Set(bootstrapPeers))
                    if !freshIds.isEmpty {
                        appendLog(.network, "BG-cache: seed \(freshIds.count) peer(s) connu(s)")
                        for peerId in freshIds {
                            await self.seedPeerRoute(nodeId: peerId, relayUrl: relayUrl, source: "bg-cache")
                        }
                    }
                }

                startAntiSleep()
                startPolling()
                startStatusServer()
                log.info("Node started — identity: \(self.identityPath ?? "ephemeral"), data: \(self.dataDir ?? "none")")

            } catch {
                log.error("Failed to start node: \(error.localizedDescription)")
                appendLog(.error, "START FAILED: \(error.localizedDescription)")
                state = .error
                errorMessage = error.localizedDescription
            }
            startTask = nil
        }
    }

    func stop() {
        // .starting inclus : un Stop pendant un démarrage lent doit TOUJOURS
        // être obéi. Avant, le guard n'acceptait que .running — un tap sur
        // Stop pendant "Starting…" était ignoré en silence et l'UI moulinait.
        guard state == .running || state == .starting else { return }
        let wasStarting = (state == .starting)
        startTask?.cancel()
        startTask = nil
        // Flip UI state IMMEDIATELY — never leave the user stuck on "Stopping".
        // The node teardown is I/O (QUIC/DHT close) and must not gate the UI.
        state = .stopped
        pollTask?.cancel()
        pollTask = nil
        statusServer?.stop()
        statusServer = nil
        autoMessagedPeerIds.removeAll()
        autoMessageAttemptedAt.removeAll()
        seededPeerIds.removeAll()
        firstBootstrapSource = nil
        stopAntiSleep()
        stopNetworkLogExport()
        nodeId = ""
        peersCount = 0
        groupsCount = 0
        nodeStartTime = nil
        totalMessagesSentCount = 0
        appendLog(.info, "Node stopped. Echo count: \(echoCount)")
        log.info("Node stopped")

        // Tear down the runtime in the background; the UI is already updated.
        // Si un start était en vol, c'est SA tâche (coopérative, annulée) qui
        // libère le handle via forceReset — ne pas déclencher un stop concurrent.
        if !wasStarting {
            Task { await node.stop() }
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
                totalMessagesSentCount += 1
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

    /// Called when the app enters background — persist state for fast reconnect.
    func handleEnterBackground() {
        wasRunningBeforeSleep = (state == .running)
        let ids = discoveredPeers.map { $0.nodeId }
        UserDefaults.standard.set(ids, forKey: Self.cachedPeerKey)
        appendLog(.info, "BG: \(ids.count) peer(s) mis en cache, wasRunning=\(wasRunningBeforeSleep)")
    }

    /// Called when the app returns to foreground.
    /// QUIC connections die only when iOS *actually suspends* the app — i.e. after
    /// a genuine `.background` transition (handleEnterBackground sets the flag).
    /// Transient `.inactive` → `.active` blips (notification/control center, app
    /// switcher peek, banners) do NOT suspend us: connections are still alive, so
    /// restarting then would needlessly free + rebuild the whole node in a loop.
    /// Therefore restart ONLY after a real background. Keep history + seeded peers.
    func handleReturnToForeground() {
        guard wasRunningBeforeSleep else { return }
        wasRunningBeforeSleep = false

        // Un démarrage est déjà en vol — ne pas empiler un second start
        // (forcer .stopped ici pendant un .starting créait deux séquences
        // create/start concurrentes → alreadyRunning / état incohérent).
        guard state != .starting else {
            appendLog(.info, "FOREGROUND: démarrage déjà en cours, pas de restart")
            return
        }

        appendLog(.warning, "FOREGROUND RETURN — restarting node (connexions QUIC perdues)")
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

            // Auto-restart — les peers du cache seront seeded dans start()
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

                    // Auto-echo: reply to incoming messages
                    // Reply with a fixed-size response (no growing chain)
                    if self.autoEchoEnabled, !msg.text.hasPrefix("recu 5/5") {
                        // Répondre à tout message SAUF aux échos eux-mêmes :
                        // un message manuel mérite son accusé de réception
                        // (feedback UX), mais un écho ne déclenche jamais
                        // d'écho — sinon deux nœuds s'entre-répondent à
                        // l'infini (tempête constatée en campagne, 100% CPU).
                        do {
                            // Fixed reply — never forward the original text (prevents ECHO:ECHO:... growth)
                            let reply = "recu 5/5 (msg #\(self.totalMessagesCount))"
                            let replyData = reply.data(using: .utf8) ?? Data()
                            try await self.node.sendMessage(to: msg.from, payload: replyData)
                            self.echoCount += 1
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
                    if let role = status.localRole { self.localRole = role }
                    if let pk = status.pathKind { self.pathKind = pk }
                    if let rtt = status.pathRttMs { self.pathRttMs = rtt }
                }
                // Keep activeRelayUrl in sync — prefer configured, fallback to discovered
                if self.normalizedRelayUrl == nil,
                   let discovered = await self.node.getDiscoveredRelay(),
                   !discovered.isEmpty {
                    self.activeRelayUrl = discovered
                }
                let currentConnected = await self.node.connectedPeers()
                self.connectedPeers = currentConnected

                let currentDiscovered = await self.node.discoveredPeers()
                // Log new peer discoveries
                for peer in currentDiscovered {
                    if !knownPeerIds.contains(peer.nodeId) {
                        knownPeerIds.insert(peer.nodeId)
                        let name = peer.username.isEmpty ? peer.shortId : "\(peer.username) (\(peer.shortId))"
                        let src = peer.source.lowercased()
                        if self.firstBootstrapSource == nil { self.firstBootstrapSource = src }
                        self.appendLog(.success, "PEER DISCOVERED: \(name) via \(peer.source)", sourceAmorcage: src)
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
                let effectiveRelay = normalizedRelayUrl ?? (activeRelayUrl.isEmpty ? nil : activeRelayUrl)
                if let relayUrl = effectiveRelay {
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
                totalMessagesSentCount += 1
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
    private var statusServer: StatusServer?

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

    // MARK: - Status Server (dev dashboard)

    private func startStatusServer() {
        statusServer?.stop()
        statusServer = StatusServer { [weak self] in
            await MainActor.run { self?.buildStatusJSON() ?? "{}" }
        }
        statusServer?.start()
        let ip = Self.getLocalIPv4() ?? "127.0.0.1"
        appendLog(.info, "Status server: http://\(ip):\(StatusServer.defaultPort)/")
    }

    @MainActor
    func buildStatusJSON() -> String {
        let phase: String
        if state == .running {
            phase = peersCount >= 2 ? "Converged" : "RelayAssist"
        } else {
            phase = state.rawValue
        }
        let uptimeSec = nodeStartTime.map { Int(-$0.timeIntervalSinceNow) } ?? 0
        let sentCount = totalMessagesSentCount
        let recvCount = max(0, totalMessagesCount - totalMessagesSentCount)
        let dict: [String: Any] = [
            "schema_version": 1,
            "node": username,
            "node_id": nodeId,
            "platform": Self.appareil,
            "relay_url_active": activeRelayUrl,
            "phase": phase,
            "taille_reseau": peersCount,
            "role": localRole,
            "relayeurs": connectedPeers.count,
            "pairs_connectes": connectedPeers,
            "groupes": [[String: Any]](),
            "messages_envoyes": sentCount,
            "messages_recus": recvCount,
            "messages_echoues": 0,
            "uptime_secondes": uptimeSec
        ]
        guard let data = try? JSONSerialization.data(withJSONObject: dict, options: [.sortedKeys]),
              let str = String(data: data, encoding: .utf8) else { return "{}" }
        return str
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
