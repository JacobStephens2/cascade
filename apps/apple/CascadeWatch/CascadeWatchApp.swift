import SwiftUI

@main
struct CascadeWatchApp: App {
    @State private var connectivity = WatchConnectivityClient.shared

    init() {
        WatchConnectivityClient.shared.activate()
    }

    var body: some Scene {
        WindowGroup {
            WatchRootView()
                .environment(connectivity)
        }
    }
}
