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
    private static let tickIntervalMs: UInt64 = 250

    /// Bootstrap from disk. Failures fall back to defaults — the user never
    /// gets stuck on a startup error for something as trivial as malformed
    /// settings JSON.
    static func bootstrap() -> AppStore {
        let settings = SettingsStore()
        let json = settings.readSafely()
        let bridge = CoreBridge(persistedSettings: json)
        return AppStore(bridge: bridge, settings: settings)
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
        let prev = snapshot
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
            }
        }

        // Tick loop: run while a timer is active. We mirror the web app's
        // 250ms cadence so the countdown reads smoothly.
        let timerActive = update.snapshot.timer.kind == .sleep || update.snapshot.timer.kind == .pomodoro
        let wasActive = prev.timer.kind == .sleep || prev.timer.kind == .pomodoro
        if timerActive && !wasActive { startTicking() }
        if !timerActive && wasActive { stopTicking() }

        nowPlaying.update(snapshot: update.snapshot)
    }

    // MARK: - Tick loop

    private func startTicking() {
        // Tell macOS this app is doing time-sensitive work — without this the
        // kernel can throttle our Timer to once-per-10-seconds when the main
        // window is closed and we're in the background.
        napGuard.begin(reason: "Cascade focus / sleep timer")

        var last = Date()
        tickTimer?.invalidate()
        tickTimer = Timer.scheduledTimer(withTimeInterval: 0.25, repeats: true) { [weak self] _ in
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
        errorMessage: nil
    )
}
