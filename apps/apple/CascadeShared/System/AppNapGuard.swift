import Foundation

/// Tells macOS the app is doing important work so the kernel doesn't throttle
/// our 250 ms `Timer` to once-every-10-seconds when the main window is closed.
///
/// On iOS this is a no-op: there's no App Nap, and `UIBackgroundModes: audio`
/// already keeps the runtime alive while playback is active. The class still
/// exists on iOS so `AppStore` doesn't have to `#if` around its callsites.
@MainActor
final class AppNapGuard {
    #if os(macOS)
    private var token: NSObjectProtocol?
    #endif

    func begin(reason: String) {
        #if os(macOS)
        guard token == nil else { return }
        token = ProcessInfo.processInfo.beginActivity(
            options: [.userInitiated, .latencyCritical, .idleSystemSleepDisabled],
            reason: reason
        )
        #endif
    }

    func end() {
        #if os(macOS)
        if let t = token {
            ProcessInfo.processInfo.endActivity(t)
            token = nil
        }
        #endif
    }

    deinit {
        #if os(macOS)
        if let t = token {
            ProcessInfo.processInfo.endActivity(t)
        }
        #endif
    }
}
