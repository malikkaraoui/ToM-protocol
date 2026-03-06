import SwiftUI

@main
struct TomNodeApp: App {
    @StateObject private var nodeService = TomNodeService.shared
    @Environment(\.scenePhase) private var scenePhase

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environmentObject(nodeService)
                .onChange(of: scenePhase) { newPhase in
                    if newPhase == .active {
                        nodeService.handleReturnToForeground()
                    }
                }
        }
    }
}
