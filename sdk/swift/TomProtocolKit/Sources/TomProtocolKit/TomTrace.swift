import Foundation

/// Boîte noire du cycle de vie start/stop (enquête gel post-arrêt-propre,
/// 2026-07-17). Chaque étape est ÉCRITE SUR DISQUE immédiatement : quand un
/// appel FFI ne rend jamais la main (actor empoisonné, plus aucun log
/// os.log/UDP), le fichier livre la dernière étape atteinte — récupérable
/// sans root via :
/// `devicectl device copy from --domain-type appDataContainer
///  --domain-identifier <bundle> --source Library/Caches/start_trace.log`
///
/// Deux horloges par ligne : époque (murale) + uptime système. Un trou
/// mural >> trou d'uptime entre deux lignes = le process a été SUSPENDU par
/// l'OS entre les deux (l'uptime s'arrête avec lui, pas l'horloge murale).
public enum TomTrace {
    private static let queue = DispatchQueue(label: "org.tom-protocol.trace", qos: .utility)
    private static let maxBytes = 256 * 1024

    private static var fileURL: URL? {
        FileManager.default.urls(for: .cachesDirectory, in: .userDomainMask)
            .first?.appendingPathComponent("start_trace.log")
    }

    /// Trace une étape. Synchronisé sur disque (fsync) — survit à un kill.
    /// Hors du chemin chaud : uniquement le cycle de vie (start/stop/scène),
    /// jamais le trafic messages.
    public static func log(_ step: String) {
        let line = String(
            format: "%.3f up=%.3f %@\n",
            Date().timeIntervalSince1970,
            ProcessInfo.processInfo.systemUptime,
            step
        )
        queue.async {
            guard let url = fileURL, let data = line.data(using: .utf8) else { return }
            let fm = FileManager.default
            if !fm.fileExists(atPath: url.path) {
                fm.createFile(atPath: url.path, contents: nil)
            }
            guard let fh = try? FileHandle(forWritingTo: url) else { return }
            defer { try? fh.close() }
            if let size = try? fh.seekToEnd(), size > maxBytes {
                // Rotation grossière : on repart de zéro (l'historique utile
                // est toujours la fin — le gel est récent par construction).
                // Échec de troncature = on N'ÉCRIT PAS (sinon le plafond
                // 256 Ko devient fictif et le fichier grossit sans borne).
                do { try fh.truncate(atOffset: 0) } catch { return }
            }
            try? fh.write(contentsOf: data)
            try? fh.synchronize()
        }
    }
}
