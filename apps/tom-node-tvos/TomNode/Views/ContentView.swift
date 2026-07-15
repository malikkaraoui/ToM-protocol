import SwiftUI
import TomProtocolKit

struct ContentView: View {
    @EnvironmentObject var nodeService: TomNodeService

    var body: some View {
        TabView {
            StatusView()
                .tabItem {
                    Label("Status", systemImage: "antenna.radiowaves.left.and.right")
                }

            LogView()
                .tabItem {
                    Label("Live Log", systemImage: "terminal")
                }

            MessagesView()
                .tabItem {
                    Label("Messages", systemImage: "message")
                }

            ActivityView()
                .tabItem {
                    Label("Activité", systemImage: "waveform.circle.fill")
                }

            SettingsView()
                .tabItem {
                    Label("Réglages", systemImage: "gear")
                }
        }
    }
}

#Preview {
    ContentView()
        .environmentObject(TomNodeService.shared)
}
