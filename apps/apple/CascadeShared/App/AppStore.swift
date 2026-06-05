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
        // Restore the listening ledger once at startup. The core ignores a
        // missing/incompatible blob and never lets a restore lower the counter.
        if let listeningJson = settings.readListeningSafely() {
            store.dispatch(.restoreListening(json: listeningJson))
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

    func dispatch(_ command: Command) {
        do {
            let update = try bridge.dispatch(command)
            apply(update)
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
            }
        }

        // Tick loop: run while a timer is active (fine cadence, so the countdown
        // reads smoothly) and also while audio is simply playing (coarse cadence,
        // just to accrue listening time). Restart only when the cadence changes.
        let timerActive = update.snapshot.timer.kind == .sleep
            || update.snapshot.timer.kind == .pomodoro
            || update.snapshot.timer.kind == .stopwatch
        let desired: UInt64 = timerActive ? 250 : (update.snapshot.isPlaying ? 1000 : 0)
        if desired != tickIntervalMs {
            stopTicking()
            if desired > 0 { startTicking(intervalMs: desired) }
        }

        nowPlaying.update(snapshot: update.snapshot)
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
        )
    )
}
