import AVFoundation
import SwiftUI

@main
struct CascadeiOSApp: App {
    @State private var store = AppStore.bootstrap()

    init() {
        // iOS-only audio-session setup. Done here at process start so the
        // session is ready before any view appears — `AudioEngine.start()`
        // re-asserts it on every play, but routes / interruptions can come
        // through before that.
        let session = AVAudioSession.sharedInstance()
        try? session.setCategory(.playback, mode: .default,
                                 options: [.allowAirPlay, .allowBluetoothA2DP])
        try? session.setActive(true)
    }

    var body: some Scene {
        WindowGroup {
            CascadeScreen()
                .environment(store)
                .onAppear {
                    wireWatchConnectivity()
                }
        }
    }

    /// Hook the iPhone-side WatchConnectivity bridge to the live AppStore.
    /// Commands from the watch run through `store.dispatch`; every snapshot
    /// update (set by `AppStore.apply`) is pushed back to the watch.
    @MainActor
    private func wireWatchConnectivity() {
        let svc = PhoneConnectivityService.shared
        svc.activate(
            dispatch: { [weak store] command in
                store?.dispatch(command)
            },
            snapshotProvider: { [weak store] in
                guard let store else { return .placeholder }
                return WatchSnapshotMapper.map(store.snapshot)
            }
        )
        store.onSnapshotChanged = { snapshot in
            PhoneConnectivityService.shared.pushSnapshot(
                WatchSnapshotMapper.map(snapshot)
            )
        }
    }
}
