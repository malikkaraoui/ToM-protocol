import Foundation
import TomProtocolKit
import os.log
#if !os(macOS)
import UIKit
import AVFoundation
#endif
import Combine
import Network

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
    // Watchdog anti-blocage : `.starting` est autrement un état PIÈGE — ni le
    // retour foreground (skip si .starting) ni la relance réseau (exige
    // .running/.error) n'en sortent. Si `node.start()` hang pendant un handoff
    // Wi-Fi↔cellulaire, le nœud restait « STARTING / Nœud inactif » à vie
    // (observé terrain, iPhone build 70). Ce timer force reset+retry.
    private var startWatchdogTask: Task<Void, Never>?
    private static let startWatchdogTimeout: TimeInterval = 30
    /// Horodatage d'entrée en `.starting` — permet au retour foreground de
    /// détecter un démarrage figé et de le récupérer sans attendre le watchdog.
    private var startingSince: Date?
    private static let stuckStartForegroundThreshold: TimeInterval = 8
    private var probeTask: Task<Void, Never>?
    private var probeSeq: Int = 0
    private var probeExpirationTimes: [String: Date] = [:]
    private var hasScheduledInitialAutoStart = false

    // Observateur de connectivité réseau (Wi-Fi / cellulaire / Ethernet).
    // iOS ne change PAS le scenePhase quand seule la data bascule (ex. mode
    // avion ON→OFF) : sans ce moniteur, un nœud .running dont les connexions
    // QUIC sont mortes pendant la coupure restait figé sur « déconnexion »
    // et ne relançait jamais la découverte. On observe donc le réseau
    // directement et on relance comme au retour de background.
    private let netMonitor = NWPathMonitor()
    private let netMonitorQueue = DispatchQueue(label: "org.tom-protocol.netmonitor")
    private var netMonitorStarted = false
    private var lastPathSatisfied = true
    /// Interface active courante (Wi-Fi / cellulaire / Ethernet). Un changement
    /// pendant que le réseau reste « satisfait » (handoff Wi-Fi→cellulaire en
    /// sortant de la maison) invalide silencieusement les sockets QUIC : on
    /// relance aussi dans ce cas, pas seulement sur perte/retour.
    private var lastPrimaryInterface: NWInterface.InterfaceType?
    /// Un lien LAN (WiFi/Ethernet) est-il disponible ? Sert à décider si le
    /// broadcast UDP des logs peut atteindre le collecteur (true) ou doit être
    /// bufferisé (false = cellulaire pur). Vrai par défaut (ne pas bufferiser
    /// à froid avant le premier callback du moniteur réseau).
    private var lanInterfaceAvailable = true
    /// Anti-tempête : intervalle mini entre deux relances déclenchées réseau
    /// (réseau qui flappe en train/ascenseur, ou .error qui se répète).
    private var lastNetworkRestartAt: Date?
    private static let networkRestartThrottle: TimeInterval = 3

    private var autoMessagedPeerIds = Set<String>()
    private var autoMessageAttemptedAt: [String: Date] = [:]
    private var seededPeerIds = Set<String>()

    // Mapping id→nom découvert, persisté en UserDefaults (anti-reset tvOS)
    private var knownNames: [String: String] = [:]
    private static let knownNamesKey = "tom_known_names"
    private var lastSaveTime: Date = Date()

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
    @Published var pathAddr: String?
    /// Per-peer path info (node_id → kind, rtt_ms, addr)
    @Published var pathsByPeer: [String: PathInfo] = [:]
    // Écart d'horloge médian vs le réseau (ms) — nil tant que < 5 échantillons.
    // Signal honnête : inclut la latence réseau (pas une synchro NTP).
    @Published var clockSkewMs: Int64?
    @Published var clockSkewSamples: UInt64 = 0
    // Application build number (from status)
    @Published var appBuild: UInt32 = 0

    // Activity log (events from runtime) — cappé à 100 entrées FIFO
    @Published var activityLog: [ActivityEntry] = []

    // Security alerts — Phase 2 Sprint 3 (indicateur ATTAQUE)
    @Published var securityAlerts: Int = 0
    @Published var lastSecurityEventAt: Date?
    @Published var lastSecurityEventText: String = ""

    // Live log
    @Published var logEntries: [LogEntry] = []

    // Auto-echo for stress testing
    @Published var autoEchoEnabled: Bool = true
    @Published var echoCount: Int = 0

    // Mode sonde : petit message texte (~40 octets) envoyé toutes les 30s à
    // chaque pair connecté. Permet de valider que le réseau reste vivant +
    // de tester le backup (quand une app tue, les sondes sont backupées par
    // un relais, et l'utilisatrice peut vérifier la purge après 5 min).
    // ACTIVÉ par défaut pour pré-livraison — pas de charge réseau importante.
    @Published var probeEnabled: Bool = true
    private static let probeIntervalNanoseconds: UInt64 = 30_000_000_000 // 30s
    private static let probeTtlMinutes: TimeInterval = 5 * 60 // 5 minutes

    static func fmtTaille(_ n: Int) -> String {
        if n >= 1_000_000 { return String(format: "%.1f Mo", Double(n) / 1_000_000) }
        if n >= 1_000 { return "\(n / 1_000) Ko" }
        return "\(n) o"
    }

    static func shortDeviceName(_ fullName: String) -> String {
        if fullName.contains("iPad") { return "iPad" }
        if fullName.contains("iPhone") { return "iPhone" }
        if fullName.contains("Mac") { return "Mac" }
        if fullName.contains("Apple") || fullName.contains("TV") { return "Apple TV" }
        return fullName
    }

    // Anti-sleep audio player (iOS/tvOS only)
    #if !os(macOS)
    private var silentPlayer: AVAudioPlayer?
    /// Cached silent WAV so the keepalive can be rebuilt after an interruption.
    private var antiSleepWav: Data?
    /// Guard so audio-session observers are installed only once.
    private var audioObserversInstalled = false
    #else
    private var sleepAssertion: NSObjectProtocol?
    #endif

    // Relay vide = chaque nœud embarque son propre relay (full-node mode).
    // Mettre une URL ici seulement pour la phase genesis (<4 nœuds sans relay actif).
    @Published var relayUrl: String = ""
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
    /// Cumulative count of messages RECEIVED — monotone, independent of the
    /// message store (the probe TTL purge shrinks the store, and deriving
    /// recv from total-sent went negative once probes outnumbered receipts).
    private var totalMessagesReceivedCount: Int = 0
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

    /// Le FIL D'ACTIVITÉ ne montre QUE des événements pertinents pour un humain.
    /// Tout le reste (debug protocole brut type `RelayReadyReceived { NodeId(…) }`,
    /// churn d'état des pairs « Pair vieilli »/voisins gossip qui vit désormais
    /// dans la carte Pairs, attestations de présence à 15s, etc.) part au Live
    /// Log mais PAS dans l'activité. Liste blanche explicite = des règles claires.
    private static let activityFeedTypes: Set<String> = [
        // Sécurité / anti-spam
        "SenderThrottled", "MessageRejected", "GroupSecurityViolation",
        // Livraison
        "DeliveryTimeout", "DeliveryRetry",
        // Relais embarqué
        "EmbeddedRelayStarted", "EmbeddedRelayFailed",
        // Groupes
        "GroupCreated", "GroupJoined", "GroupMemberJoined", "GroupMemberLeft",
        // Chemin réseau (Direct ↔ Relais)
        "PathChanged",
        // Backup (messages différés)
        "BackupDelivered", "BackupExpired",
        // Mon propre rôle change
        "LocalRoleChanged",
    ]

    private init() {
        loadPersistentNames()
    }

    // MARK: - Device Name Mapping & Persistence

    /// Nom joli de CE device (pour le titre) : "Mac" → "MacBook Pro", etc.
    var localDisplayName: String {
        Self.prettifyNames[username] ?? username
    }

    /// Table de noms jolis : username brut → affichage utilisatrice
    private static let prettifyNames: [String: String] = [
        "nas": "Freebox",
        "Mac": "MacBook Pro",
        "iPad": "iPad",
        "iPhone": "iPhone",
        "AppleTV": "Apple TV"
    ]

    /// Charge le cache knownNames depuis UserDefaults (anti-reset tvOS)
    private func loadPersistentNames() {
        guard let jsonData = UserDefaults.standard.data(forKey: Self.knownNamesKey),
              let dict = try? JSONDecoder().decode([String: String].self, from: jsonData) else {
            knownNames = [:]
            return
        }
        knownNames = dict
    }

    /// Sauvegarde knownNames en UserDefaults (throttled : max 1×/5s)
    private func savePersistentNames() {
        let now = Date()
        guard now.timeIntervalSince(lastSaveTime) >= 5 else { return }
        lastSaveTime = now

        if let jsonData = try? JSONEncoder().encode(knownNames) {
            UserDefaults.standard.set(jsonData, forKey: Self.knownNamesKey)
        }
    }

    /// Charge les compteurs persistants depuis UserDefaults
    private func loadPersistentCounters() {
        let saved = UserDefaults.standard
        totalMessagesCount = saved.integer(forKey: "tom_total_messages_count")
        totalMessagesSentCount = saved.integer(forKey: "tom_total_messages_sent_count")
        totalMessagesReceivedCount = saved.integer(forKey: "tom_total_messages_received_count")
    }

    /// Sauvegarde les compteurs en UserDefaults (throttled : max 1×/50 messages)
    private func savePersistentCounters() {
        let saved = UserDefaults.standard
        saved.set(totalMessagesCount, forKey: "tom_total_messages_count")
        saved.set(totalMessagesSentCount, forKey: "tom_total_messages_sent_count")
        saved.set(totalMessagesReceivedCount, forKey: "tom_total_messages_received_count")
    }

    /// Retourne le nom affichable pour un nodeId :
    /// - Cherche dans le cache knownNames (id complet)
    /// - Sinon cherche dans discoveredPeers (id complet OU préfixe 8)
    /// - Applique le mapping joli (nas→Freebox, etc.)
    /// - Retour : "Freebox (01f9bff9…)" ou "01f9bff9…" si inconnu
    /// Format d'affichage : NOM d'abord, puis le début de l'ID unique.
    /// Ex : "Freebox · 11d5bb11" · "Apple TV · 75145c05". Deux iPhones se
    /// distinguent naturellement par leur début d'ID.
    private func formatName(_ pretty: String, _ nodeId: String) -> String {
        "\(pretty) · \(String(nodeId.prefix(8)))"
    }

    /// Résout le username brut d'un pair depuis le cache ou discoveredPeers.
    private func rawUsername(for nodeId: String) -> String? {
        if let peer = discoveredPeers.first(where: { $0.nodeId == nodeId
            || $0.nodeId.hasPrefix(nodeId.prefix(8)) }), !peer.username.isEmpty {
            return peer.username
        }
        return nil
    }

    func displayName(for nodeId: String) -> String {
        // Résout le nom courant (peut arriver APRÈS un premier affichage sans
        // nom — on ne fige donc pas un fallback dans le cache).
        if let raw = rawUsername(for: nodeId) {
            let pretty = Self.prettifyNames[raw] ?? raw
            let display = formatName(pretty, nodeId)
            if knownNames[nodeId] != display {
                knownNames[nodeId] = display
                savePersistentNames()
            }
            return display
        }
        // Nom déjà connu (persisté) même si le pair n'est plus dans discoveredPeers.
        if let cached = knownNames[nodeId] { return cached }
        // Fallback : début d'ID seul, jamais mémorisé (pour laisser le vrai nom arriver).
        return String(nodeId.prefix(8))
    }

    /// Build (version) propagé par un pair, 0 si inconnu. Pour l'affichage « Nom 69 ».
    func appBuild(for nodeId: String) -> Int {
        discoveredPeers.first(where: { $0.nodeId == nodeId
            || $0.nodeId.hasPrefix(nodeId.prefix(8)) })?.appBuild ?? 0
    }

    /// « Nom Build » (ex « iPad 69 »), le build accolé au nom comme un n° de série.
    /// Build omis si inconnu (0).
    func nameWithBuild(for nodeId: String) -> String {
        let name = displayName(for: nodeId)
        let b = appBuild(for: nodeId)
        return b > 0 ? "\(name) \(b)" : name
    }

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
        let msgsRecv = totalMessagesReceivedCount
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
        sleepAssertion = ProcessInfo.processInfo.beginActivity(
            options: [.idleSystemSleepDisabled, .suddenTerminationDisabled],
            reason: "ToM node must stay reachable")
        appendLog(.info, "Anti-sleep: system sleep disabled via ProcessInfo")
        #endif
    }

    #if !os(macOS)
    /// (Re)activate the silent-audio keepalive. Safe to call repeatedly — used
    /// both at startup and to RESUME after an audio-session interruption.
    private func activateSilentAudio() {
        guard let wav = antiSleepWav else { return }
        do {
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

    /// Resume the keepalive after any audio interruption (call, Siri, another
    /// app, the system reclaiming the session). Without this the keepalive stops
    /// and iOS suspends the whole node within ~30s — doing nothing until the next
    /// foreground.
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

        // Changement de route audio (sortie HDMI/AirPlay qui change) : iOS/tvOS
        // peut STOPPER le player SANS interruptionNotification. Sans ré-armement,
        // le keepalive meurt et le nœud est suspendu. Réf : Apple
        // AVAudioSession.routeChangeNotification.
        nc.addObserver(
            forName: AVAudioSession.routeChangeNotification,
            object: nil,
            queue: .main
        ) { [weak self] note in
            guard let self else { return }
            let reasonRaw = note.userInfo?[AVAudioSessionRouteChangeReasonKey] as? UInt ?? 0
            let reason = AVAudioSession.RouteChangeReason(rawValue: reasonRaw) ?? .unknown
            if self.silentPlayer?.isPlaying != true {
                self.appendLog(.info, "Anti-sleep: route change (\(reasonRaw)) a stoppé le keepalive — reprise")
                self.activateSilentAudio()
            } else if reason == .oldDeviceUnavailable || reason == .newDeviceAvailable {
                self.activateSilentAudio()
            }
        }
    }
    #endif

    func stopAntiSleep() {
        #if !os(macOS)
        UIApplication.shared.isIdleTimerDisabled = false
        silentPlayer?.stop()
        silentPlayer = nil
        try? AVAudioSession.sharedInstance().setActive(false)
        #else
        if let a = sleepAssertion {
            ProcessInfo.processInfo.endActivity(a)
            sleepAssertion = nil
        }
        appendLog(.info, "Anti-sleep: system sleep re-enabled")
        #endif
    }

    func start() {
        guard state == .stopped || state == .error else { return }
        state = .starting
        startingSince = Date()
        armStartWatchdog()
        errorMessage = nil
        echoCount = 0
        autoStartTask?.cancel()
        autoStartTask = nil
        autoMessagedPeerIds.removeAll()
        autoMessageAttemptedAt.removeAll()
        seededPeerIds.removeAll()
        stopNetworkLogExport()
        startNetworkLogExportIfNeeded()

        // COMPTEUR SESSION : remet à zéro à chaque démarrage (par décision UX)
        // Les compteurs persistants sont gardés pour d'autres usages, mais
        // l'affichage repartira toujours de 0.
        totalMessagesCount = 0
        totalMessagesSentCount = 0
        totalMessagesReceivedCount = 0

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
                    gossipBootstrapPeers: bootstrapPeers,
                    // L1-001 flotte PHASE 1 (plomberie) — voir
                    // docs/plans/L1-001-runbook-flotte.md : gate à 0.0
                    // (accepte toute attestation signée bien formée) +
                    // sonde auto 15s. PHASE 2 : passer le gate à nil
                    // (défaut protocole 2.0).
                    presenceContributionMin: 0.0,
                    presenceProbeIntervalSecs: 15,
                    appBuild: UInt32(TomVersion.build)
                )

                if Task.isCancelled {
                    await node.forceReset()
                    appendLog(.info, "Démarrage annulé par Stop (start)")
                    startTask = nil
                    return
                }

                state = .running
                cancelStartWatchdog()
                nodeStartTime = Date()
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
                startProbe()
                startStatusServer()
                log.info("Node started — identity: \(self.identityPath ?? "ephemeral"), data: \(self.dataDir ?? "none")")

            } catch {
                log.error("Failed to start node: \(error.localizedDescription)")
                appendLog(.error, "START FAILED: \(error.localizedDescription)")
                state = .error
                cancelStartWatchdog()
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
        cancelStartWatchdog()
        probeTask?.cancel()
        probeTask = nil
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
        // Phase 2 Sprint 3 : reset security alerts on stop
        securityAlerts = 0
        lastSecurityEventAt = nil
        lastSecurityEventText = ""
        savePersistentNames()
        savePersistentCounters()
        appendLog(.info, "Node stopped. Echo count: \(echoCount)")
        log.info("Node stopped")

        // Tear down the runtime in the background; the UI is already updated.
        // Si un start était en vol, c'est SA tâche (coopérative, annulée) qui
        // libère le handle via forceReset — ne pas déclencher un stop concurrent.
        if !wasStarting {
            Task { await node.stop() }
        }
    }

    // MARK: - Start watchdog (anti état-piège `.starting`)

    /// Arme (ou réarme) le watchdog. À échéance, si le nœud est TOUJOURS
    /// `.starting`, c'est que `node.start()` s'est bloqué (hang FFI, course
    /// avec un handoff réseau, task perdue) : on force un reset + retry, seule
    /// issue puisque ni foreground ni relance réseau ne récupèrent `.starting`.
    private func armStartWatchdog() {
        startWatchdogTask?.cancel()
        startWatchdogTask = Task { [weak self] in
            try? await Task.sleep(nanoseconds: UInt64(Self.startWatchdogTimeout * 1_000_000_000))
            guard !Task.isCancelled else { return }
            self?.recoverFromStuckStart()
        }
    }

    private func cancelStartWatchdog() {
        startWatchdogTask?.cancel()
        startWatchdogTask = nil
        startingSince = nil
    }

    /// Sort du `.starting` bloqué : reset complet puis relance propre.
    private func recoverFromStuckStart() {
        // Le start a pu aboutir juste avant l'échéance — ne rien casser.
        guard state == .starting else { return }
        appendLog(.error, "⏱️ START WATCHDOG — démarrage bloqué > \(Int(Self.startWatchdogTimeout))s, reset + retry")
        log.error("Start watchdog fired — node stuck in .starting, forcing reset + retry")
        startTask?.cancel()
        startTask = nil
        pollTask?.cancel()
        pollTask = nil
        stopAntiSleep()
        stopNetworkLogExport()
        Task {
            await node.forceReset()
            // Repasse par .stopped pour que start() (guard .stopped/.error) reparte.
            state = .stopped
            nodeId = ""
            peersCount = 0
            groupsCount = 0
            start()
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
        appendLog(.warning, "📱 BACKGROUND — app en arrière-plan")
        wasRunningBeforeSleep = (state == .running)
        let ids = discoveredPeers.map { $0.nodeId }
        UserDefaults.standard.set(ids, forKey: Self.cachedPeerKey)
        appendLog(.info, "BG: \(ids.count) peer(s) mis en cache, wasRunning=\(wasRunningBeforeSleep)")
    }

    /// Called when the app returns to foreground.
    /// QUIC connections die only when the OS *actually suspends* the app — i.e.
    /// after a genuine `.background` transition (handleEnterBackground sets the flag).
    /// Transient `.inactive` → `.active` blips (notification/control center, app
    /// switcher peek, banners) do NOT suspend us: connections are still alive, so
    /// restarting then would needlessly free + rebuild the whole node in a loop.
    /// Therefore restart ONLY after a real background. Keep history + seeded peers.
    func handleReturnToForeground() {
        appendLog(.warning, "📱 FOREGROUND — app au premier plan (wasRunning=\(wasRunningBeforeSleep))")

        // Démarrage figé (ex. handoff réseau ayant bloqué node.start() pendant
        // la suspension) : au retour à l'écran, récupérer immédiatement plutôt
        // que d'attendre le reliquat du watchdog. Couvre le cas terrain où le
        // nœud restait « STARTING / Nœud inactif » indéfiniment.
        if state == .starting,
           let since = startingSince,
           Date().timeIntervalSince(since) > Self.stuckStartForegroundThreshold {
            appendLog(.warning, "FOREGROUND: démarrage figé depuis \(Int(Date().timeIntervalSince(since)))s — recovery")
            wasRunningBeforeSleep = false
            recoverFromStuckStart()
            return
        }

        guard wasRunningBeforeSleep else { return }
        wasRunningBeforeSleep = false

        // Un démarrage RÉCENT est déjà en vol — ne pas empiler un second start
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

    /// Démarre l'observation de la connectivité réseau (une seule fois).
    /// Appelé au lancement de l'app : le moniteur reste actif en permanence
    /// pour réagir instantanément à un retour de réseau, même si aucune
    /// transition de scenePhase n'a lieu.
    func startNetworkMonitor() {
        guard !netMonitorStarted else { return }
        netMonitorStarted = true
        netMonitor.pathUpdateHandler = { [weak self] path in
            let satisfied = (path.status == .satisfied)
            // Interface effectivement utilisée par le chemin (pas la simple
            // liste des interfaces disponibles, qui fluctue) — signal fiable
            // d'un handoff Wi-Fi↔cellulaire.
            var primary: NWInterface.InterfaceType?
            for t in [NWInterface.InterfaceType.wifi, .wiredEthernet, .cellular, .other] where path.usesInterfaceType(t) {
                primary = t
                break
            }
            // Un lien LAN (WiFi/Ethernet) est-il disponible ? Le broadcast UDP
            // vers 192.168.0.255 part par l'interface du bon sous-réseau, donc
            // dès qu'un WiFi/Ethernet existe le collecteur LAN est joignable —
            // même si le cellulaire est la route « primaire ». On ne bufferise
            // QUE lorsqu'il n'y a aucun lien LAN (cellulaire pur).
            let hasLan = path.usesInterfaceType(.wifi) || path.usesInterfaceType(.wiredEthernet)
            Task { @MainActor [weak self] in
                self?.handleNetworkPathChange(satisfied: satisfied, primaryInterface: primary, hasLan: hasLan)
            }
        }
        netMonitor.start(queue: netMonitorQueue)
    }

    /// Réagit à un changement réseau rapporté par iOS. Deux déclencheurs de
    /// relance : (1) transition perte→retour (ex. sortie du mode avion) ;
    /// (2) changement d'interface active alors que le réseau reste satisfait
    /// (handoff Wi-Fi→cellulaire) — les sockets QUIC liés à l'ancienne
    /// interface deviennent muets sans que la disponibilité change.
    private func handleNetworkPathChange(satisfied: Bool, primaryInterface: NWInterface.InterfaceType?, hasLan: Bool) {
        let wasSatisfied = lastPathSatisfied
        let wasPrimary = lastPrimaryInterface
        let hadLan = lanInterfaceAvailable
        lastPathSatisfied = satisfied
        lastPrimaryInterface = primaryInterface
        lanInterfaceAvailable = hasLan

        // FLUSH D'ABORD (le socket UDP est encore ouvert). La reconnexion
        // ci-dessous appelle stopNetworkLogExport() qui FERME le socket : si on
        // flushait après, flushUDPLogBuffer trouverait le socket fermé et
        // abandonnerait → les logs cellulaires seraient perdus.
        if hasLan && !hadLan && !udpLogPendingBuffer.isEmpty {
            appendLog(.info, "📤 Flush de \(udpLogPendingBuffer.count) logs bufferisés (retour LAN)")
            flushUDPLogBuffer()
        }

        if satisfied && !wasSatisfied {
            appendLog(.warning, "🌐 RÉSEAU REVENU — relance immédiate de la découverte")
            log.info("Network reachable again — restarting discovery")
            reconnectAfterNetworkChange(reason: "retour réseau")
        } else if !satisfied && wasSatisfied {
            appendLog(.warning, "🌐 RÉSEAU PERDU — connexions QUIC mortes, attente du retour")
            log.info("Network lost — awaiting reconnection")
        } else if satisfied && wasSatisfied && wasPrimary != nil && primaryInterface != wasPrimary {
            // Handoff d'interface (ex. Wi-Fi → cellulaire en sortant de la maison).
            appendLog(.warning, "🌐 RÉSEAU CHANGE D'INTERFACE (\(wasPrimary.map(Self.ifaceLabel) ?? "?")→\(primaryInterface.map(Self.ifaceLabel) ?? "?")) — relance (ancien chemin invalidé)")
            log.info("Network interface changed — restarting discovery")
            reconnectAfterNetworkChange(reason: "changement d'interface")
        }
    }

    /// Flush tous les logs bufferisés en cellulaire et envoyer via UDP
    private func flushUDPLogBuffer() {
        guard udpLogSocket >= 0, var addr = udpLogAddr else {
            // Socket pas prêt, laisser les logs en buffer pour la prochaine fois
            return
        }

        for line in udpLogPendingBuffer {
            line.withCString { ptr in
                withUnsafePointer(to: &addr) { addrPtr in
                    addrPtr.withMemoryRebound(to: sockaddr.self, capacity: 1) { sa in
                        sendto(udpLogSocket, ptr, strlen(ptr), 0, sa, socklen_t(MemoryLayout<sockaddr_in>.size))
                    }
                }
            }
        }

        udpLogPendingBuffer.removeAll()
    }

    private static func ifaceLabel(_ t: NWInterface.InterfaceType) -> String {
        switch t {
        case .wifi: return "Wi-Fi"
        case .cellular: return "cellulaire"
        case .wiredEthernet: return "Ethernet"
        case .loopback: return "loopback"
        case .other: return "autre"
        @unknown default: return "?"
        }
    }

    /// Relance le nœud après un changement de connectivité — même séquence que
    /// le retour de foreground (les connexions QUIC ne survivent pas à une
    /// coupure/handoff réseau ; un reset complet garantit une découverte
    /// fraîche). N'agit que si le nœud était censé tourner (.running) ou est en
    /// échec (.error, ex. démarrage sans réseau) : un arrêt MANUEL (.stopped)
    /// n'est jamais outrepassé. Throttlé pour absorber un réseau qui flappe.
    private func reconnectAfterNetworkChange(reason: String) {
        guard state == .running || state == .error else {
            appendLog(.info, "RÉSEAU (\(reason)): nœud non actif (\(state)) — pas de relance auto")
            return
        }
        // Anti-tempête : ignore une relance si la précédente date de < 3s
        // (réseau qui flappe, ou .error qui se répète). Le restart en vol met
        // déjà state=.starting, cette garde borne en plus la fréquence.
        if let last = lastNetworkRestartAt, Date().timeIntervalSince(last) < Self.networkRestartThrottle {
            appendLog(.info, "RÉSEAU (\(reason)): relance ignorée (throttle < \(Int(Self.networkRestartThrottle))s)")
            return
        }
        lastNetworkRestartAt = Date()

        appendLog(.warning, "RÉSEAU RETOUR (\(reason)) — restart du nœud (connexions QUIC perdues)")
        // Bascule l'état de façon SYNCHRONE avant le Task async : sans ça, une
        // 2e transition réseau repasserait la garde pendant le `await
        // forceReset()` et lancerait un second create/start concurrent
        // (→ alreadyRunning / état incohérent).
        state = .starting
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
            start()
        }
    }

    func scheduleInitialAutoStart() {
        startNetworkMonitor()
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

    /// Boucle de sonde légère : toutes les 30s, un petit message texte (~40 octets)
    /// vers chaque pair connecté. Permet de valider la liveness du réseau et
    /// de tester le backup (quand l'app est tuée, les sondes sont backupées).
    /// TTL local : chaque sonde reçue/envoyée est purgée après ~5 min.
    private func startProbe() {
        probeTask?.cancel()
        probeTask = Task { [weak self] in
            while !Task.isCancelled {
                try? await Task.sleep(nanoseconds: Self.probeIntervalNanoseconds)
                guard let self = self else { break }
                guard self.probeEnabled, self.state == .running else { continue }

                let now = Date()
                self.probeSeq += 1
                let epochMs = Int64(now.timeIntervalSince1970 * 1000)
                let probeMsg = "probe \(self.probeSeq) \(epochMs)"

                for peer in self.connectedPeers {
                    let payload = Data(probeMsg.utf8)
                    do {
                        let uuid = UUID().uuidString
                        try await self.node.sendMessage(to: peer, payload: payload)
                        self.totalMessagesSentCount += 1
                        self.probeExpirationTimes[uuid] = now.addingTimeInterval(Self.probeTtlMinutes)
                        let displayName = self.displayName(for: peer)
                        self.appendLog(.network, "PROBE↑ seq=\(self.probeSeq) → \(displayName)")
                    } catch {
                        let displayName = self.displayName(for: peer)
                        self.appendLog(.error, "PROBE✗ seq=\(self.probeSeq) → \(displayName): \(error.localizedDescription)")
                    }
                }
            }
        }
    }

    private func startPolling() {
        var knownPeerIds = Set<String>()
        var lastPeerCount = 0
        var lastPresenceTotal: UInt64 = 0
        var lastSaveMessageCount = 0

        pollTask = Task { [weak self] in
            while !Task.isCancelled {
                guard let self = self else { break }

                // R13 : drainer les messages de groupe reçus (dont ceux rattrapés
                // hors-ligne via gap-fill) vers l'inbox lu par le serveur de contrôle.
                let newGroupMsgs = await self.node.receiveGroupMessages()
                if !newGroupMsgs.isEmpty {
                    self.groupInbox.append(contentsOf: newGroupMsgs)
                    if self.groupInbox.count > 500 {
                        self.groupInbox.removeFirst(self.groupInbox.count - 500)
                    }
                }

                // Poll messages
                let newMessages = await self.node.receiveMessages()
                for msg in newMessages {
                    let senderShort = String(msg.from.prefix(8))

                    // Taille APPROX sans décoder : `payload` est du base64 (4 chars
                    // → 3 octets). CRITIQUE : décoder `msg.text` (base64 → Data →
                    // String) d'un payload de 64 Mo coûte ~128 Mo d'alloc sur le
                    // MAIN ACTOR. Le faire plusieurs fois par message gelait l'UI →
                    // l'OS tuait l'app (watchdog / jetsam). L'Apple TV a peu de RAM,
                    // c'est encore plus critique. On ne décode QUE les petits messages.
                    let approxBytes = msg.payload.utf8.count / 4 * 3
                    let isLarge = approxBytes > 4096
                    let smallText: String? = isLarge ? nil : msg.text

                    let textPreview: String
                    let isProbe = smallText?.hasPrefix("probe ") ?? false
                    if let t = smallText, t.hasPrefix("CAMP:") || t.hasPrefix("CTRL:") {
                        let parts = t.split(separator: ":", maxSplits: 3)
                        let sz = parts.count > 1 ? (Int(parts[1]) ?? approxBytes) : approxBytes
                        let seq = parts.count > 2 ? String(parts[2]) : "?"
                        textPreview = "📦 \(Self.fmtTaille(sz)) #\(seq)"
                    } else if isProbe {
                        textPreview = smallText ?? "probe ?"
                    } else if isLarge {
                        textPreview = "📦 \(Self.fmtTaille(approxBytes))"
                    } else {
                        textPreview = String((smallText ?? "").prefix(80))
                    }

                    // TOUJOURS ne stocker qu'un APERÇU compact — jamais le payload
                    // brut dans la liste UI @Published (500 × 64 Mo = kill mémoire).
                    let display = TomMessage(
                        id: msg.id,
                        from: msg.from,
                        payload: Data(textPreview.utf8).base64EncodedString(),
                        timestamp: msg.timestamp,
                        signatureValid: msg.signatureValid,
                        wasEncrypted: msg.wasEncrypted
                    )
                    self.messages.append(display)
                    self.totalMessagesCount += 1
                    self.totalMessagesReceivedCount += 1

                    // Log et TTL pour les sondes
                    let senderDisplay = self.displayName(for: msg.from)
                    if isProbe {
                        self.probeExpirationTimes[msg.id] = Date().addingTimeInterval(Self.probeTtlMinutes)
                        self.appendLog(.network, "PROBE↓ from \(senderDisplay): \(textPreview)")
                    } else {
                        self.appendLog(.network, "MSG from \(senderDisplay): \(textPreview)")
                    }

                    // Auto-echo : réponse FIXE (accusé), y compris pour un gros
                    // message. Un écho est toujours petit → un gros message n'en
                    // est jamais un, pas besoin de décoder pour le savoir.
                    // ATTENTION : NE PAS echo les sondes (boucle infinie de trafic).
                    let isEcho = !isLarge && (smallText?.hasPrefix("recu 5/5") ?? false)
                    if self.autoEchoEnabled, !isEcho, !isProbe {
                        do {
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

                // Poll and integrate protocol events into activity log
                let newEvents = await self.node.receiveEvents()
                for event in newEvents {
                    // Texte avec NOMS d'appareils résolus (« Freebox · 11d5bb11 »
                    // au lieu de l'ID brut) — l'utilisatrice veut lire des noms.
                    let text = event.displayLine(resolvingNames: { self.displayName(for: $0) })

                    // Le Live Log reçoit TOUT (c'est là que vivent les logs bruts).
                    self.appendLog(.info, text)

                    // Bandeau « anomalie sécurité » réservé aux VRAIES violations
                    // (signature forgée, sécurité de groupe). L'antispam
                    // (`SenderThrottled`) est un CONTRÔLE DE FLUX progressif par
                    // décision LOCKED #5 (« the sprinkler gets sprinkled »), PAS
                    // une attaque : pacer un ami reconnecté n'est pas un incident.
                    // Le lever en rouge contredisait #5 et l'invisibilité (#6).
                    // Il reste visible au Live Log (ci-dessus), pas en alarme.
                    let isSecurityEvent = event.type.contains("Rejected")
                        || event.type.contains("GroupSecurity")
                    if isSecurityEvent {
                        self.securityAlerts += 1
                        self.lastSecurityEventAt = Date()
                        self.lastSecurityEventText = text
                    }

                    // Le FIL D'ACTIVITÉ ne montre QUE des événements humains
                    // pertinents (liste blanche). Le debug protocole brut et le
                    // churn d'état des pairs restent au Live Log, pas ici.
                    guard Self.activityFeedTypes.contains(event.type) else { continue }

                    // Anti-bruit : pas deux entrées identiques consécutives.
                    if let last = self.activityLog.last,
                       last.type == event.type, last.displayText == text {
                        continue
                    }
                    let entry = ActivityEntry(
                        id: UUID(),
                        timestamp: TimeInterval(event.ts_ms) / 1000,
                        type: event.type,
                        displayText: text,
                        relativeTime: event.relativeTime
                    )
                    self.activityLog.append(entry)
                    // Cap activity log to 100 entries (FIFO)
                    if self.activityLog.count > 100 {
                        self.activityLog = Array(self.activityLog.suffix(100))
                    }
                }

                // Purge expired probe messages (TTL 5 min) — du dictionnaire TTL
                // ET du store UI, sinon le cache ne se vide jamais réellement.
                let now = Date()
                let expiredKeys = Set(self.probeExpirationTimes.filter { $0.value <= now }.keys)
                if !expiredKeys.isEmpty {
                    for key in expiredKeys {
                        self.probeExpirationTimes.removeValue(forKey: key)
                    }
                    let before = self.messages.count
                    self.messages.removeAll { expiredKeys.contains($0.id) }
                    let purgeCnt = before - self.messages.count
                    self.appendLog(.info, "PROBE✕ \(expiredKeys.count) TTL expirés, \(purgeCnt) messages retirés du store")
                }

                // Sauvegarde les compteurs tous les 50 messages
                if self.totalMessagesCount - lastSaveMessageCount >= 50 {
                    lastSaveMessageCount = self.totalMessagesCount
                    self.savePersistentCounters()
                }

                // Poll status + peers
                if let status = await self.node.status() {
                    self.nodeId = status.nodeId
                    self.peersCount = status.peersCount
                    self.groupsCount = status.groupsCount
                    if let role = status.localRole { self.localRole = role }
                    if let pk = status.pathKind { self.pathKind = pk }
                    if let rtt = status.pathRttMs { self.pathRttMs = rtt }
                    self.pathAddr = status.pathAddr
                    self.pathsByPeer = status.pathsByPeer ?? [:]
                    self.clockSkewMs = status.clockSkewMs
                    self.clockSkewSamples = status.clockSkewSamples ?? 0
                    self.appBuild = status.appBuild
                }
                // L1-001 presence : logge chaque attestation acceptée
                // (sonde auto 15s → Live Log, zéro UI dédiée)
                if let ps = await self.node.presenceStats(), ps.acceptedTotal > lastPresenceTotal {
                    let short = String(ps.lastAttester.prefix(8))
                    self.appendLog(.success, "PRESENCE ✓ \(short) atteste en \(ps.lastLatencyMs)ms (total \(ps.acceptedTotal), fenêtre \(ps.windowCount), seed \(ps.seedPrefix))")
                    lastPresenceTotal = ps.acceptedTotal
                }
                let currentConnected = await self.node.connectedPeers()
                // Dédoublonnage : le transport peut exposer plusieurs chemins /
                // connexions vers un même pair → une seule entrée par NodeId
                // (ordre préservé). Sinon la liste « Connected » affiche des doublons.
                var seenConnected = Set<NodeId>()
                self.connectedPeers = currentConnected.filter { seenConnected.insert($0).inserted }

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
                if let relayUrl = normalizedRelayUrl {
                    await seedPeerRoute(nodeId: peer.nodeId, relayUrl: relayUrl, source: "auto-discovery")
                }
                let payload = Data(probe.utf8)
                try await node.sendMessage(to: peer.nodeId, payload: payload)
                autoMessagedPeerIds.insert(peer.nodeId)
                let displayName = displayName(for: peer.nodeId)
                appendLog(.network, "AUTO-PING → \(displayName): \(probe)")

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
    /// Inbox des messages de groupe reçus (dont rattrapage R13), lu par
    /// l'endpoint /inbox du serveur de contrôle. Borné.
    private var groupInbox: [TomGroupMessage] = []
    /// Buffer des logs UDP quand on est en cellulaire (hors LAN broadcast)
    private var udpLogPendingBuffer: [String] = []
    private static let udpLogBufferMax = 5000

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
        statusServer = StatusServer { [weak self] method, path, query in
            guard let self else { return "{}" }
            return await self.handleControl(method: method, path: path, query: query)
        }
        statusServer?.start()
        let ip = Self.getLocalIPv4() ?? "127.0.0.1"
        appendLog(.info, "Status server: http://\(ip):\(StatusServer.defaultPort)/")
    }

    /// Routeur de l'API de contrôle (miroir du CLI tom-node) — pilotage R13 sur
    /// device réel : créer un groupe, envoyer, accepter une invite, lire l'inbox.
    @MainActor
    func handleControl(method: String, path: String, query: [String: String]) async -> String {
        switch path {
        case "/":
            return buildStatusJSON()
        case "/inbox":
            let contains = query["contains"]
            let items = groupInbox.filter { contains == nil || $0.text.contains(contains!) }
            return encodeGroupInbox(items)
        case "/groups":
            return await node.listGroupsJSON()
        case "/invites":
            return await node.pendingInvitesJSON()
        case "/group/create", "/group/send", "/group/accept":
            // Écritures = pilotage du nœud : réservé aux builds DEBUG (outil de
            // test R13). Les builds release n'exposent AUCUN contrôle write sur
            // le LAN (durcissement — cf review sécurité, StatusServer sur 0.0.0.0).
            #if DEBUG
            return await handleControlWrite(path: path, query: query)
            #else
            return "{\"ok\":false,\"erreur\":\"contrôle write désactivé en release\"}"
            #endif
        default:
            return "{\"erreur\":\"route inconnue\"}"
        }
    }

    #if DEBUG
    @MainActor
    private func handleControlWrite(path: String, query: [String: String]) async -> String {
        switch path {
        case "/group/create":
            let name = query["name"] ?? "grp"
            let members = (query["members"] ?? "")
                .split(separator: ",").map(String.init).filter { !$0.isEmpty }
            do {
                try await node.createGroup(name: name, members: members, inviteOnly: false)
                return "{\"ok\":true,\"groupe_cree\":\"\(jsonEscape(name))\"}"
            } catch {
                return "{\"ok\":false,\"erreur\":\"\(jsonEscape("\(error)"))\"}"
            }
        case "/group/send":
            guard let gid = query["group"] else {
                return "{\"ok\":false,\"erreur\":\"param group manquant\"}"
            }
            let text = query["text"] ?? "CTRL:app"
            do {
                try await node.sendGroupMessage(groupId: gid, text: text)
                return "{\"ok\":true}"
            } catch {
                return "{\"ok\":false,\"erreur\":\"\(jsonEscape("\(error)"))\"}"
            }
        case "/group/accept":
            guard let gid = query["group"] else {
                return "{\"ok\":false,\"erreur\":\"param group manquant\"}"
            }
            do {
                try await node.acceptInvite(groupId: gid)
                return "{\"ok\":true}"
            } catch {
                return "{\"ok\":false,\"erreur\":\"\(jsonEscape("\(error)"))\"}"
            }
        default:
            return "{\"erreur\":\"route inconnue\"}"
        }
    }
    #endif

    /// Échappe une chaîne pour inclusion sûre dans une valeur JSON construite à
    /// la main (évite qu'un `name`/`error` avec `"` casse la réponse).
    private func jsonEscape(_ s: String) -> String {
        var out = ""
        for c in s {
            switch c {
            case "\"": out += "\\\""
            case "\\": out += "\\\\"
            case "\n": out += "\\n"
            case "\r": out += "\\r"
            case "\t": out += "\\t"
            default: out.append(c)
            }
        }
        return out
    }

    private func encodeGroupInbox(_ items: [TomGroupMessage]) -> String {
        struct Out: Encodable { let total: Int; let messages: [TomGroupMessage] }
        let out = Out(total: items.count, messages: items)
        guard let data = try? JSONEncoder().encode(out),
              let s = String(data: data, encoding: .utf8) else {
            return "{\"total\":0,\"messages\":[]}"
        }
        return s
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
        let recvCount = totalMessagesReceivedCount
        let dict: [String: Any] = [
            "schema_version": 1,
            "node": username,
            "node_id": nodeId,
            "platform": Self.appareil,
            "relay_url_active": relayUrl,
            "phase": phase,
            "taille_reseau": peersCount,
            "role": localRole,
            "relayeurs": connectedPeers.count,
            "pairs_connectes": connectedPeers,
            "groupes": [[String: Any]](),
            "messages_envoyes": sentCount,
            "messages_recus": recvCount,
            "messages_echoues": 0,
            "uptime_secondes": uptimeSec,
            // Build affiché à l'écran — exposé ici pour vérifier à distance
            // quel binaire tourne réellement (anti-staleness / anti-drift).
            "app_build": TomVersion.build,
            // Path par pair (vérité terrain #33) : node_id → kind/rtt/addr
            "paths_by_peer": pathsByPeer.mapValues { info -> [String: Any] in
                ["kind": info.kind, "rtt_ms": info.rtt_ms, "addr": info.addr]
            }
        ]
        guard let data = try? JSONSerialization.data(withJSONObject: dict, options: [.sortedKeys]),
              let str = String(data: data, encoding: .utf8) else { return "{}" }
        return str
    }

    /// Send a JSON log line over UDP (fire-and-forget).
    /// Sans lien LAN (cellulaire pur) : bufferiser (broadcast inutile).
    /// Dès qu'un WiFi/Ethernet existe : envoyer live (le broadcast atteint le
    /// collecteur par l'interface du bon sous-réseau, même si le cellulaire
    /// est la route primaire).
    private func sendLogUDP(_ message: String) {
        // Aucun lien LAN → bufferiser au lieu d'envoyer
        if !lanInterfaceAvailable {
            // Bufferiser la ligne
            let line = "\(message)\n"
            udpLogPendingBuffer.append(line)

            // Borne mémoire : drop les plus anciennes lignes si buffer trop gros
            if udpLogPendingBuffer.count > Self.udpLogBufferMax {
                let excess = udpLogPendingBuffer.count - Self.udpLogBufferMax
                udpLogPendingBuffer.removeFirst(excess)
            }
            return
        }

        // WiFi/Ethernet/nil : envoyer live comme avant
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

/// Activity log entry from a protocol event (used in the Activity section of StatusView).
struct ActivityEntry: Identifiable, Equatable {
    let id: UUID
    let timestamp: TimeInterval // seconds since epoch
    let type: String
    let displayText: String
    let relativeTime: String

    static func == (lhs: ActivityEntry, rhs: ActivityEntry) -> Bool {
        lhs.id == rhs.id
    }
}
