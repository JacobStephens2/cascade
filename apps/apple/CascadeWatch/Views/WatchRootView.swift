import SwiftUI

/// Vertical-paged root, per the watchOS 10 design language. Three pages:
///
/// 1. **Player** — the 95% case: status line, big play/pause, Crown for volume.
/// 2. **Session** — focus / sleep timer presets.
/// 3. **Status** — connectivity diagnostics for when something's off.
struct WatchRootView: View {
    var body: some View {
        TabView {
            WatchPlayerView()
                .tag(0)
            WatchSessionView()
                .tag(1)
            WatchStatusView()
                .tag(2)
        }
        .tabViewStyle(.verticalPage)
    }
}
