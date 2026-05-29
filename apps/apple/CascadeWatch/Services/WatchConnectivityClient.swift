import Foundation
import Observation
import WatchConnectivity

/// Watch-side end of the WatchConnectivity link.
///
/// Holds the latest `PhoneSnapshotForWatch` the iPhone has pushed and exposes
/// `send(_:)` to dispatch commands back to the phone.
///
/// `@Observable` so SwiftUI views just read `snapshot` and rebind when it
/// changes — no `@Published`, no Combine.
@MainActor
@Observable
final class WatchConnectivityClient: NSObject {
    static let shared = WatchConnectivityClient()

    /// Latest snapshot the iPhone has sent. Persisted across launches via
    /// `UserDefaults` so a cold open shows the last known state immediately.
    private(set) var snapshot: PhoneSnapshotForWatch
    private(set) var isReachable: Bool = false
    private(set) var isActivated: Bool = false

    private let session: WCSession?
    private let encoder = JSONEncoder()
    private let decoder = JSONDecoder()

    private static let snapshotKey = "cascade.lastSnapshot.v1"

    override init() {
        self.session = WCSession.isSupported() ? WCSession.default : nil
        self.snapshot = WatchConnectivityClient.loadCachedSnapshot()
        super.init()
    }

    func activate() {
        guard let session else { return }
        session.delegate = self
        session.activate()
    }

    /// Send a command to the iPhone. Prefers the live `sendMessageData`
    /// channel (instant, requires reachability) and falls back to
    /// `transferUserInfo` for guaranteed delivery when the iPhone app is
    /// suspended.
    func send(_ command: WatchToPhoneCommand) {
        guard let session, let data = try? encoder.encode(command) else { return }
        if session.isReachable {
            session.sendMessageData(data, replyHandler: { [weak self] reply in
                Task { @MainActor in
                    self?.applySnapshotData(reply)
                }
            }, errorHandler: { error in
                NSLog("[Cascade] sendMessageData failed: \(error)")
            })
        } else {
            session.transferUserInfo(["queuedCommand": data])
        }
    }

    private func applySnapshotData(_ data: Data) {
        guard let snap = try? decoder.decode(PhoneSnapshotForWatch.self, from: data) else { return }
        snapshot = snap
        Self.cacheSnapshot(snap)
    }

    // MARK: - Cached snapshot for cold-start render

    private static func loadCachedSnapshot() -> PhoneSnapshotForWatch {
        guard let data = UserDefaults.standard.data(forKey: snapshotKey),
              let snap = try? JSONDecoder().decode(PhoneSnapshotForWatch.self, from: data)
        else { return .placeholder }
        return snap
    }

    private static func cacheSnapshot(_ snapshot: PhoneSnapshotForWatch) {
        if let data = try? JSONEncoder().encode(snapshot) {
            UserDefaults.standard.set(data, forKey: snapshotKey)
        }
    }
}

extension WatchConnectivityClient: WCSessionDelegate {
    nonisolated func session(_ session: WCSession,
                             activationDidCompleteWith activationState: WCSessionActivationState,
                             error: Error?) {
        Task { @MainActor in
            self.isActivated = activationState == .activated
            self.isReachable = session.isReachable
            // Ask the phone for a fresh snapshot now that we're up.
            self.send(.requestSnapshot)
        }
    }

    nonisolated func sessionReachabilityDidChange(_ session: WCSession) {
        Task { @MainActor in
            self.isReachable = session.isReachable
            if session.isReachable {
                self.send(.requestSnapshot)
            }
        }
    }

    /// Proactive snapshot push from the iPhone (state changed without the
    /// watch asking — e.g. the user hit pause on the phone).
    nonisolated func session(_ session: WCSession,
                             didReceiveMessageData messageData: Data) {
        Task { @MainActor in
            self.applySnapshotData(messageData)
        }
    }

    /// Last-known-good snapshot delivered via `updateApplicationContext`.
    /// Used on cold start before the iPhone has had a chance to reply live.
    nonisolated func session(_ session: WCSession,
                             didReceiveApplicationContext applicationContext: [String: Any]) {
        guard let data = applicationContext["snapshot"] as? Data else { return }
        Task { @MainActor in
            self.applySnapshotData(data)
        }
    }
}
