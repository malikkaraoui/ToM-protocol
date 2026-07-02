import SwiftUI

@main
struct TomNodeApp: App {
    @StateObject private var nodeService = TomNodeService.shared
    @Environment(\.scenePhase) private var scenePhase

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
