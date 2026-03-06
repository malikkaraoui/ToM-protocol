import SwiftUI

@main
struct TomNodeApp: App {
    @StateObject private var nodeService = TomNodeService.shared

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environmentObject(nodeService)
        }
    }
}
