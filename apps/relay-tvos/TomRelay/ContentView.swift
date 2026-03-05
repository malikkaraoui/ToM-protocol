import SwiftUI

struct ContentView: View {
    @StateObject private var manager = RelayManager()

    var body: some View {
        VStack(spacing: 18) {
            Text("ToM Relay tvOS")
                .font(.title)

            Text(manager.statusText)
                .foregroundColor(manager.isRunning ? .green : .orange)

            HStack(spacing: 16) {
                Button("Start") {
                    manager.start()
                }
                .buttonStyle(.borderedProminent)

                Button("Stop") {
                    manager.stop()
                }
                .buttonStyle(.bordered)
            }

            Text("HTTP: \(manager.bindAddress)")
                .font(.footnote)
        }
        .padding(24)
    }
}

#Preview {
    ContentView()
}
