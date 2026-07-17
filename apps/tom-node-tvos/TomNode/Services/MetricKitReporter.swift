import Foundation
#if os(iOS)
import MetricKit

/// Réception des diagnostics système (crash, hang, watchdog, CPU/mémoire)
/// via MetricKit — LA voie officielle pour diagnostiquer une app sur un
/// appareil perso quand « Données et analyses » (Réglages) est vide :
/// iOS livre les payloads À L'APP elle-même au lancement suivant.
///
/// Les diagnostics sont : (1) tracés dans le Live Log (donc broadcastés au
/// collecteur UDP), (2) archivés en JSON dans <data_dir>/metrickit/ pour
/// récupération ultérieure (idevicecrashreport/Xcode ne montrent pas tout).
@MainActor
final class MetricKitReporter: NSObject, MXMetricManagerSubscriber {
    static let shared = MetricKitReporter()

    private var started = false
    private var archiveDir: URL?

    func start(dataDir: String?) {
        guard !started else { return }
        started = true
        if let dir = dataDir {
            let url = URL(fileURLWithPath: dir).appendingPathComponent("metrickit", isDirectory: true)
            try? FileManager.default.createDirectory(at: url, withIntermediateDirectories: true)
            archiveDir = url
        }
        MXMetricManager.shared.add(self)
        TomNodeService.shared.appendLog(.info, "MetricKit actif (diagnostics crash/hang livrés au prochain lancement)")
    }

    nonisolated func didReceive(_ payloads: [MXDiagnosticPayload]) {
        Task { @MainActor in
            for payload in payloads {
                let crashes = payload.crashDiagnostics?.count ?? 0
                let hangs = payload.hangDiagnostics?.count ?? 0
                let cpu = payload.cpuExceptionDiagnostics?.count ?? 0
                let disk = payload.diskWriteExceptionDiagnostics?.count ?? 0
                TomNodeService.shared.appendLog(
                    .error,
                    "MetricKit: \(crashes) crash, \(hangs) hang, \(cpu) CPU, \(disk) disque (fenêtre \(payload.timeStampBegin)→\(payload.timeStampEnd))"
                )
                self.archive(payload.jsonRepresentation(), prefix: "diagnostic")
            }
        }
    }

    nonisolated func didReceive(_ payloads: [MXMetricPayload]) {
        // Métriques quotidiennes (batterie, mémoire pic, temps foreground…) —
        // archivées sans bruit dans le Live Log.
        Task { @MainActor in
            for payload in payloads {
                self.archive(payload.jsonRepresentation(), prefix: "metrics")
            }
        }
    }

    private func archive(_ json: Data, prefix: String) {
        guard let dir = archiveDir else { return }
        let stamp = ISO8601DateFormatter().string(from: Date())
            .replacingOccurrences(of: ":", with: "-")
        let file = dir.appendingPathComponent("\(prefix)-\(stamp).json")
        try? json.write(to: file)
    }
}
#endif
