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
        }
    }
}
