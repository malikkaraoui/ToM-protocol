import SwiftUI

struct ContentView: View {
    @EnvironmentObject var nodeService: TomNodeService

    var body: some View {
        TabView {
            StatusView()
                .tabItem {
                    Label("Status", systemImage: "antenna.radiowaves.left.and.right")
                }

            MessagesView()
                .tabItem {
                    Label("Messages", systemImage: "message")
                }

            GroupsView()
                .tabItem {
                    Label("Groups", systemImage: "person.3")
                }

            SettingsView()
                .tabItem {
                    Label("Settings", systemImage: "gear")
                }
        }
    }
}

#Preview {
    ContentView()
        .environmentObject(TomNodeService.shared)
}
