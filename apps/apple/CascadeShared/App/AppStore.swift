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

    // Optional account + sync. The shell decides when it can talk (signed
    // in, a lifecycle trigger) and owns the HTTP; the core decides whether
    // there is anything to say, and what.
    private(set) var account: SyncAccount?
    private(set) var syncStatus: String?
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

    /// Bootstrap from disk. Failures fall back to defaults — the user never
    /// gets stuck on a startup error for something as trivial as malformed
    /// settings JSON.
    static func bootstrap() -> AppStore {
        let settings = SettingsStore()
        let json = settings.readSafely()
        let bridge = CoreBridge(persistedSettings: json)
        let store = AppStore(bridge: bridge, settings: settings)
        // Restore the listening ledger once at startup — always, even with no
        // blob, so the core can adopt a device id. The core ignores a
        // missing/incompatible blob and never lets a restore lower the counter.
        // The fallback id is the one this shell used to store itself (so an
        // existing server slot carries over), else a fresh one; the core only
        // adopts it if the blob carries no id.
        store.dispatch(.restoreListening(
            json: settings.readListeningSafely() ?? "",
            fallbackDeviceId: store.accountStore.legacyDeviceId() ?? UUID().uuidString))
        store.account = store.accountStore.readAccount()
        if store.account != nil {
            Task { await store.sync(reason: .refresh) }
        }
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

    /// Returns the effects of this dispatch, so a caller (sync) can read the
    /// answer to its own command synchronously.
    @discardableResult
    func dispatch(_ command: Command) -> [Effect] {
        do {
            let update = try bridge.dispatch(command)
            apply(update)
            return update.effects
        } catch {
            lastError = "\(error)"
            return []
        }
    }

    private func apply(_ update: Update) {
        snapshot = update.snapshot
        lastError = update.snapshot.errorMessage
        onSnapshotChanged?(update.snapshot)

        for effect in update.effects {
            switch effect {
            case .startPlayback(let volumePercent):
                audio.start(volumePercent: volumePercent)
                power.acquire()
                Task { @MainActor in dispatch(.platformPlaybackStarted) }
            case .pausePlayback:
                audio.pause()
                power.release()
            case .setPlatformVolume(let volumePercent):
                audio.setVolume(volumePercent: volumePercent)
            case .persistSettings(let json):
                settings.writeSafely(json)
            case .persistListening(let json):
                settings.writeListeningSafely(json)
            case .pushListening, .clearSession:
                // Answers to sync commands; `sync(reason:)` reads them off its
                // own dispatch.
                break
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

    /// Ask the core whether there is anything to send and, if so, PUT exactly
    /// what it says. The core owns the threshold, the payload, the device id,
    /// the in-flight guard, and the 401 rule; this only carries the request.
    func sync(reason: SyncReason) async {
        guard let account else { return }
        let effects = dispatch(.beginListeningSync(reason: reason))
        var push: (deviceId: String, deviceTotalMs: UInt64)?
        for case let .pushListening(deviceId, deviceTotalMs) in effects {
            push = (deviceId, deviceTotalMs)
        }
        // No effect: nothing worth sending, or a sync is already in flight.
        guard let push else { return }
        // Every begun sync must be settled, or the core never starts another.
        do {
            let res = try await syncApi.putListening(
                token: account.sessionToken,
                deviceId: push.deviceId,
                deviceTotalMs: Int64(clamping: push.deviceTotalMs))
            dispatch(.listeningSyncSucceeded(serverTotalMs: UInt64(max(0, res.serverTotalMs))))
        } catch {
            // Offline / transient failures retry on the next trigger; a 401
            // comes back as clearSession.
            let unauthorized = (error as? SyncError)?.status == 401
            let failure = dispatch(.listeningSyncFailed(unauthorized: unauthorized))
            if failure.contains(where: { if case .clearSession = $0 { return true } else { return false } }),
               self.account == account {
                self.account = nil
                accountStore.clearAccount()
                syncStatus = "Signed out — sign in again to sync."
            }
        }
    }

    func signIn(email: String) async {
        syncStatus = nil
        do {
            try await syncApi.requestLink(email: email)
            syncStatus = "Check \(email) for a sign-in link."
        } catch {
            syncStatus = "Couldn't send the sign-in link."
        }
    }

    /// Complete a magic-link sign-in from a pasted link (…/auth?token=XYZ) or a
    /// raw token. (Universal Links / a URL scheme are the on-device follow-up.)
    func completeSignIn(fromLinkOrToken input: String) async {
        guard let token = Self.extractToken(input) else {
            syncStatus = "Paste the full sign-in link."
            return
        }
        syncStatus = "Signing in…"
        do {
            let res = try await syncApi.verify(token: token)
            let acct = SyncAccount(sessionToken: res.sessionToken, email: res.email)
            accountStore.writeAccount(acct)
            account = acct
            syncStatus = "Signed in as \(res.email)."
            await sync(reason: .refresh)
        } catch {
            syncStatus = "That sign-in link was invalid or expired."
        }
    }

    /// Entry point for `.onOpenURL` once Universal Links are configured.
    func handleOpenURL(_ url: URL) {
        Task { await completeSignIn(fromLinkOrToken: url.absoluteString) }
    }

    func signOut() async {
        let previous = account
        account = nil
        accountStore.clearAccount()
        syncStatus = nil
        if let previous {
            try? await syncApi.logout(token: previous.sessionToken)
        }
    }

    func deleteListeningData() async {
        guard let account else { return }
        do {
            try await syncApi.deleteListening(token: account.sessionToken)
            // One dispatch rotates the device id and zeroes the ledger in a
            // single persisted write.
            dispatch(.resetListeningData(newDeviceId: UUID().uuidString))
            syncStatus = "Listening data deleted."
        } catch {
            syncStatus = "Couldn't delete listening data."
        }
    }

    func deleteAccount() async {
        guard let account else { return }
        do {
            try await syncApi.deleteAccount(token: account.sessionToken)
            dispatch(.resetListeningData(newDeviceId: UUID().uuidString))
            self.account = nil
            accountStore.clearAccount()
            syncStatus = "Account deleted."
        } catch {
            syncStatus = "Couldn't delete the account."
        }
    }

    private static func extractToken(_ input: String) -> String? {
        let s = input.trimmingCharacters(in: .whitespacesAndNewlines)
        if s.isEmpty { return nil }
        if let range = s.range(of: "token=") {
            let rest = s[range.upperBound...]
            if let amp = rest.firstIndex(of: "&") {
                return String(rest[..<amp])
            }
            return String(rest)
        }
        return s
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
                if self.account != nil {
                    await self.sync(reason: .threshold)
                }
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
        primaryButtonLabel: "Play",
        timer: TimerSnapshot(kind: .off, remainingLabel: "", remainingMs: 0, totalMs: 0, progress: 0),
        errorMessage: nil,
        listening: ListeningSnapshot(
            trackingEnabled: true,
            deviceTotalMs: 0,
            displayedTotalMs: 0,
            unsyncedMs: 0,
            totalLabel: "0m"
        ),
        tickIntervalMs: 0
    )
}
