import SwiftUI
import TomProtocolKit

@main
struct TomNodeApp: App {
    @StateObject private var nodeService = TomNodeService.shared
    @Environment(\.scenePhase) private var scenePhase

    init() {
        // Exigence BGTaskScheduler : enregistrer les handlers AVANT la fin du
        // lancement de l'app (sinon exception à la première planification).
        BackgroundCoordinator.shared.registerTasks()
    }

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environmentObject(nodeService)
                .onAppear {
                    nodeService.scheduleInitialAutoStart()
                }
                .onChange(of: scenePhase) { newPhase in
                    switch newPhase {
                    case .background:
                        nodeService.handleEnterBackground()
                    case .active:
                        nodeService.handleReturnToForeground()
                    default:
                        break
                    }
                }
        }
    }
}
