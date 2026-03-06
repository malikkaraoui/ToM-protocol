import Foundation
import os.log

actor TomNodeWrapper {
    private let log = Logger(subsystem: "org.tom-protocol.tom-node", category: "TomNodeWrapper")
    private var handle: TomNodeHandle?

    var isRunning: Bool {
        handle != nil
    }

    func create(relayUrl: String, identityPath: String?, n0Discovery: Bool) throws {
        guard handle == nil else {
            throw TomError.alreadyRunning
        }

        var config: [String: Any] = [
            "relay_url": relayUrl,
            "n0_discovery": n0Discovery
        ]
        if let path = identityPath {
            config["identity_path"] = path
        }

        let jsonData = try JSONSerialization.data(withJSONObject: config)
        guard let jsonStr = String(data: jsonData, encoding: .utf8) else {
            throw TomError.jsonParseFailed("Failed to encode config")
        }

        log.info("Creating node with config: \(jsonStr)")

        guard let h = jsonStr.withCString({ tom_node_create($0) }) else {
            throw TomError.ffiReturnedNull(function: "tom_node_create")
        }

        handle = h
        log.info("Node created successfully")
    }

    func start(
        username: String,
        encryption: Bool,
        enableDht: Bool,
        relayUrl: String,
        identityPath: String?,
        n0Discovery: Bool,
        dataDir: String?
    ) throws {
        guard let h = handle else {
            throw TomError.notStarted
        }

        var config: [String: Any] = [
            "username": username,
            "encryption": encryption,
            "enable_dht": enableDht,
            "relay_url": relayUrl,
            "n0_discovery": n0Discovery
        ]
        if let path = identityPath {
            config["identity_path"] = path
        }
        if let dir = dataDir {
            config["data_dir"] = dir
        }

        let jsonData = try JSONSerialization.data(withJSONObject: config)
        guard let jsonStr = String(data: jsonData, encoding: .utf8) else {
            throw TomError.jsonParseFailed("Failed to encode runtime config")
        }

        log.info("Starting node runtime...")

        let result = jsonStr.withCString { tom_node_start(h, $0) }
        if result != 0 {
            var errorDetail = "tom_node_start returned \(result)"
            if let errPtr = tom_node_last_error(h) {
                errorDetail = String(cString: errPtr)
                tom_node_free_string(errPtr)
            }
            log.error("Start failed: \(errorDetail)")
            throw TomError.unknown(errorDetail)
        }

        log.info("Node runtime started")
    }

    func stop() {
        guard let h = handle else { return }
        log.info("Stopping node...")
        tom_node_stop(h)
        handle = nil
        log.info("Node stopped")
    }

    /// Drop the handle without graceful shutdown (safe after OS suspend)
    func forceReset() {
        guard let h = handle else { return }
        log.warning("Force-resetting node handle (post-sleep cleanup)")
        tom_node_free(h)
        handle = nil
    }

    func sendMessage(to target: NodeId, payload: Data) throws {
        guard let h = handle else {
            throw TomError.notStarted
        }

        let result = target.withCString { targetCStr in
            payload.withUnsafeBytes { bufPtr -> Int32 in
                let ptr = bufPtr.baseAddress?.assumingMemoryBound(to: UInt8.self)
                return tom_node_send_message(h, targetCStr, ptr, bufPtr.count)
            }
        }

        if result != 0 {
            throw TomError.sendFailed("tom_node_send_message returned \(result)")
        }
    }

    func createGroup(name: String, members: [NodeId], inviteOnly: Bool) throws {
        guard let h = handle else {
            throw TomError.notStarted
        }

        let config: [String: Any] = [
            "name": name,
            "initial_members": members,
            "invite_only": inviteOnly
        ]

        let jsonData = try JSONSerialization.data(withJSONObject: config)
        guard let jsonStr = String(data: jsonData, encoding: .utf8) else {
            throw TomError.jsonParseFailed("Failed to encode group config")
        }

        let result = jsonStr.withCString { tom_node_create_group(h, $0) }
        if result != 0 {
            throw TomError.groupCreateFailed("tom_node_create_group returned \(result)")
        }
    }

    func sendGroupMessage(groupId: GroupId, text: String) throws {
        guard let h = handle else {
            throw TomError.notStarted
        }

        let result = groupId.withCString { gidCStr in
            text.withCString { textCStr in
                tom_node_send_group_message(h, gidCStr, textCStr)
            }
        }

        if result != 0 {
            throw TomError.sendFailed("tom_node_send_group_message returned \(result)")
        }
    }

    func receiveMessages() -> [TomMessage] {
        guard let h = handle else { return [] }

        guard let cStr = tom_node_receive_messages(h) else { return [] }
        let jsonStr = String(cString: cStr)
        tom_node_free_string(cStr)

        guard jsonStr != "[]",
              let data = jsonStr.data(using: .utf8) else {
            return []
        }

        do {
            return try JSONDecoder().decode([TomMessage].self, from: data)
        } catch {
            log.error("Failed to decode messages: \(error.localizedDescription)")
            return []
        }
    }

    func status() -> TomNodeStatus? {
        guard let h = handle else { return nil }

        guard let cStr = tom_node_status(h) else { return nil }
        let jsonStr = String(cString: cStr)
        tom_node_free_string(cStr)

        guard let data = jsonStr.data(using: .utf8) else { return nil }

        do {
            return try JSONDecoder().decode(TomNodeStatus.self, from: data)
        } catch {
            log.error("Failed to decode status: \(error.localizedDescription)")
            return nil
        }
    }
}
