import Foundation

final class RelayManager: ObservableObject {
    @Published var isRunning = false
    @Published var statusText = "Stopped"

    let bindAddress = "0.0.0.0:3343"
    let metricsAddress = "0.0.0.0:9093"

    func start() {
        // NOTE:
        // This expects Rust FFI symbols exposed by a tvOS-compatible static library.
        // See TomRelayFFI.h and apps/relay-tvos/README.md.
        let rc = tom_relay_start(bindAddress, metricsAddress)
        if rc == 0 {
            isRunning = true
            statusText = "Running"
        } else {
            isRunning = false
            statusText = "Start failed (code \(rc))"
        }
    }

    func stop() {
        tom_relay_stop()
        isRunning = false
        statusText = "Stopped"
    }
}
