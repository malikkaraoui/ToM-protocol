import Foundation
import TomProtocolKit
#if os(iOS) || os(tvOS)
import BackgroundTasks
#endif

/// Réveils opportunistes en arrière-plan via BGTaskScheduler (iOS/tvOS).
///
/// Modèle produit (décision 2026-07-15, non-objectif APNs/daemon) : l'app ne
/// vit PAS en permanence en arrière-plan. Elle accepte des réveils COURTS
/// (~30 s) planifiés par iOS pour relever les messages en attente (backup
/// ADR-009) puis se rendort proprement. La déclaration `fetch` +
/// `BGTaskSchedulerPermittedIdentifiers` rend aussi l'app visible dans
/// Réglages > Général > Actualisation en arrière-plan.
///
/// IMPORTANT : `registerTasks()` doit être appelé AVANT la fin du lancement
/// de l'app (exigence BGTaskScheduler) — fait dans `TomNodeApp.init()`.
@MainActor
final class BackgroundCoordinator {
    static let shared = BackgroundCoordinator()
    private init() {}

    /// Identifiant déclaré dans BGTaskSchedulerPermittedIdentifiers (project.yml).
    static let refreshTaskIdentifier = "malik.karaoui.TomNode.refresh"

    /// Durée de travail visée pendant un réveil (budget système ≈ 30 s).
    private static let refreshWorkSeconds: UInt64 = 18

    #if os(iOS) || os(tvOS)
    private var registered = false

    /// À appeler dans `App.init()` — enregistre le handler de réveil.
    func registerTasks() {
        guard !registered else { return }
        registered = true
        BGTaskScheduler.shared.register(
            forTaskWithIdentifier: Self.refreshTaskIdentifier,
            using: nil
        ) { task in
            guard let refresh = task as? BGAppRefreshTask else {
                task.setTaskCompleted(success: false)
                return
            }
            Task { @MainActor in
                BackgroundCoordinator.shared.handleRefresh(refresh)
            }
        }
    }

    /// Planifie le prochain réveil (idempotent — le système remplace une
    /// demande existante avec le même identifiant). `earliestBeginDate` est
    /// un MINIMUM : iOS choisit le moment réel selon l'usage de l'appareil.
    func scheduleRefresh(minutes: Double = 15) {
        let request = BGAppRefreshTaskRequest(identifier: Self.refreshTaskIdentifier)
        request.earliestBeginDate = Date(timeIntervalSinceNow: minutes * 60)
        do {
            try BGTaskScheduler.shared.submit(request)
            TomNodeService.shared.appendLog(.info, "BG: réveil planifié (≥\(Int(minutes)) min)")
        } catch {
            // BGTaskScheduler.Error.unavailable = Actualisation en arrière-plan
            // désactivée dans Réglages, ou simulateur. Pas fatal — on le dit.
            TomNodeService.shared.appendLog(.warning, "BG: planification refusée: \(error.localizedDescription)")
        }
    }

    /// Réveil accordé par iOS : relever les messages (backup ADR-009) pendant
    /// ~18 s avec un nœud démarré PROPREMENT, puis arrêt PROPRE. Ne touche à
    /// rien si le nœud tourne déjà (retour foreground concurrent).
    private func handleRefresh(_ task: BGAppRefreshTask) {
        let service = TomNodeService.shared
        service.appendLog(.warning, "BG: réveil BGAppRefresh accordé par iOS")

        // Toujours replanifier le suivant — sinon les réveils s'arrêtent.
        scheduleRefresh()

        // Si le nœud tourne déjà (grâce en cours / retour foreground), ne pas
        // empiler un cycle start/stop par-dessus.
        guard service.state == .stopped else {
            service.appendLog(.info, "BG: nœud déjà actif — rien à faire")
            task.setTaskCompleted(success: true)
            return
        }

        let work = Task { @MainActor in
            service.start()
            try? await Task.sleep(nanoseconds: Self.refreshWorkSeconds * 1_000_000_000)
            guard !Task.isCancelled else { return }
            service.appendLog(.info, "BG: fin du réveil — arrêt propre")
            service.stop()
            task.setTaskCompleted(success: true)
        }

        task.expirationHandler = {
            Task { @MainActor in
                work.cancel()
                TomNodeService.shared.appendLog(.warning, "BG: budget épuisé — arrêt immédiat")
                TomNodeService.shared.stop()
                task.setTaskCompleted(success: false)
            }
        }
    }
    #else
    // macOS : pas de BGTaskScheduler (l'app reste active à l'écran).
    func registerTasks() {}
    func scheduleRefresh(minutes: Double = 15) {}
    #endif
}
