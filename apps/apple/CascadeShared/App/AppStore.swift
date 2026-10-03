import Combine
import Foundation
import Observation
import SwiftUI

/// Single source of truth for the SwiftUI layer. Owns the Rust bridge, the
/// audio engine, the persisted-settings store, and the power-management
/// helpers. Same shape as the Android `CascadeBridgeHolder` and the web
/// `useCascade` hook.
///
/// `@Observable` (macOS 14+) means every SwiftUI view that reads `snapshot`
/// automatically re-renders when it changes — no manual `@Published`.
@MainActor
@Observable
final class AppStore {
    private(set) var snapshot: Snapshot
    private(set) var lastError: String?

    // Optional account + sync. The core holds the account and decides what
    // to send; the shell carries the requests it is handed over HTTP and
    // decides only when it can talk (a lifecycle trigger).
    var syncAvailable: Bool { SyncConfig.available }

    private let accountStore = AccountStore()
    private let syncApi = SyncApi()

    /// Side-channel observer for non-SwiftUI consumers (the iPhone's
    /// `PhoneConnectivityService` uses this to push every snapshot down to
    /// the watch). SwiftUI views observe `snapshot` directly via `@Observable`.
    var onSnapshotChanged: ((Snapshot) -> Void)?

    private let bridge: CoreBridge
    private let audio: AudioEngine
    private let settings: SettingsStore
    private let power: PowerAssertion
    private let napGuard: AppNapGuard
    private let nowPlaying: NowPlayingController

    private var tickTimer: Timer?
    /// Interval the tick loop is currently running at, in ms; 0 when stopped.
    private var tickIntervalMs: UInt64 = 0

    /// Bootstrap from disk. The core's `restore` keeps its defaults for any
    /// missing or malformed blob, so the user never gets stuck on a startup
    /// error for something as trivial as malformed settings JSON.
    static func bootstrap() -> AppStore {
        let settings = SettingsStore()
        let store = AppStore(bridge: CoreBridge(), settings: settings)
        // Boot is one step: a fresh core, then one `restore` carrying the
        // persisted blobs (empty = none) and a fallback device id. The core
        // ignores a missing/incompatible blob and never lets a restore lower
        // the counter. The fallback id is the one this shell used to store
        // itself (so an existing server slot carries over), else a fresh one;
        // the core only adopts it if the listening blob carries no id.
        store.dispatch(.restore(
            settingsJson: settings.readSafely() ?? "",
            listeningJson: settings.readListeningSafely() ?? "",
            fallbackDeviceId: store.accountStore.legacyDeviceId() ?? UUID().uuidString,
            accountJson: store.accountStore.readAccountJson()))
        // Fetch the cross-device total straight away. The core sends nothing
        // while signed out, and answers a fresh sign-in with its own refresh.
        store.sync(reason: .refresh)
        return store
    }

    private init(bridge: CoreBridge, settings: SettingsStore) {
        self.bridge = bridge
        self.settings = settings
        self.audio = AudioEngine()
        self.power = PowerAssertion()
        self.napGuard = AppNapGuard()
        // Initial snapshot — if the bridge can't even render its own state,
        // we fall back to an empty one and surface the error.
        let initial: Snapshot
        do {
            initial = try bridge.snapshot()
            self.lastError = nil
        } catch {
            initial = Snapshot.empty
            self.lastError = "\(error)"
        }
        self.snapshot = initial
        self.nowPlaying = NowPlayingController()
        // Now Playing remote commands route back through `dispatch` — close
        // the loop after `self` exists.
        nowPlaying.bindRemoteCommands { [weak self] command in
            Task { @MainActor in self?.dispatch(command) }
        }
        nowPlaying.update(snapshot: initial)
    }

    func dispatch(_ command: Command) {
        do {
            apply(try bridge.dispatch(command))
        } catch {
            lastError = "\(error)"
        }
    }

    private func apply(_ update: Update) {
        snapshot = update.snapshot
        lastError = update.snapshot.errorMessage
        onSnapshotChanged?(update.snapshot)

        for effect in update.effects {
            switch effect {
            case .startPlayback(let gain):
                audio.start(gain: gain)
                power.acquire()
                Task { @MainActor in dispatch(.platformPlaybackStarted) }
            case .pausePlayback:
                audio.pause()
                power.release()
            case .setPlatformVolume(let gain):
                audio.setVolume(gain: gain)
            case .persistSettings(let json):
                settings.writeSafely(json)
            case .persistListening(let json):
                settings.writeListeningSafely(json)
            case .persistAccount(let json):
                accountStore.writeAccountJson(json)
            case .clearSession:
                // Only for a shell that keeps its own account; this one
                // restores with `accountJson`, so the core holds it.
                break
            case .pushListening, .sendSignInLink, .verifySignInToken, .revokeSession,
                 .deleteServerListening, .deleteServerAccount:
                carry(effect)
            }
        }

        // Tick loop: run at the cadence the core asks for (0 = stop). Restart
        // only when the cadence changes.
        let desired = update.snapshot.tickIntervalMs
        if desired != tickIntervalMs {
            stopTicking()
            if desired > 0 { startTicking(intervalMs: desired) }
        }

        nowPlaying.update(snapshot: update.snapshot)
    }

    // MARK: - Sync

    /// Offer the core a listening sync. It answers with a `pushListening` only
    /// if there is something to send and it holds a session; `apply` carries it.
    func sync(reason: SyncReason) {
        guard syncAvailable else { return }
        dispatch(.beginListeningSync(reason: reason))
    }

    /// Entry point for `.onOpenURL`: the core finds the token in the link.
    func handleOpenURL(_ url: URL) {
        guard syncAvailable else { return }
        dispatch(.submitSignInLink(input: url.absoluteString))
    }

    /// Carry one request effect over HTTP and settle it with the core. Every
    /// request but `revokeSession` must be settled, or the core won't start
    /// another; the settle's own effects come back through `apply`. Only
    /// reached with sync available: `sync`, `handleOpenURL` and the account
    /// controls are the only ways a request starts.
    private func carry(_ effect: Effect) {
        let api = syncApi
        let accountFailed = { (unauthorized: Bool) in Command.accountRequestFailed(unauthorized: unauthorized) }
        Task { @MainActor in
            switch effect {
            case .pushListening(let deviceId, let deviceTotalMs, let sessionToken):
                await settle(
                    {
                        let res = try await api.putListening(
                            token: sessionToken,
                            deviceId: deviceId,
                            deviceTotalMs: Int64(clamping: deviceTotalMs))
                        return .listeningSyncSucceeded(serverTotalMs: UInt64(max(0, res.serverTotalMs)))
                    },
                    failed: { .listeningSyncFailed(unauthorized: $0) })
            case .sendSignInLink(let email):
                await settle(
                    {
                        try await api.requestLink(email: email)
                        return .signInLinkSent
                    },
                    failed: accountFailed)
            case .verifySignInToken(let token):
                await settle(
                    {
                        let res = try await api.verify(token: token)
                        return .signInVerified(sessionToken: res.sessionToken, email: res.email)
                    },
                    failed: accountFailed)
            case .revokeSession(let sessionToken):
                // Already gone server-side or offline — local sign-out stands.
                try? await api.logout(token: sessionToken)
            case .deleteServerListening(let sessionToken):
                await settle(
                    {
                        try await api.deleteListening(token: sessionToken)
                        return .listeningDataDeleted(newDeviceId: UUID().uuidString)
                    },
                    failed: accountFailed)
            case .deleteServerAccount(let sessionToken):
                await settle(
                    {
                        try await api.deleteAccount(token: sessionToken)
                        return .accountDeleted(newDeviceId: UUID().uuidString)
                    },
                    failed: accountFailed)
            case .startPlayback, .pausePlayback, .setPlatformVolume, .persistSettings,
                 .persistListening, .clearSession, .persistAccount:
                // Not requests: `apply` handles these itself.
                break
            }
        }
    }

    /// Run one request and dispatch the command that settles it; `failed` is
    /// told whether the failure was an HTTP 401.
    private func settle(_ request: () async throws -> Command, failed: (Bool) -> Command) async {
        let outcome: Command
        do {
            outcome = try await request()
        } catch {
            outcome = failed((error as? SyncError)?.status == 401)
        }
        dispatch(outcome)
    }

    // MARK: - Tick loop

    private func startTicking(intervalMs: UInt64) {
        // Tell macOS this app is doing time-sensitive work — without this the
        // kernel can throttle our Timer to once-per-10-seconds when the main
        // window is closed and we're in the background.
        napGuard.begin(reason: "Cascade listening / timer")
        tickIntervalMs = intervalMs

        var last = Date()
        tickTimer?.invalidate()
        tickTimer = Timer.scheduledTimer(withTimeInterval: Double(intervalMs) / 1000.0, repeats: true) { [weak self] _ in
            Task { @MainActor [weak self] in
                guard let self else { return }
                let now = Date()
                let elapsedMs = UInt64(now.timeIntervalSince(last) * 1000)
                last = now
                self.dispatch(.tick(elapsedMs: elapsedMs))
                // Routine check: the core sends only once enough unsynced
                // time has accrued (ticks are the only thing that accrues it).
                self.sync(reason: .threshold)
            }
        }
        // Schedule on the common run-loop modes so menu interaction doesn't
        // pause the tick.
        if let timer = tickTimer {
            RunLoop.main.add(timer, forMode: .common)
        }
    }

    private func stopTicking() {
        tickTimer?.invalidate()
        tickTimer = nil
        tickIntervalMs = 0
        napGuard.end()
    }
}

extension Snapshot {
    static let empty = Snapshot(
        title: "Cascade",
        subtitle: "Loading…",
        isPlaying: false,
        volumePercent: 60,
        isMuted: false,
        outputGain: 0.36,
        primaryButtonLabel: "Play",
        timer: TimerSnapshot(
            kind: .off, remainingLabel: "", remainingMs: 0, totalMs: 0, progress: 0,
            isActive: false, statusLabel: "Paused"
        ),
        timerOptions: TimerOptions(
            focusPresets: [
                TimerPreset(minutes: 30, label: "30 min", shortLabel: "30m"),
                TimerPreset(minutes: 60, label: "1 hr", shortLabel: "1h"),
                TimerPreset(minutes: 480, label: "8 hr", shortLabel: "8h"),
            ],
            sleepPresets: [
                TimerPreset(minutes: 15, label: "15 min", shortLabel: "15m"),
                TimerPreset(minutes: 30, label: "30 min", shortLabel: "30m"),
                TimerPreset(minutes: 60, label: "1 hr", shortLabel: "1h"),
            ],
            minMinutes: 1,
            maxMinutes: 1440,
            customFocusMinutes: 30,
            customSleepMinutes: 30
        ),
        errorMessage: nil,
        listening: ListeningSnapshot(
            trackingEnabled: true,
            deviceTotalMs: 0,
            displayedTotalMs: 0,
            unsyncedMs: 0,
            totalLabel: "0m"
        ),
        account: AccountSnapshot(email: nil, signedInLabel: nil, statusLabel: nil, busy: false),
        tickIntervalMs: 0
    )
}
