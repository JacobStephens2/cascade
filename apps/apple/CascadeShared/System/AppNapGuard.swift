import Foundation

/// Tells macOS the app is doing important work so it doesn't throttle our
/// 250ms `Timer` to once-every-10-seconds when the main window is closed.
///
/// Audio playback alone keeps the *audio* path warm, but the timer-tick
/// scheduling is a separate concern — without this the countdown would lag.
@MainActor
final class AppNapGuard {
    private var token: NSObjectProtocol?

    func begin(reason: String) {
        guard token == nil else { return }
        token = ProcessInfo.processInfo.beginActivity(
            options: [.userInitiated, .latencyCritical, .idleSystemSleepDisabled],
            reason: reason
        )
    }

    func end() {
        if let t = token {
            ProcessInfo.processInfo.endActivity(t)
            token = nil
        }
    }

    deinit {
        if let t = token {
            ProcessInfo.processInfo.endActivity(t)
        }
    }
}
