import Foundation
import WatchConnectivity

/// iPhone-side WatchConnectivity bridge.
///
/// Receives commands from the watch (play, pause, set volume, start a focus
/// session, cancel the timer), translates them into core `Command`s, and
/// pushes a compact `PhoneSnapshotForWatch` back after every dispatch.
///
/// Reachability:
/// - `sendMessageData` — used when both apps are in the foreground; the watch
///   gets the snapshot back inline as the message reply.
/// - `updateApplicationContext` — used as the "last known good" snapshot the
///   watch sees on cold start. Delivered to the watch by the OS opportunistically,
///   even when one side isn't running. Only the latest one survives.
@MainActor
final class PhoneConnectivityService: NSObject {
    static let shared = PhoneConnectivityService()

    private let session: WCSession?

    private var dispatch: ((Command) -> Void)?
    private var snapshotProvider: (() -> PhoneSnapshotForWatch)?

    /// Latest snapshot pushed via `updateApplicationContext`. Tracked so we
    /// don't churn the OS-managed channel with duplicates.
    private var lastContext: PhoneSnapshotForWatch?

    private let encoder = JSONEncoder()
    private let decoder = JSONDecoder()

    override init() {
        self.session = WCSession.isSupported() ? WCSession.default : nil
        super.init()
    }

    /// Wire the service to an active `AppStore`. Idempotent — calling twice
    /// just overwrites the closures (last writer wins).
    func activate(
        dispatch: @escaping (Command) -> Void,
        snapshotProvider: @escaping () -> PhoneSnapshotForWatch
    ) {
        self.dispatch = dispatch
        self.snapshotProvider = snapshotProvider
        guard let session else { return }
        session.delegate = self
        session.activate()
        // Seed the application context so the watch has something to render
        // on a cold launch even before it asks.
        pushSnapshot(snapshotProvider())
    }

    /// Push a fresh snapshot to the watch. Called after every dispatch.
    func pushSnapshot(_ snapshot: PhoneSnapshotForWatch) {
        guard let session, session.activationState == .activated else { return }
        // Live channel for foreground use.
        if session.isReachable,
           let data = try? encoder.encode(snapshot) {
            session.sendMessageData(data, replyHandler: nil)
        }
        // Background-survivable channel for cold-start render. Avoid
        // re-sending the same payload (the OS coalesces but it costs a copy).
        if snapshot != lastContext,
           let data = try? encoder.encode(snapshot) {
            try? session.updateApplicationContext(["snapshot": data])
            lastContext = snapshot
        }
    }

    private func handle(_ commandData: Data) {
        guard let command = try? decoder.decode(WatchToPhoneCommand.self, from: commandData) else {
            return
        }
        let dispatch = self.dispatch
        switch command {
        case .requestSnapshot:
            break // reply already includes the snapshot
        case .togglePlayback: dispatch?(.togglePlayback)
        case .play:           dispatch?(.play)
        case .pause:          dispatch?(.pause)
        case .setVolume(let percent):
            dispatch?(.setVolume(percent: percent))
        case .toggleMute:
            dispatch?(.toggleMute)
        case .startSession(let preset):
            dispatch?(.startPomodoro(minutes: preset.minutes))
        case .startCustom(let minutes, let sleep):
            let clamped = max(1, min(1440, minutes))
            dispatch?(sleep ? .startSleepTimer(minutes: clamped)
                            : .startPomodoro(minutes: clamped))
        case .cancelTimer:    dispatch?(.cancelTimer)
        }
    }
}

extension PhoneConnectivityService: WCSessionDelegate {
    nonisolated func session(_ session: WCSession,
                             activationDidCompleteWith activationState: WCSessionActivationState,
                             error: Error?) {
        if let error {
            NSLog("[Cascade] WCSession activation failed: \(error)")
        }
    }

    nonisolated func sessionDidBecomeInactive(_ session: WCSession) {}

    nonisolated func sessionDidDeactivate(_ session: WCSession) {
        // The system handed us off; reactivate on the same delegate.
        session.activate()
    }

    /// Live foreground command from the watch — handle it and reply with the
    /// post-dispatch snapshot so the watch UI updates in one round trip.
    nonisolated func session(_ session: WCSession,
                             didReceiveMessageData messageData: Data,
                             replyHandler: @escaping (Data) -> Void) {
        Task { @MainActor in
            self.handle(messageData)
            let snap = self.snapshotProvider?() ?? .placeholder
            let data = (try? self.encoder.encode(snap)) ?? Data()
            replyHandler(data)
        }
    }

    /// User issued a command while the iPhone app was suspended — queued for
    /// guaranteed delivery via `transferUserInfo`. Apply on next launch.
    nonisolated func session(_ session: WCSession, didReceiveUserInfo userInfo: [String: Any] = [:]) {
        guard let data = userInfo["queuedCommand"] as? Data else { return }
        Task { @MainActor in
            self.handle(data)
        }
    }
}
