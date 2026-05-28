# Perplexity Model Council Synthesis

### 1. Where Models Agree

| Finding | GPT-5.5 Thinking | Claude Opus 4.7 Thinking | Gemini 3.1 Pro Thinking | Evidence |
|---------|-----------|-----------|-----------|----------|
| iOS should be a **native SwiftUI app** (no web wrapper) | ✓ | ✓ | ✓ | SwiftUI lifecycle + native app expectation for platform-native shells. [ppl-ai-file-upload.s3.amazonaws](https://ppl-ai-file-upload.s3.amazonaws.com/web/direct-files/attachments/130224663/e826d6d9-002d-4f5c-8a8c-2b9f967bd923/clave-tech-stack.md) |
| Use iOS-native audio stack: **AVFoundation + AVAudioSession(.playback)** with **Background Audio** enabled | ✓ | ✓ | ✓ | Background audio requires enabling background mode and setting audio session category for playback. |
| Integrate lock-screen / Control Center controls via **MPNowPlayingInfoCenter + MPRemoteCommandCenter** | ✓ | ✓ | ✓ | MPNowPlayingInfoCenter is the Now Playing surface; remote commands are handled via MPRemoteCommandCenter. |
| Keep “app truth” in a single store/reducer and drive UI from a **snapshot**, executing **effects** via services | ✓ | ✓ | ✓ | SwiftUI is state-driven; the pattern matches your existing macOS “dispatch → snapshot/effects” approach in spirit. [ppl-ai-file-upload.s3.amazonaws](https://ppl-ai-file-upload.s3.amazonaws.com/web/direct-files/attachments/130224663/e826d6d9-002d-4f5c-8a8c-2b9f967bd923/clave-tech-stack.md) |
| Use **UNUserNotificationCenter** for “session complete” notifications (esp. if timer ends while app isn’t foreground) | ✓ | ✓ | ✓ | UNUserNotificationCenter manages local notifications. |

***

### 2. Where Models Disagree

| Topic | GPT-5.5 Thinking | Claude Opus 4.7 Thinking | Gemini 3.1 Pro Thinking | Why They Differ |
|-------|-----------|-----------|-----------|-----------------|
| Shared core: **Rust** vs **Swift-only** | Prefer Swift-only core unless you explicitly want Rust | Strongly recommends Rust `cascade-core` via **UniFFI XCFramework** | Recommends Rust via UniFFI XCFramework | GPT optimizes for simplicity given your remembered macOS Swift architecture; Claude/Gemini optimize for cross-platform parity consistent with the Clave “one Rust core” doctrine. [perplexity](https://www.perplexity.ai/search/b0a0932b-d65b-4c69-99a8-ac6d9b31cb49) |
| Audio engine choice | `AVAudioPlayer` for bundled MP3 looping | `AVPlayerLooper` (v1) → `AVAudioEngine` (gapless v2) | `AVQueuePlayer + AVPlayerLooper` for seamless looping | Different weighting of simplicity (AVAudioPlayer has `numberOfLoops`) vs guaranteed gapless looping and better system integration via AVPlayer/AVPlayerLooper. [perplexity](https://www.perplexity.ai/search/4d468959-772d-44bf-8478-3d47c2bb85ce) |
| iOS 17+/Observation vs ObservableObject | Doesn’t insist | Prefers iOS 17 + `@Observable` | Prefers modern `@Observable` | Claude/Gemini prioritize the newer Observation model; GPT keeps it compatible with a broader set / existing macOS approach. [ppl-ai-file-upload.s3.amazonaws](https://ppl-ai-file-upload.s3.amazonaws.com/web/direct-files/attachments/130224663/e826d6d9-002d-4f5c-8a8c-2b9f967bd923/clave-tech-stack.md) |
| Timer behavior when audio paused | Emphasizes local notifications + correction on resume | Similar, plus Live Activities as phase-2 | Similar | All agree BGTaskScheduler isn’t a reliable “exact timer”; disagreement is mostly how aggressively to add iOS-specific UX (Live Activities). BG tasks timing isn’t guaranteed. [perplexity](https://www.perplexity.ai/search/c38a5812-9eae-481c-bffe-3b19e90dbbcf) |

***

### 4. Comprehensive Analysis

**High-Confidence Findings.** Cascade on iOS should be a small, calm **SwiftUI** app that primarily does three things reliably: (1) loop the waterfall audio, (2) keep playing when the phone locks/backgrounds, and (3) expose native playback controls on the lock screen / Control Center. Achieving (1) and (2) requires correct **AVAudioSession** configuration—specifically using a playback-oriented category and enabling the **Background Audio** capability so the system allows ongoing audio when the app isn’t foreground. All three models also converge that iOS should integrate with Apple’s “Now Playing” stack: **MPNowPlayingInfoCenter** for metadata and **MPRemoteCommandCenter** for play/pause from the system UI and headset buttons. For timers (30/60/8h/custom), use **UNUserNotificationCenter** to notify the user at session completion, and always correct state on resume by comparing `Date()` to `endsAt` rather than relying on background execution to be perfectly punctual. [perplexity](https://www.perplexity.ai/search/c38a5812-9eae-481c-bffe-3b19e90dbbcf)

**Areas of Divergence.** The main architectural fork is whether Cascade iOS should consume a shared **Rust `cascade-core`**. Claude Opus 4.7 and Gemini 3.1 both recommend following the Clave-style doctrine: compile Rust to an **XCFramework** and generate Swift bindings via **UniFFI**, so iOS uses the same `dispatch(command) -> {snapshot, effects}` contract as your other clients. GPT-5.5, informed by your remembered macOS architecture being Swift-native, suggests *not* adding Rust unless you want Cascade to explicitly rehearse Clave’s cross-platform architecture. Practically: if Cascade is a single-purpose utility, a Swift-only core is faster; if Cascade is also a “practice run” for the multi-platform Rust-core approach, then UniFFI-on-iOS is the right investment. [perplexity](https://www.perplexity.ai/search/b0a0932b-d65b-4c69-99a8-ac6d9b31cb49)

Audio engine choice also differs. GPT-5.5 recommends **AVAudioPlayer** because it loops locally with `numberOfLoops = -1` and is minimal surface area. Claude/Gemini lean toward **AVPlayerLooper** (`AVQueuePlayer` + `AVPlayerLooper`) and optionally **AVAudioEngine** later for truly gapless looping. This is a tradeoff: AVAudioPlayer is simpler; AVPlayerLooper is often the more “media-app” idiom and can reduce audible seams at loop boundaries, especially if you later swap in a lossless asset. [perplexity](https://www.perplexity.ai/search/4d468959-772d-44bf-8478-3d47c2bb85ce)

**Unique Insights Worth Noting.** Claude’s suggestion to add **Live Activities / Dynamic Island** as phase-2 polish is uniquely iOS-native and particularly compelling for long sessions (8 hours): it makes “remaining time” visible without opening the app. Gemini’s emphasis that background audio keeps your process alive *while audio is playing* is important: your timer ticks can be driven in-process during playback, but when audio is paused, you must fall back to scheduled notifications and state correction (since iOS will suspend you).

**Recommendations.** Pick one of two clear architectures:

1) **Cross-platform “Clave rehearsal” (recommended if you want parity):**  
SwiftUI shell + UniFFI-generated Swift bindings to Rust `cascade-core` packaged as an XCFramework. SwiftUI views render from `AppSnapshot`; an `EffectExecutor` owns AVAudioSession, playback (AVPlayerLooper or AVAudioPlayer), Now Playing, notifications. [perplexity](https://www.perplexity.ai/search/b0a0932b-d65b-4c69-99a8-ac6d9b31cb49)

2) **Fastest-to-ship utility:**  
SwiftUI + pure Swift reducer/store mirroring your macOS pattern; same services (AVAudioSession, Now Playing, notifications), no Rust.

Either way, implement: Background Audio capability + `.playback` session category, MPNowPlayingInfoCenter/MPRemoteCommandCenter, and UNUserNotificationCenter for session completion.

# GPT 5.5 Thinking

Cascade iOS Architecture
Recommendation
Build Cascade for iOS as a small native SwiftUI audio utility:

text
Cascade iOS
= SwiftUI app shell
+ AVFoundation playback service
+ AVAudioSession background audio
+ MPNowPlayingInfoCenter / MPRemoteCommandCenter
+ UNUserNotificationCenter for session-complete notifications
+ same reducer-style app model as macOS:
  dispatch(action) -> { snapshot, effects }
Since Cascade is a daily-use looping waterfall/noise app, I would not overcomplicate iOS with a Rust core unless you specifically want Cascade to rehearse the Clave architecture. Your remembered macOS architecture already uses SwiftUI, an AVAudioPlayer-based playback manager, a state machine, settings, and a dispatch(action) -> {snapshot, effects} pattern. The iOS version should share that shape almost exactly, with iOS-specific services for background audio, lock-screen controls, notifications, haptics, and app lifecycle.

The key principle:

The state machine owns Cascade’s truth; iOS owns Cascade’s iOS-ness.

1. Product Shape on iOS
Cascade iOS should feel like a “set it and forget it” sound utility:

launch app

tap Play

waterfall loops indefinitely

optional 30 min / 60 min / 8 hr / custom timer

phone locks, audio continues

lock screen shows Cascade in Now Playing

headphone / Control Center play-pause works

when timer expires, audio stops and optional notification appears

settings persist across launches

Primary use case:

text
Open Cascade → Start waterfall → Lock phone → live life
So the iOS architecture should prioritize:

reliable background playback

low battery impact

simple state restoration

native lock-screen controls

minimal UI surface

2. Recommended Stack
Layer	Choice
UI	SwiftUI
App lifecycle	SwiftUI App + @UIApplicationDelegateAdaptor for app delegate hooks
State	@MainActor ObservableObject store
Audio	AVAudioPlayer for bundled local MP3
Background audio	AVAudioSession.Category.playback + UIBackgroundModes = audio
Lock-screen / Control Center	MPNowPlayingInfoCenter + MPRemoteCommandCenter
Notifications	UNUserNotificationCenter
Persistence	JSON settings file in Application Support or UserDefaults for tiny preferences
Architecture pattern	reducer/state machine + platform effects
Testing	reducer tests + playback service smoke tests
For Cascade’s one local 13-minute MP3 loop, AVAudioPlayer is a better fit than AVPlayer: it is simple, local-file-oriented, has direct volume, duration/current-time APIs, and supports infinite looping through numberOfLoops = -1.
 Background audio on iOS requires enabling the audio background mode and configuring the audio session for playback.

3. Repo Layout
If Cascade currently has macOS and perhaps Windows/web/android folders, I’d structure iOS like this:

text
cascade/
├── apps/
│   ├── cascade-macos/
│   ├── cascade-ios/
│   │   ├── CascadeiOS.xcodeproj
│   │   └── CascadeiOS/
│   │       ├── CascadeiOSApp.swift
│   │       ├── AppDelegate.swift
│   │       ├── Info.plist
│   │       │
│   │       ├── Assets.xcassets/
│   │       ├── Audio/
│   │       │   └── waterfall.mp3
│   │       │
│   │       ├── App/
│   │       │   ├── CascadeStore.swift
│   │       │   ├── CascadeState.swift
│   │       │   ├── CascadeAction.swift
│   │       │   ├── CascadeEffect.swift
│   │       │   ├── CascadeReducer.swift
│   │       │   └── CascadeSnapshot.swift
│   │       │
│   │       ├── Services/
│   │       │   ├── EffectExecutor.swift
│   │       │   ├── PlaybackService.swift
│   │       │   ├── AudioSessionService.swift
│   │       │   ├── NowPlayingService.swift
│   │       │   ├── RemoteCommandService.swift
│   │       │   ├── SettingsStore.swift
│   │       │   ├── NotificationService.swift
│   │       │   ├── TimerService.swift
│   │       │   └── HapticsService.swift
│   │       │
│   │       ├── Views/
│   │       │   ├── RootView.swift
│   │       │   ├── PlayerView.swift
│   │       │   ├── TimerPresetBar.swift
│   │       │   ├── VolumeSlider.swift
│   │       │   ├── SettingsView.swift
│   │       │   └── Components/
│   │       │
│   │       └── Tests/
│   │           ├── CascadeReducerTests.swift
│   │           └── SettingsStoreTests.swift
If you later decide to share code between macOS and iOS, split the pure Swift state machine into a package:

text
packages/
└── CascadeCoreSwift/
    ├── Sources/
    │   └── CascadeCoreSwift/
    │       ├── CascadeState.swift
    │       ├── CascadeAction.swift
    │       ├── CascadeEffect.swift
    │       └── CascadeReducer.swift
    └── Tests/
Then:

text
cascade-ios imports CascadeCoreSwift
cascade-macos imports CascadeCoreSwift
That gives you reuse without Rust/FFI complexity.

4. Core State Pattern
Keep the model identical to macOS:

swift
Action -> Reducer -> Snapshot + Effects -> Services
Example:

swift
enum CascadeAction: Equatable {
    case appLaunched
    case settingsLoaded(CascadeSettings)
    case playTapped
    case pauseTapped
    case togglePlayback
    case setVolume(Double)
    case startTimer(TimerPreset)
    case cancelTimer
    case tick(now: Date)

    case playbackStarted
    case playbackPaused
    case playbackFailed(String)

    case audioInterruptedBegan
    case audioInterruptedEnded(shouldResume: Bool)

    case remotePlayCommand
    case remotePauseCommand
    case remoteToggleCommand
}
swift
struct CascadeState: Equatable {
    var isPlaying: Bool
    var volume: Double
    var selectedTimer: TimerPreset?
    var sessionStartedAt: Date?
    var sessionEndsAt: Date?
    var lastError: String?
    var settings: CascadeSettings
}
swift
struct CascadeUpdate {
    var snapshot: CascadeSnapshot
    var effects: [CascadeEffect]
}
swift
enum CascadeEffect: Equatable {
    case configureAudioSession
    case startPlayback(volume: Double, loopForever: Bool)
    case pausePlayback
    case setPlaybackVolume(Double)

    case persistSettings(CascadeSettings)
    case updateNowPlaying(NowPlayingState)
    case registerRemoteCommands
    case scheduleTimerFinishedNotification(Date)
    case cancelTimerNotification
    case haptic(HapticKind)
}
The reducer should be pure:

swift
struct CascadeReducer {
    mutating func dispatch(
        state: inout CascadeState,
        action: CascadeAction,
        now: Date = Date()
    ) -> CascadeUpdate {
        // update state
        // emit effects
    }
}
Or better: pass now in explicitly and do not call Date() inside the reducer. That keeps it deterministic and testable.

5. App Store / Entitlements Setup
In Xcode, enable:

text
Signing & Capabilities
└── Background Modes
    └── Audio, AirPlay, and Picture in Picture
This adds:

xml
<key>UIBackgroundModes</key>
<array>
    <string>audio</string>
</array>
iOS background audio requires both the background audio capability and an audio session category appropriate for playback.

You probably do not need:

Background Fetch

BGTaskScheduler

Push Notifications

Location

Bluetooth

HealthKit

iCloud

Cascade is not a background-processing app. It is a foreground-created audio playback session that continues because audio is actively playing.

Do not rely on BGTaskScheduler for timers. iOS background tasks are opportunistic and not guaranteed to run at an exact time.
 For session expiry, use one of these patterns:

while audio is playing: app process can continue under background audio mode, so your timer can stop playback

for user-facing completion: schedule a local notification at the session end

on resume: compare Date() against sessionEndsAt and immediately correct state

6. Audio Session Service
Create a dedicated service:

swift
import AVFoundation

final class AudioSessionService {
    func configure() throws {
        let session = AVAudioSession.sharedInstance()

        try session.setCategory(
            .playback,
            mode: .default,
            options: []
        )

        try session.setActive(true)
    }
}
Use .playback because Cascade’s sound is the main purpose of the app and should continue when the screen locks.

Potential options:

swift
options: [.mixWithOthers]
But I would not enable .mixWithOthers by default. Cascade is itself the background sound. Let users choose later:

text
[ ] Mix with other audio
If enabled, configure:

swift
try session.setCategory(.playback, mode: .default, options: [.mixWithOthers])
Possible settings:

Setting	Audio Session
Default	.playback, no mix
“Mix with other audio”	.playback, .mixWithOthers
“Duck other audio”	.playback, .duckOthers
7. Playback Service
Use AVAudioPlayer.

swift
import AVFoundation

@MainActor
final class PlaybackService: NSObject, AVAudioPlayerDelegate {
    private var player: AVAudioPlayer?

    var isPlaying: Bool {
        player?.isPlaying ?? false
    }

    func prepareIfNeeded() throws {
        if player != nil { return }

        guard let url = Bundle.main.url(
            forResource: "waterfall",
            withExtension: "mp3"
        ) else {
            throw PlaybackError.assetMissing
        }

        let player = try AVAudioPlayer(contentsOf: url)
        player.numberOfLoops = -1
        player.prepareToPlay()
        player.delegate = self

        self.player = player
    }

    func play(volume: Double) throws {
        try prepareIfNeeded()
        player?.volume = Float(volume)
        player?.numberOfLoops = -1
        player?.play()
    }

    func pause() {
        player?.pause()
    }

    func stopAndRewind() {
        player?.stop()
        player?.currentTime = 0
    }

    func setVolume(_ volume: Double) {
        player?.volume = Float(volume)
    }
}

enum PlaybackError: Error {
    case assetMissing
}
For infinite local looping, AVAudioPlayer.numberOfLoops = -1 is the key API.

8. Audio Interruptions
Handle interruptions from phone calls, Siri, alarms, route changes, and Bluetooth disconnects.

swift
final class AudioInterruptionService {
    private let dispatch: (CascadeAction) -> Void

    init(dispatch: @escaping (CascadeAction) -> Void) {
        self.dispatch = dispatch

        NotificationCenter.default.addObserver(
            self,
            selector: #selector(handleInterruption),
            name: AVAudioSession.interruptionNotification,
            object: AVAudioSession.sharedInstance()
        )
    }

    @objc private func handleInterruption(_ notification: Notification) {
        guard
            let info = notification.userInfo,
            let typeValue = info[AVAudioSessionInterruptionTypeKey] as? UInt,
            let type = AVAudioSession.InterruptionType(rawValue: typeValue)
        else { return }

        switch type {
        case .began:
            dispatch(.audioInterruptedBegan)

        case .ended:
            let optionsValue =
                info[AVAudioSessionInterruptionOptionKey] as? UInt ?? 0
            let options = AVAudioSession.InterruptionOptions(rawValue: optionsValue)

            dispatch(.audioInterruptedEnded(
                shouldResume: options.contains(.shouldResume)
            ))

        @unknown default:
            break
        }
    }
}
Apple exposes interruption notifications through AVAudioSession.interruptionNotification; the common pattern is to observe that notification and update playback accordingly.

Reducer behavior:

text
interruption began:
  - mark wasPlayingBeforeInterruption = isPlaying
  - set isPlaying = false
  - update Now Playing

interruption ended:
  - if wasPlayingBeforeInterruption && shouldResume:
      emit startPlayback
  - else stay paused
9. Now Playing + Remote Commands
This is critical for iOS.

Cascade should appear on:

Lock Screen

Control Center

Dynamic Island / Live Activities? maybe later

headphone controls

car controls

Bluetooth speaker controls

Use:

swift
import MediaPlayer
NowPlayingService
swift
final class NowPlayingService {
    func update(_ state: NowPlayingState) {
        var info: [String: Any] = [
            MPMediaItemPropertyTitle: "Cascade",
            MPMediaItemPropertyArtist: "Waterfall",
            MPNowPlayingInfoPropertyPlaybackRate: state.isPlaying ? 1.0 : 0.0
        ]

        if let duration = state.duration {
            info[MPMediaItemPropertyPlaybackDuration] = duration
        }

        info[MPNowPlayingInfoPropertyElapsedPlaybackTime] =
            state.elapsedTime

        MPNowPlayingInfoCenter.default().nowPlayingInfo = info
    }

    func clear() {
        MPNowPlayingInfoCenter.default().nowPlayingInfo = nil
    }
}

struct NowPlayingState: Equatable {
    var isPlaying: Bool
    var duration: TimeInterval?
    var elapsedTime: TimeInterval
}
MPNowPlayingInfoCenter manages Now Playing metadata.

RemoteCommandService
swift
final class RemoteCommandService {
    private let dispatch: (CascadeAction) -> Void

    init(dispatch: @escaping (CascadeAction) -> Void) {
        self.dispatch = dispatch
    }

    func register() {
        let center = MPRemoteCommandCenter.shared()

        center.playCommand.isEnabled = true
        center.pauseCommand.isEnabled = true
        center.togglePlayPauseCommand.isEnabled = true

        center.playCommand.addTarget { [weak self] _ in
            self?.dispatch(.remotePlayCommand)
            return .success
        }

        center.pauseCommand.addTarget { [weak self] _ in
            self?.dispatch(.remotePauseCommand)
            return .success
        }

        center.togglePlayPauseCommand.addTarget { [weak self] _ in
            self?.dispatch(.remoteToggleCommand)
            return .success
        }

        center.nextTrackCommand.isEnabled = false
        center.previousTrackCommand.isEnabled = false
        center.changePlaybackPositionCommand.isEnabled = false
    }
}
Use MPRemoteCommandCenter for play/pause/toggle commands and MPNowPlayingInfoCenter to keep system UI synchronized with app state.

You can also call:

swift
UIApplication.shared.beginReceivingRemoteControlEvents()
This starts delivery of remote control events through the responder chain.
 With MPRemoteCommandCenter, you may not need to manually handle responder-chain events, but calling it during app launch is harmless for an audio app.

10. Store + Effect Executor
The store coordinates the reducer and services.

swift
@MainActor
final class CascadeStore: ObservableObject {
    @Published private(set) var snapshot: CascadeSnapshot

    private var state: CascadeState
    private let reducer: CascadeReducer
    private let effects: EffectExecutor

    init(
        initialState: CascadeState,
        reducer: CascadeReducer,
        effects: EffectExecutor
    ) {
        self.state = initialState
        self.reducer = reducer
        self.effects = effects
        self.snapshot = CascadeSnapshot(state: initialState)
    }

    func dispatch(_ action: CascadeAction) {
        var reducer = reducer

        let update = reducer.dispatch(
            state: &state,
            action: action,
            now: Date()
        )

        snapshot = update.snapshot

        Task {
            await effects.execute(update.effects) { [weak self] action in
                await MainActor.run {
                    self?.dispatch(action)
                }
            }
        }
    }
}
Effect executor:

swift
@MainActor
final class EffectExecutor {
    let audioSession: AudioSessionService
    let playback: PlaybackService
    let nowPlaying: NowPlayingService
    let remoteCommands: RemoteCommandService
    let settings: SettingsStore
    let notifications: NotificationService
    let haptics: HapticsService

    func execute(
        _ effects: [CascadeEffect],
        dispatch: @escaping (CascadeAction) async -> Void
    ) async {
        for effect in effects {
            switch effect {
            case .configureAudioSession:
                do {
                    try audioSession.configure()
                } catch {
                    await dispatch(.playbackFailed(error.localizedDescription))
                }

            case let .startPlayback(volume, loopForever):
                do {
                    try playback.play(volume: volume)
                    await dispatch(.playbackStarted)
                } catch {
                    await dispatch(.playbackFailed(error.localizedDescription))
                }

            case .pausePlayback:
                playback.pause()
                await dispatch(.playbackPaused)

            case let .setPlaybackVolume(volume):
                playback.setVolume(volume)

            case let .persistSettings(settingsValue):
                try? settings.save(settingsValue)

            case let .updateNowPlaying(state):
                nowPlaying.update(state)

            case .registerRemoteCommands:
                remoteCommands.register()

            case let .scheduleTimerFinishedNotification(date):
                await notifications.scheduleTimerFinished(at: date)

            case .cancelTimerNotification:
                notifications.cancelTimerFinished()

            case let .haptic(kind):
                haptics.play(kind)
            }
        }
    }
}
The important rule:

text
Views dispatch actions.
Reducer emits effects.
Services execute effects.
Views never call AVAudioPlayer directly.
11. SwiftUI App Entry
swift
@main
struct CascadeiOSApp: App {
    @UIApplicationDelegateAdaptor(AppDelegate.self)
    var appDelegate

    @StateObject private var store = AppFactory.makeStore()

    @Environment(\.scenePhase)
    private var scenePhase

    var body: some Scene {
        WindowGroup {
            RootView()
                .environmentObject(store)
                .onAppear {
                    store.dispatch(.appLaunched)
                }
                .onChange(of: scenePhase) { _, newPhase in
                    switch newPhase {
                    case .active:
                        store.dispatch(.tick(now: Date()))
                    case .background:
                        store.dispatch(.tick(now: Date()))
                    case .inactive:
                        break
                    @unknown default:
                        break
                    }
                }
        }
    }
}
Use @UIApplicationDelegateAdaptor when you need UIKit lifecycle hooks in a SwiftUI app.

AppDelegate can handle notification delegate setup and remote-control setup:

swift
final class AppDelegate: NSObject, UIApplicationDelegate {
    func application(
        _ application: UIApplication,
        didFinishLaunchingWithOptions launchOptions:
            [UIApplication.LaunchOptionsKey: Any]? = nil
    ) -> Bool {
        UNUserNotificationCenter.current().delegate = NotificationDelegate.shared
        UIApplication.shared.beginReceivingRemoteControlEvents()
        return true
    }
}
12. Main UI
The UI should be almost comically simple.

swift
struct PlayerView: View {
    @EnvironmentObject var store: CascadeStore

    var body: some View {
        VStack(spacing: 32) {
            Spacer()

            Image(systemName: "water.waves")
                .font(.system(size: 72))
                .foregroundStyle(.blue)

            Text("Cascade")
                .font(.largeTitle.bold())

            Text(store.snapshot.statusText)
                .foregroundStyle(.secondary)

            Button {
                store.dispatch(.togglePlayback)
            } label: {
                Image(systemName: store.snapshot.isPlaying ? "pause.fill" : "play.fill")
                    .font(.system(size: 44))
                    .frame(width: 96, height: 96)
            }
            .buttonStyle(.borderedProminent)
            .clipShape(Circle())

            TimerPresetBar(
                selected: store.snapshot.selectedTimer,
                onSelect: { preset in
                    store.dispatch(.startTimer(preset))
                },
                onCancel: {
                    store.dispatch(.cancelTimer)
                }
            )

            VolumeSlider(
                value: store.snapshot.volume,
                onChange: { volume in
                    store.dispatch(.setVolume(volume))
                }
            )

            Spacer()
        }
        .padding()
    }
}
Navigation:

swift
struct RootView: View {
    var body: some View {
        NavigationStack {
            PlayerView()
                .toolbar {
                    NavigationLink {
                        SettingsView()
                    } label: {
                        Image(systemName: "gear")
                    }
                }
        }
    }
}
For iOS design, use:

big central play/pause

large tap targets

low visual noise

system colors

subtle haptics

no “desktop-like” dense controls

13. Timer Semantics
State:

swift
enum TimerPreset: Equatable, Codable {
    case infinite
    case minutes30
    case minutes60
    case hours8
    case custom(minutes: Int)
}
Reducer behavior:

text
Start timer:
  - set sessionStartedAt = now
  - set sessionEndsAt = now + preset duration
  - emit scheduleTimerFinishedNotification(endDate)
  - if not playing, emit startPlayback

Tick:
  - if sessionEndsAt != nil && now >= sessionEndsAt:
      set isPlaying = false
      clear session
      emit pausePlayback
      emit updateNowPlaying
      emit haptic/sessionFinished
Timer service:

swift
final class TimerService {
    private var timer: Timer?

    func start(dispatch: @escaping () -> Void) {
        timer?.invalidate()

        timer = Timer.scheduledTimer(
            withTimeInterval: 1,
            repeats: true
        ) { _ in
            dispatch()
        }
    }

    func stop() {
        timer?.invalidate()
        timer = nil
    }
}
But don’t trust Timer alone in the background. Always compare Date() to sessionEndsAt on:

app active

app background

interruption ended

playback started

periodic tick

14. Local Notifications
Use notifications only for timer completion, and only after permission.

swift
import UserNotifications

final class NotificationService: NSObject {
    func requestAuthorizationIfNeeded() async {
        _ = try? await UNUserNotificationCenter.current()
            .requestAuthorization(options: [.alert, .sound])
    }

    func scheduleTimerFinished(at date: Date) async {
        let content = UNMutableNotificationContent()
        content.title = "Cascade"
        content.body = "Your Cascade session is complete."
        content.sound = nil

        let triggerDate = Calendar.current.dateComponents(
            [.year, .month, .day, .hour, .minute, .second],
            from: date
        )

        let trigger = UNCalendarNotificationTrigger(
            dateMatching: triggerDate,
            repeats: false
        )

        let request = UNNotificationRequest(
            identifier: "cascade.timer.finished",
            content: content,
            trigger: trigger
        )

        try? await UNUserNotificationCenter.current().add(request)
    }

    func cancelTimerFinished() {
        UNUserNotificationCenter.current()
            .removePendingNotificationRequests(
                withIdentifiers: ["cascade.timer.finished"]
            )
    }
}
UNUserNotificationCenter is the central API for managing local notification behavior.

In foreground, iOS may suppress notification UI unless you implement the notification center delegate and specify how to present it.

15. Settings
For Cascade, settings are tiny:

swift
struct CascadeSettings: Codable, Equatable {
    var defaultVolume: Double = 0.7
    var defaultTimer: TimerPreset = .infinite
    var continueAfterLaunch: Bool = false
    var mixWithOtherAudio: Bool = false
    var showTimerNotifications: Bool = true
    var hapticsEnabled: Bool = true
}
For the cleanest cross-platform and macOS parity, store as JSON:

swift
final class SettingsStore {
    private var url: URL {
        let dir = FileManager.default.urls(
            for: .applicationSupportDirectory,
            in: .userDomainMask
        )[0].appendingPathComponent("Cascade", isDirectory: true)

        try? FileManager.default.createDirectory(
            at: dir,
            withIntermediateDirectories: true
        )

        return dir.appendingPathComponent("settings.json")
    }

    func load() throws -> CascadeSettings {
        let data = try Data(contentsOf: url)
        return try JSONDecoder().decode(CascadeSettings.self, from: data)
    }

    func save(_ settings: CascadeSettings) throws {
        let data = try JSONEncoder().encode(settings)
        try data.write(to: url, options: [.atomic])
    }
}
You could use @AppStorage / UserDefaults for tiny values, but a JSON file keeps parity with macOS/Windows and makes migration/versioning explicit.

16. iOS-Specific Lifecycle Rules
Handle these cases deliberately:

App launched
text
load settings
configure audio session
register remote commands
render initial snapshot
App enters background while playing
text
do nothing special
audio continues because background audio mode + playback session
App enters background while not playing
text
no background work
no fake keepalive
User force-quits app
iOS will not keep your app running. Do not try to fight this. On next launch:

text
load previous settings
do not auto-play unless explicit setting allows it
Device reboot
Same: no guaranteed background task after reboot. Restore cleanly on launch.

Audio route changes
Optional v1.5 feature:

headphones disconnected → pause?

AirPods switched → continue?

Bluetooth speaker disconnected → pause?

For v1, observe route changes and pause only if the output disappears unexpectedly.

17. App Icon / Branding
For iOS:

simple blue/white water icon

display name: Cascade

bundle id: page.stephens.cascade

audio metadata title: Cascade

artist/subtitle: Waterfall

Now Playing metadata:

text
Title: Cascade
Artist: Waterfall
Album: Background Noise
Optional lock-screen artwork:

a simple generated waterfall gradient

do not use a busy image; the app’s calmness matters

18. Testing Plan
Unit tests
Reducer tests:

text
playTapped emits configureAudioSession + startPlayback
pauseTapped emits pausePlayback
setVolume persists settings + updates playback volume
startTimer schedules notification
tick after sessionEndsAt pauses playback
interruption began pauses logical playback
interruption ended resumes only if shouldResume and was playing
Manual device tests
Must test on real iPhone, not just simulator:

Play, lock phone, verify audio continues.

Pause from lock screen.

Resume from Control Center.

Use AirPods play/pause.

Start 30 min timer, background app, verify stop/notification.

Receive phone call / Siri interruption.

Disconnect Bluetooth headphones.

Kill app manually; verify expected behavior.

Enable Low Power Mode; verify audio still works.

Test with silent switch on; .playback should still play.

19. Implementation Phases
Phase I1 — Skeleton
text
SwiftUI project
bundle waterfall.mp3
PlayerView
Play/Pause button
Volume slider
AVAudioPlayer loops forever
Phase I2 — Background Audio
text
Background Modes: Audio
AVAudioSession .playback
lock phone test
interruption observer
Phase I3 — State Machine
text
CascadeStore
CascadeReducer
Snapshot/Effect pattern
PlaybackService behind effects
SettingsStore JSON
Phase I4 — Timers
text
30 min / 60 min / 8 hr
custom timer
tick correction via Date
local notification on completion
Phase I5 — Now Playing
text
MPNowPlayingInfoCenter metadata
MPRemoteCommandCenter play/pause/toggle
lock screen / Control Center test
Phase I6 — Polish
text
haptics
settings screen
route change handling
artwork
accessibility labels
App Store screenshots
20. Final Architecture
text
CascadeiOSApp
 ├─ AppDelegate
 │   ├─ notification delegate
 │   └─ remote control event setup
 │
 ├─ CascadeStore (@MainActor ObservableObject)
 │   ├─ CascadeState
 │   ├─ CascadeReducer
 │   └─ CascadeSnapshot
 │
 ├─ SwiftUI Views
 │   ├─ RootView
 │   ├─ PlayerView
 │   ├─ TimerPresetBar
 │   ├─ VolumeSlider
 │   └─ SettingsView
 │
 └─ Services
     ├─ EffectExecutor
     ├─ AudioSessionService
     ├─ PlaybackService        AVAudioPlayer
     ├─ NowPlayingService      MPNowPlayingInfoCenter
     ├─ RemoteCommandService   MPRemoteCommandCenter
     ├─ NotificationService    UNUserNotificationCenter
     ├─ SettingsStore          settings.json
     ├─ TimerService
     └─ HapticsService
Build it native, small, and boring. For Cascade, boring is good: the app should disappear into the background and just keep the waterfall running.

# Opus 4.7 Thinking

Cascade — iOS Architecture (SwiftUI + Rust core)
The iOS version of Cascade should be the Apple-native shell over the same headless cascade-core Rust crate that powers the web app, Android APK, and (planned) Windows client. Per your Clave tech stack, iOS must be genuinely native: SwiftUI + Swift, consuming the core through UniFFI-generated Swift bindings packaged as an XCFramework
 — no Capacitor, no React Native, no WebView wrapper around cascade.stephens.page.

Note: I couldn't fetch the GitHub repo or the deployed web app in this environment, and the APK isn't directly inspectable as a tool-readable artifact. This architecture is anchored on (1) your existing Cascade decisions for web/Android/macOS/Windows, (2) the Clave tech stack doc
, and (3) current Apple platform references.

1. Platform Decisions
Concern	Choice	Rationale
UI framework	SwiftUI	Mandated by your Clave stack
; modern, declarative, shares a large fraction of views with macOS via a CascadeShared SwiftPM package.
Language	Swift 6 (strict concurrency on)	Latest stable; first-class actors and Sendable checking pair well with a single-threaded core.
Min iOS	iOS 17	Required for .observable macros, modern MenuBarExtra-equivalent gestures, robust AVAudioSession lifecycle, and Observation framework.
Pattern	MV (Observable) over MVVM, with thin @Observable view models	iOS 17's Observation framework removes the ceremony of ObservableObject/@Published and matches Cascade's "snapshot-driven UI" model perfectly.
Core access	UniFFI Swift bindings built into an XCFramework (covers aarch64-apple-ios, aarch64-apple-ios-sim, x86_64-apple-ios)	Direct quote from Clave doc.
Audio engine	AVAudioEngine with a looping AVAudioPlayerNode (preferred) or AVPlayer with AVPlayerLooper (simpler v1)	Native iOS audio, gapless looping, full AVAudioSession lifecycle, and free integration with Now Playing/Control Center.
Background audio	AVAudioSession category .playback + UIBackgroundModes: audio	Standard iOS pattern — keeps audio playing when the screen locks or the app is backgrounded, which is essential for a focus/prayer session app.
Now Playing / lock screen	MPNowPlayingInfoCenter + MPRemoteCommandCenter	iOS-native equivalent of Windows SMTC and Android MediaSession; gives lock-screen controls, AirPods squeeze, CarPlay, and Siri "pause Cascade."
Local multiplayer	N/A for Cascade (Clave-only concern)	Cascade is single-user; ignore Multipeer Connectivity here.
Settings persistence	Same settings.json schema as other clients, in FileManager.default.urls(for: .applicationSupportDirectory)/Cascade/settings.json	Single Rust-owned schema across all five platforms.
Audio asset	cascade.mp3 (and optional cascade.caf for true gapless) bundled in the app	Same source asset as Android/macOS/Windows; CAF/AIFF if you want sample-accurate looping.
Distribution	TestFlight first, then App Store	Bypasses your household whitelisting friction nicely (TestFlight installs are first-class).
Bundle ID	page.stephens.cascade	Reverse-DNS of your domain; matches macOS/Windows.
Live Activities	Phase 2 polish via ActivityKit	Show remaining session time on the lock screen / Dynamic Island.
2. Repo Layout — iOS Slice
Following the apps/cascade-apple/ shape from your Clave stack
:

text
cascade/
├── crates/
│   └── cascade-core/                     # unchanged Rust core
├── bindings/
│   └── cascade-uniffi/
│       ├── cascade.udl                   # shared with Kotlin/C#
│       └── src/lib.rs
├── apps/
│   ├── cascade-web/
│   ├── cascade-android/                  # the APK
│   ├── cascade-windows/
│   └── cascade-apple/                    # ← iOS + macOS share this tree
│       ├── Cascade.xcworkspace
│       ├── CascadeShared/                # SwiftPM package (views/VMs/services that work on both)
│       │   ├── Package.swift
│       │   └── Sources/CascadeShared/
│       │       ├── Core/
│       │       │   ├── CoreBridge.swift          # wraps generated UniFFI Swift API
│       │       │   ├── EffectExecutor.swift
│       │       │   └── AppViewModel.swift        # @Observable root
│       │       ├── Views/
│       │       │   ├── MainView.swift
│       │       │   ├── PlayPauseButton.swift
│       │       │   ├── VolumeSlider.swift
│       │       │   ├── TimerPresetBar.swift
│       │       │   └── SettingsView.swift
│       │       └── Services/
│       │           ├── AudioEngine.swift          # AVAudioEngine wrapper
│       │           ├── NowPlayingController.swift
│       │           ├── SettingsStore.swift
│       │           └── TickScheduler.swift
│       ├── CascadeiOS/
│       │   ├── CascadeiOSApp.swift
│       │   ├── Info.plist                         # UIBackgroundModes: audio, NSMicrophoneUsage NO
│       │   ├── Cascade.entitlements
│       │   ├── ContentView.swift                  # composes CascadeShared.MainView
│       │   ├── LiveActivity/
│       │   │   ├── CascadeActivityAttributes.swift
│       │   │   └── CascadeLiveActivityWidget.swift
│       │   ├── Assets.xcassets/
│       │   │   ├── AppIcon.appiconset
│       │   │   └── AccentColor.colorset
│       │   └── Resources/
│       │       ├── cascade.mp3
│       │       └── cascade.caf                    # optional gapless asset
│       ├── CascadeMac/                            # already designed in your macOS arch
│       └── CascadeCore.xcframework/               # built by build-rust.sh
└── build/
    └── build-rust-apple.sh                        # cargo build --target … + uniffi-bindgen swift + xcodebuild -create-xcframework
The CascadeShared SwiftPM package is the key structural decision. SwiftUI views, the AppViewModel, settings store, tick scheduler, and effect executor all live in CascadeShared. Only iOS-specific concerns live in CascadeiOS (background audio session, Live Activities, app delegate hooks). This matches the Clave doc's "large fraction of views shared between iOS and macOS via ClaveShared" guidance
.

3. The Rust → Swift Boundary
The same cascade.udl that generates Kotlin and C# bindings generates Swift. Build script:

bash
# build/build-rust-apple.sh
set -euo pipefail
cd crates/cascade-core

cargo build --release --target aarch64-apple-ios
cargo build --release --target aarch64-apple-ios-sim
cargo build --release --target x86_64-apple-ios

cd ../../bindings/cascade-uniffi
cargo run --bin uniffi-bindgen generate \
    --library ../../target/aarch64-apple-ios/release/libcascade_core.a \
    --language swift \
    --out-dir ../../apps/cascade-apple/CascadeShared/Sources/CascadeFFI

# Combine sim slices, then create the XCFramework
xcodebuild -create-xcframework \
    -library ../../target/aarch64-apple-ios/release/libcascade_core.a \
    -headers ../../apps/cascade-apple/CascadeShared/Sources/CascadeFFI/include \
    -library ../../target/ios-sim-universal/release/libcascade_core.a \
    -headers ../../apps/cascade-apple/CascadeShared/Sources/CascadeFFI/include \
    -output ../../apps/cascade-apple/CascadeCore.xcframework
UniFFI emits idiomatic Swift: enums become Swift enums (with associated values), records become structs, errors conform to Error. The generated dispatch(command: AppCommand) -> AppUpdate API is exactly the same shape Android and Windows consume — that's the whole point of the headless core.

The AppCommand / AppSnapshot / PlatformEffect contract is identical to the one the Windows architecture used. iOS just executes a different set of effects:

swift
// Effects that are uniquely "iOS-flavored"
case .startPlayback(let soundID, let loop, let vol, let fadeIn):
    try await audioEngine.startLoop(soundID: soundID,
                                    volume: vol,
                                    fadeIn: .milliseconds(fadeIn))
    nowPlaying.update(title: "Cascade", isPlaying: true)
    await app.dispatch(.platformPlaybackStarted)

case .acquireFocusPower(true):
    UIApplication.shared.isIdleTimerDisabled = true   // iOS-specific
    // (Android equivalent: KEEP_SCREEN_ON; macOS: IOPMAssertion; Windows: SetThreadExecutionState)

case .pausePlayback(let fadeOut):
    try await audioEngine.stop(fadeOut: .milliseconds(fadeOut))
    nowPlaying.update(isPlaying: false)
4. App Lifecycle & Audio Session
The single biggest iOS-specific wrinkle is AVAudioSession. The shell — never the core — owns this:

swift
@main
struct CascadeiOSApp: App {
    @State private var app = AppViewModel.shared

    init() {
        configureAudioSession()
    }

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environment(app)
        }
        .onChange(of: scenePhase) { _, phase in
            // Forward to the core; core decides whether to keep playing
            Task { await app.dispatch(.scenePhaseChanged(phase.toCore())) }
        }
    }

    private func configureAudioSession() {
        let session = AVAudioSession.sharedInstance()
        try? session.setCategory(.playback,
                                 mode: .default,
                                 options: [.allowAirPlay, .allowBluetoothA2DP])
        try? session.setActive(true)

        // Handle interruptions (phone calls, Siri, other media apps)
        NotificationCenter.default.addObserver(
            forName: AVAudioSession.interruptionNotification,
            object: session, queue: .main
        ) { note in
            // Translate to a core action; core decides what to do
            Task { await AppViewModel.shared.dispatch(.audioInterruption(note.toCore())) }
        }

        // Handle route changes (AirPods unplug = pause)
        NotificationCenter.default.addObserver(
            forName: AVAudioSession.routeChangeNotification,
            object: session, queue: .main
        ) { note in
            Task { await AppViewModel.shared.dispatch(.audioRouteChanged(note.toCore())) }
        }
    }
}
Crucially: the core is the policy-maker. The shell forwards interruption/route-change events as AppCommands, and the core decides whether to emit PausePlayback, StartPlayback, etc. That keeps the Apple-headphones-unplug = "pause" behavior consistent across all platforms.

Info.plist needs:

xml
<key>UIBackgroundModes</key>
<array><string>audio</string></array>
This unlocks lock-screen + backgrounded audio playback, which is non-negotiable for an 8-hour focus session app.

5. Audio Engine
Two viable choices:

v1 — Simple, robust: AVPlayer + AVPlayerLooper with an AVQueuePlayer. Three lines of code, handles MP3, integrates with Now Playing automatically. Use this to ship.

v2 — Sample-accurate gapless: AVAudioEngine + AVAudioPlayerNode scheduling the same AVAudioPCMBuffer in a loop. Required if you ever notice a seam at the loop boundary on a 13-minute MP3. Pair with a cascade.caf (PCM/uncompressed) asset for true gapless behavior.

swift
final class AudioEngine {
    private let engine = AVAudioEngine()
    private let player = AVAudioPlayerNode()
    private var buffer: AVAudioPCMBuffer?

    init() {
        engine.attach(player)
        engine.connect(player, to: engine.mainMixerNode, format: nil)
    }

    func startLoop(soundID: String, volume: UInt8, fadeIn: Duration) async throws {
        if buffer == nil {
            let url = Bundle.main.url(forResource: soundID, withExtension: "caf")!
            let file = try AVAudioFile(forReading: url)
            let buf  = AVAudioPCMBuffer(pcmFormat: file.processingFormat,
                                        frameCapacity: AVAudioFrameCount(file.length))!
            try file.read(into: buf)
            buffer = buf
        }
        try engine.start()
        player.scheduleBuffer(buffer!, at: nil, options: .loops)
        player.volume = Float(volume) / 100
        player.play()
    }

    func stop(fadeOut: Duration) async throws {
        // Ramp volume to 0 over fadeOut, then pause()
    }
}
6. Now Playing / Remote Commands
iOS audio apps must integrate with MPNowPlayingInfoCenter if they want to feel native. This is the iOS analogue to Windows SMTC and Android MediaSession:

swift
final class NowPlayingController {
    func register(dispatch: @escaping (AppCommand) async -> Void) {
        let cc = MPRemoteCommandCenter.shared()
        cc.playCommand.addTarget   { _ in Task { await dispatch(.play) };  return .success }
        cc.pauseCommand.addTarget  { _ in Task { await dispatch(.pause) }; return .success }
        cc.togglePlayPauseCommand.addTarget { _ in Task { await dispatch(.togglePlayback) }; return .success }
    }

    func update(title: String = "Cascade",
                subtitle: String = "Waterfall focus sound",
                isPlaying: Bool) {
        var info: [String: Any] = [
            MPMediaItemPropertyTitle: title,
            MPMediaItemPropertyArtist: subtitle,
        ]
        MPNowPlayingInfoCenter.default().nowPlayingInfo = info
        MPNowPlayingInfoCenter.default().playbackState = isPlaying ? .playing : .paused
    }
}
This gives you for free: lock-screen controls, Control Center media tile, AirPods squeeze, CarPlay support, and Siri ("hey Siri, pause Cascade").

7. Live Activities & Dynamic Island
For an 8-hour session preset, this is the iOS-specific superpower. While a session is running:

Lock screen: a Cascade Live Activity shows remaining time + a pause button.

Dynamic Island: a compact pill with the waterfall glyph and minutes remaining.

Use ActivityKit:

swift
struct CascadeActivityAttributes: ActivityAttributes {
    public struct ContentState: Codable, Hashable {
        var endsAt: Date
        var isPaused: Bool
    }
}
Drive it from a PlatformEffect::ShowSessionActivity { ends_at_ms } emitted by the core when a session starts. The shell starts/updates/ends the Activity; the core stays platform-agnostic.

8. Settings Persistence
Match the Rust-owned schema used everywhere else:

swift
final class SettingsStore {
    private var url: URL {
        let support = FileManager.default.urls(for: .applicationSupportDirectory,
                                               in: .userDomainMask).first!
        let dir = support.appendingPathComponent("Cascade", isDirectory: true)
        try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        return dir.appendingPathComponent("settings.json")
    }

    func load() throws -> String? {
        guard FileManager.default.fileExists(atPath: url.path) else { return nil }
        return try String(contentsOf: url, encoding: .utf8)
    }

    func save(json: String) throws {
        try json.write(to: url, atomically: true, encoding: .utf8)
    }
}
Do not use UserDefaults for the settings schema. Rust owns the schema and emits a PersistSettings { json } effect; the shell just writes bytes to disk.

9. UI Architecture (SwiftUI)
The root view binds to an @Observable AppViewModel that wraps the core handle and snapshot:

swift
@Observable
final class AppViewModel {
    static let shared = AppViewModel()

    private(set) var snapshot: AppSnapshot
    private let core: CoreHandle
    private let effects: EffectExecutor

    init() {
        let settingsJSON = try? SettingsStore().load()
        self.core     = try! CoreHandle.create(manifestJson: manifest,
                                               settingsJson: settingsJSON)
        self.snapshot = core.snapshot()
        self.effects  = EffectExecutor(...)
    }

    func dispatch(_ command: AppCommand) async {
        do {
            let update = try core.dispatch(command: command)
            self.snapshot = update.snapshot
            await effects.execute(update.effects, dispatch: { await self.dispatch($0) })
        } catch {
            // Surface to UI via snapshot.errorBanner if the core supports it
        }
    }
}
The SwiftUI view tree is essentially a function of snapshot:

swift
struct MainView: View {
    @Environment(AppViewModel.self) private var app

    var body: some View {
        VStack(spacing: 32) {
            Text("Cascade").font(.largeTitle.weight(.semibold))
            Text("Waterfall focus sound").foregroundStyle(.secondary)

            PlayPauseButton(isPlaying: app.snapshot.isPlaying) {
                Task { await app.dispatch(.togglePlayback) }
            }

            TimerPresetBar(snapshot: app.snapshot) { preset in
                Task { await app.dispatch(.startPomodoro(preset)) }
            }

            if case .running(let progress, let remaining) = app.snapshot.timer {
                ProgressView(value: progress)
                Text(remaining.formatted()).monospacedDigit()
            }

            VolumeSlider(value: app.snapshot.volumePercent) { v in
                Task { await app.dispatch(.setVolume(percent: v)) }
            }
        }
        .padding()
    }
}
This is exactly the pattern you'll reuse on macOS, just composed inside a WindowGroup with toolbar/menu commands instead of a phone-shaped layout.

10. Implementation Phases
Phase	Deliverable	Why
i0	Confirm cascade.udl and the existing AppCommand / AppSnapshot / PlatformEffect shapes from JacobStephens2/cascade	Don't invent a parallel iOS-only contract
i1	XCFramework spike: blank SwiftUI app calls dispatch(.play) and prints the snapshot	De-risks the entire FFI path; runs on a personal device via free Apple Developer signing or TestFlight
i2	AVPlayerLooper-based AudioEngine + background audio mode + AVAudioSession	App can replace any focus-sound app you currently use
i3	Timer presets (30/60/8h/custom), Tick dispatch, settings.json persistence	Functional parity with web/Android
i4	Now Playing / lock-screen controls / interruption handling	iOS-native polish — feels like a real audio app
i5	Live Activity + Dynamic Island for active sessions	iOS-specific superpower
i6	Shared CascadeShared views with macOS, settle final visual design with SF Symbols + system materials	Sets up macOS for cheap parity
11. Non-Goals (Important)
No Capacitor/Cordova/React Native wrapper around cascade.stephens.page

No WKWebView shell — the deployed PWA already covers iOS Safari users

No Swift implementation of timer/session/settings logic — that all lives in Rust

No UserDefaults for the cross-platform settings schema

No background audio without UIBackgroundModes: audio and .playback category

No ad-hoc audio session activation scattered across views — one owner: AudioEngine

No Apple-only command shapes — every AppCommand must be expressible across all four platforms

Final Recommendation
Build apps/cascade-apple/CascadeiOS as a SwiftUI + Swift 6 + iOS 17 app that:

Consumes cascade-core via a UniFFI-generated XCFramework

Uses AVPlayerLooper (v1) → AVAudioEngine (v2) for gapless looping waterfall audio with full AVAudioSession lifecycle handling

Integrates with MPNowPlayingInfoCenter + MPRemoteCommandCenter for lock-screen and AirPods control

Persists Rust-owned settings.json to Application Support

Uses an @Observable AppViewModel that exposes a snapshot and a single dispatch(AppCommand) entry point — identical conceptual shape to the Windows architecture

Adds ActivityKit Live Activities for in-progress sessions in Phase 2

Shares the SwiftUI view tree with macOS via a CascadeShared SwiftPM package

The core architectural rule, identical to Windows: the Rust core owns Cascade's truth; iOS owns Cascade's iOS-ness. Everything that's "what Cascade is" (sessions, timers, settings, playback policy) belongs in cascade-core. Everything that's "how iOS does things" (AVAudioSession, Now Playing, Live Activities, isIdleTimerDisabled) belongs in the SwiftUI shell.

If you can paste the current AppCommand / AppSnapshot / PlatformEffect definitions from your cascade-core crate, I can tailor the Swift bindings consumer code and exact EffectExecutor switch arms to your real types rather than the inferred ones above.

# Gemini 3.1 Pro Thinking

Following the architectural philosophy from the clave-tech-stack.md and our previous Windows design, the iOS version of Cascade must be a genuinely native SwiftUI app. It will use the exact same headless Rust core (cascade-core), compiled to an XCFramework via UniFFI, and rely on iOS-native frameworks (AVFoundation, MediaPlayer) for audio and system integration.

Here is the iOS architecture for Cascade.

1. iOS Architectural Stack
Concern	Choice	Rationale
UI Framework	SwiftUI	The definitive Apple UI framework. Matches the Clave mandate.
Language	Swift 5.10+	Use modern Swift concurrency (async/await) and @Observable.
Core Access	UniFFI (Swift)	uniffi-bindgen generates idiomatic Swift wrappers. Packaged as a binary .xcframework.
Audio Engine	AVQueuePlayer + AVPlayerLooper	AVFoundation is Apple's lowest-level audio API. The queue/looper pattern provides perfect, gapless playback.
Background Playback	AVAudioSession (.playback) + Background Modes	Required for audio to continue playing when the iPhone screen is locked.
Lock Screen Controls	MPRemoteCommandCenter	Integrates play/pause into the iOS Lock Screen and Control Center.
Notifications	UNUserNotificationCenter	Triggers local "session complete" push notifications.
State Management	@Observable ViewModel	Holds the AppSnapshot. The SwiftUI views bind directly to this snapshot.
2. The Rust → Swift Boundary (XCFramework)
Unlike Windows where a simple .dll suffices, iOS requires building for devices (aarch64-apple-ios) and simulators (aarch64-apple-ios-sim), then combining them.

Build Pipeline:

Compile the Rust core for iOS targets using cargo build --target ...

Run uniffi-bindgen generate src/cascade.udl --language swift

Use xcodebuild -create-xcframework to wrap the static libraries (libcascade_core.a), generated C headers, and module.modulemap into CascadeCore.xcframework.

You integrate this into Xcode by dragging the .xcframework into your project and adding the generated CascadeCore.swift file to your source tree.

3. iOS State Flow & ViewModel
SwiftUI reacts perfectly to UniFFI's generated models if you wrap them in an @Observable class. The state flow matches the Windows/Web architecture exactly: UI action → dispatch → Rust reduces state → Snapshot & Effects returned → UI renders & executes effects.

swift
import SwiftUI
import CascadeCore // The generated UniFFI module

@Observable
@MainActor
final class AppViewModel {
    private let core: CoreHandle
    private let effectExecutor: EffectExecutor
    
    // The single source of truth for the UI
    var snapshot: AppSnapshot
    
    init(core: CoreHandle, effectExecutor: EffectExecutor) {
        self.core = core
        self.effectExecutor = effectExecutor
        self.snapshot = core.snapshot()
    }
    
    func dispatch(_ action: AppCommand) {
        do {
            let update = try core.dispatch(command: action)
            self.snapshot = update.snapshot
            
            // Fire-and-forget effect execution
            Task {
                await effectExecutor.execute(effects: update.effects, dispatch: self.dispatch)
            }
        } catch {
            print("Core dispatch failed: \(error)")
        }
    }
}
4. Background Audio & Lifecycle (Crucial for iOS)
iOS aggressively suspends apps when they go to the background. However, if you configure an AVAudioSession properly, your app's thread remains active as long as audio is playing. This means your Rust Tick(now_ms) will safely continue running in the background.

Step 1: Info.plist capability
Add the UIBackgroundModes array and include audio.

Step 2: Configure AVAudioSession
On app launch, you must tell the iOS system that this app is a media player.

swift
import AVFoundation

func configureAudioSession() {
    do {
        let session = AVAudioSession.sharedInstance()
        // .playback prevents audio from stopping when the silent switch is on or the screen locks
        try session.setCategory(.playback, mode: .default, options: [])
        try session.setActive(true)
    } catch {
        print("Failed to set audio session category: \(error)")
    }
}
Step 3: Gapless Playback Service
To ensure the waterfalls.mp3 or .wav loops flawlessly, avoid the older AVAudioPlayer. Use AVPlayerLooper:

swift
final class iOSPlaybackService {
    private var queuePlayer: AVQueuePlayer?
    private var playerLooper: AVPlayerLooper?
    
    func startLoop(soundId: String, volumePercent: UInt8) {
        guard let url = Bundle.main.url(forResource: soundId, withExtension: "mp3") else { return }
        
        let playerItem = AVPlayerItem(url: url)
        let player = AVQueuePlayer(playerItem: playerItem)
        player.volume = Float(volumePercent) / 100.0
        
        // This guarantees a seamless, gapless loop at the OS level
        self.playerLooper = AVPlayerLooper(player: player, templateItem: playerItem)
        self.queuePlayer = player
        
        player.play()
    }
    
    func pause() {
        queuePlayer?.pause()
    }
    
    func setVolume(percent: UInt8) {
        queuePlayer?.volume = Float(percent) / 100.0
    }
}
5. Lock Screen Integration (Now Playing)
On iOS, users expect background audio apps to be controllable from the Lock Screen, Apple Watch, and Control Center. This maps perfectly to your Rust core actions.

swift
import MediaPlayer

final class MediaTransportService {
    func setupControls(dispatch: @escaping (AppCommand) -> Void) {
        let center = MPRemoteCommandCenter.shared()
        
        center.playCommand.addTarget { _ in
            dispatch(.Play)
            return .success
        }
        
        center.pauseCommand.addTarget { _ in
            dispatch(.Pause)
            return .success
        }
    }
    
    func updateNowPlaying(title: String, subtitle: String, isPlaying: Bool) {
        var info = [String: Any]()
        info[MPMediaItemPropertyTitle] = title
        info[MPMediaItemPropertyArtist] = subtitle
        // Tells the OS to show the Pause button vs Play button
        info[MPNowPlayingInfoPropertyPlaybackRate] = isPlaying ? 1.0 : 0.0 
        
        MPNowPlayingInfoCenter.default().nowPlayingInfo = info
    }
}
When Rust returns the PlatformEffect.UpdateSystemMediaControls effect, you pass the payload to updateNowPlaying().

6. Timers and Backgrounding Safety
Because AVAudioSession keeps your app alive, a simple Task sleeping for 1 second and dispatching Tick(now_ms) will work flawlessly while the waterfall plays.

However, if the user starts a timer and pauses the audio, iOS will suspend the app within ~30 seconds. To fix this:

When Rust executes a timer start, map it to a PlatformEffect.ScheduleLocalNotification(trigger_at_ms).

The iOS UI schedules a UNNotificationRequest for that exact timestamp.

If the app is alive, Rust handles the expiration and plays/stops media. If the app was suspended, iOS delivers the push notification ("Your 60-minute session is complete") to wake the user.

7. App Structure
text
apps/cascade-ios/
├── Cascade/
│   ├── CascadeApp.swift           # App entry point, configures AVAudioSession
│   ├── Info.plist                 # UIBackgroundModes: audio
│   ├── ViewModels/
│   │   └── AppViewModel.swift     # @Observable source of truth
│   ├── Views/
│   │   ├── MainView.swift         # SwiftUI window (Volume slider, Session presets)
│   │   ├── SettingsView.swift     # SwiftUI view for preferences
│   │   └── Components/
│   ├── Services/
│   │   ├── EffectExecutor.swift   # Switch statement iterating over PlatformEffects
│   │   ├── AudioService.swift     # AVQueuePlayer + AVPlayerLooper
│   │   ├── MediaControls.swift    # MPRemoteCommandCenter
│   │   ├── TickService.swift      # Async Task emitting 1s Ticks
│   │   └── SettingsStore.swift    # UserDefaults / JSON file bridge
│   ├── CoreBridge/
│   │   └── CascadeCore.swift      # Generated by uniffi-bindgen
│   └── Resources/
│       ├── waterfalls.mp3
│       └── Assets.xcassets
└── CascadeCore.xcframework        # Compiled Rust binary
8. SwiftUI Implementation
Because the Rust snapshot dictates everything, the UI is purely declarative and requires almost no logic.

swift
struct MainView: View {
    @Environment(AppViewModel.self) private var viewModel
    
    var body: some View {
        let snapshot = viewModel.snapshot
        
        VStack(spacing: 40) {
            // Waterfall Title
            Text("Cascade")
                .font(.largeTitle.weight(.semibold))
            
            // Play/Pause Button
            Button(action: { viewModel.dispatch(.TogglePlayback) }) {
                Image(systemName: snapshot.isPlaying ? "pause.circle.fill" : "play.circle.fill")
                    .resizable()
                    .frame(width: 80, height: 80)
            }
            
            // Session Presets
            HStack {
                PresetButton(title: "∞", action: { viewModel.dispatch(.StartSession(duration: .Infinite)) })
                PresetButton(title: "30", action: { viewModel.dispatch(.StartSession(duration: .Minutes30)) })
                PresetButton(title: "60", action: { viewModel.dispatch(.StartSession(duration: .Minutes60)) })
            }
            
            // Timer Progress
            if let remaining = snapshot.remainingTimeStr {
                Text(remaining)
                    .font(.system(.title, design: .monospaced))
                
                ProgressView(value: snapshot.progressPercent)
                    .tint(.blue)
            }
            
            // Volume
            Slider(
                value: Binding(
                    get: { Double(snapshot.volumePercent) },
                    set: { viewModel.dispatch(.SetVolume(percent: UInt8($0))) }
                ),
                in: 0...100
            )
        }
        .padding()
    }
}
Summary of the iOS approach
By keeping the Rust core headless, your iOS client acts purely as a SwiftUI rendering layer and an AVFoundation hardware interface. It adheres to the clave-tech-stack.md principles exactly: Rust owns the timer mechanics, the rules, and the settings schema; iOS merely paints the pixels, schedules the local notifications, and operates the speaker hardware.