# Perplexity Model Council Synthesis

### 1. Where Models Agree

| Finding | GPT-5.5 Thinking | Claude Opus 4.7 Thinking | Gemini 3.1 Pro Thinking | Evidence |
|---------|-----------|-----------|-----------|----------|
| macOS should be a **native SwiftUI** client on top of the same **headless Rust core** via **UniFFI Swift** | ✓ | ✓ | ✓ | Claude proposes SwiftUI + UniFFI + XCFramework path. [ppl-ai-file-upload.s3.amazonaws](https://ppl-ai-file-upload.s3.amazonaws.com/web/direct-files/attachments/130224663/e826d6d9-002d-4f5c-8a8c-2b9f967bd923/clave-tech-stack.md) |
| Keep the Clave boundary: Rust owns **state/timer/settings**, macOS owns **audio/UI/OS integration** | ✓ | ✓ | ✓ | Clave doc: audio and UI are platform concerns; core stays headless. |
| Implement the macOS app as a **menu bar utility** (MenuBarExtra), optionally with a main window + Settings scene | ✓ | ✓ | ✓ | MenuBarExtra intended for always-available utility access. |
| Persist settings to a **single JSON artifact** (App Support file or equivalent), with Rust as schema owner | ✓ | ✓ | ✓ | Clave doc: core owns serialization schemas (JSON via serde) and coarse-grained boundaries. |
| Timer should be driven by explicit **Tick(now)** events from the macOS wall clock | ✓ | ✓ | ✓ | Clave doc: no system time inside core; UI supplies time inputs. |

***

### 2. Where Models Disagree

| Topic | GPT-5.5 Thinking | Claude Opus 4.7 Thinking | Gemini 3.1 Pro Thinking | Why They Differ |
|-------|-----------|-----------|-----------|-----------------|
| Audio implementation | `AVAudioPlayer` (simple) for looping + volume. | `AVAudioEngine` + `AVAudioPlayerNode` for sample-accurate looping and fades | `AVAudioPlayer` but strongly urges converting audio to WAV/CAF for gapless looping | Different weighting of “simplicity” vs “gapless perfection.” Claude assumes you’ll care about audible seams; GPT optimizes for minimal moving parts; Gemini assumes MP3 loop padding will be noticeable. |
| App shape: menu-bar-only vs Dock app + menu bar | Recommends Dock app **plus** MenuBarExtra first | Menu bar + a normal window; distribution via DMG; MAS later | Prefers menu-bar-only (LSUIElement) from the start | Different focus on daily ergonomics vs development/debuggability. |
| Settings storage mechanism | JSON file in Application Support | Same (explicit Application Support path) | Suggests putting JSON in UserDefaults (simpler) | Gemini optimizes for speed; GPT/Claude optimize for a durable “settings.json is a first-class artifact” contract you can reuse cross-platform. |
| “Now Playing” / media keys | Optional Phase 2, best-effort | More strongly recommends it as native polish | Mentions it, but focuses more on App Nap and loop format | Different views of what makes it “daily-driver” on macOS and what’s reliable on macOS. |

***

### 3. Unique Discoveries

| Model | Unique Finding | Why It Matters |
|-------|----------------|----------------|
| Claude Opus 4.7 Thinking | Use `IOPMAssertion` to prevent sleep during long sessions | Matches your 8-hour preset and “prayer/work sessions” requirement without keeping the display awake. |
| Gemini 3.1 Pro Thinking | Watch out for App Nap / timer throttling; consider `ProcessInfo.beginActivity` | Ensures your Tick cadence stays responsive even when the popover is closed and the app is backgrounded. |

***

### 4. Comprehensive Analysis

**High-Confidence Findings.** The macOS version of **Cascade** should be a genuinely native **SwiftUI** app, consuming your shared headless **Rust core** via **UniFFI-generated Swift bindings** (ideally packaged as an XCFramework so you can later reuse the same artifact for iOS). Claude Opus 4.7’s layout—`cascade-core` + `bindings/uniffi` + `apps/cascade-macos`—is exactly the Clave pattern, and it’s the right way to “test the architecture” without accidentally turning this into a cross-platform UI or audio-engine project.

All models also align with your key boundary rule: the Rust core owns the state machine, pomodoro/sleep timer logic, and settings schema; the macOS shell owns audio playback, UI, filesystem paths, menu bar, and OS integrations. Concretely: SwiftUI sends **AppCommand**s into the core; the core returns an **AppSnapshot** (for rendering) plus **PlatformEffect**s (for side effects like “start playback,” “set volume,” “persist settings”). This is the exact discipline your Clave stack is trying to enforce (coarse-grained calls; snapshots rather than getters; determinism by passing time in).

Finally, for daily usability on macOS, all three converge on a **MenuBarExtra** control surface: it’s the native “always-available utility” paradigm, and Apple explicitly positions MenuBarExtra for functionality you want accessible even when the app isn’t active. For you, that means play/pause, volume, timer presets, and “remaining time” should all be reachable from the menu bar.

**Areas of Divergence.** The largest design fork is **audio playback strategy**. GPT-5.5 prefers `AVAudioPlayer` for minimal complexity (it directly supports volume and looping), while Claude pushes `AVAudioEngine` + `AVAudioPlayerNode` to get sample-accurate looping and nicer fades. Apple documents `AVAudioPlayer` as suitable for duration playback with volume and looping controls, so GPT’s approach is totally reasonable for v1. But given you’ve listened to this file for thousands of hours, Claude and Gemini’s concern about audible loop seams is plausible: MP3/AAC padding and decoder behavior can produce tiny gaps; the safest “never hear the seam” approach is either (a) decode once into PCM and loop the buffer (Claude), or (b) convert the asset to WAV/CAF (Gemini). If “the seam every 13 minutes” would drive you crazy, choose the `AVAudioEngine` buffer loop or convert to a gapless-friendly format.

A second fork is **menu-bar-only vs normal app + menu bar**. Gemini recommends setting LSUIElement immediately (no Dock icon). GPT recommends starting as a normal Dock app plus MenuBarExtra (easier debugging, easier discoverability) and optionally adding “hide Dock icon” later. For a personal daily-driver, I’d follow GPT’s sequencing: ship a normal app first, then add a “Menu-bar only” toggle once everything is stable.

Settings persistence has a similar tradeoff: Gemini’s “UserDefaults string” is quick, but you explicitly decided on “persisted settings JSON,” and GPT/Claude’s “`~/Library/Application Support/Cascade/settings.json`” makes that JSON a real artifact you can inspect, migrate, and potentially sync later. That’s also closer to the Clave idea that the core owns schemas and data formats.

**Unique Insights Worth Noting.** Claude’s `IOPMAssertion` and Gemini’s `ProcessInfo.beginActivity` are both pragmatic macOS-only concerns that fit perfectly into the “effects” architecture: the core can emit an effect like `AcquireFocusSessionPowerPolicy(enabled: true)` when a session starts, and the macOS shell can implement it with either mechanism. This is a good rehearsal for Clave: “core declares intent; platform does platform things.”

**Recommendations.** Implement macOS as:

- **SwiftUI + MenuBarExtra + Settings scene** (main window optional but recommended initially).
- **UniFFI Swift** bindings to `cascade-core`, with a Swift `CascadeStore` (Observable / @Observable) that holds `AppSnapshot` and executes `PlatformEffect`s.
- **Audio**: start with `AVAudioPlayer` for v1 (fastest), but plan an upgrade path to either convert the asset to WAV/CAF or switch to `AVAudioEngine` buffer-looping if you hear any seam.
- **Persistence**: `Application Support/Cascade/settings.json`, with Rust producing/consuming the JSON string so schema stays unified across web/android/macos.
- **Timer**: Swift emits `Tick(now_ms)` every second; Rust decides when to stop playback and emits the pause effect.

If you want, I can turn this into a concrete macOS “TDD” style doc for your repo (`apps/cascade-macos/ARCHITECTURE.md`) plus a minimal file tree and the exact Swift/Rust type shapes that should cross UniFFI.

# GPT 5.5 Thinking

Recommendation
Architect the macOS version of Cascade as a native SwiftUI app with a menu bar control surface, backed by the same headless Rust core and the same dispatch(action) -> { snapshot, effects } pattern as web and Android.

The macOS shell should own all Apple-specific behavior:

SwiftUI UI

menu bar item

bundled audio playback

keyboard shortcuts

Settings window

launch-at-login

optional Now Playing/media key integration

The Rust core should stay exactly as decided:

state machine

timer logic via explicit Tick events

settings schema

action reducer

snapshots

platform effects

no audio APIs

For daily use, I would make macOS the nicest version of Cascade: quick menu bar access, polished keyboard controls, and a small calm main window.

1. Platform stack
Use this stack:

Layer	Technology
UI	SwiftUI
App shape	Main window + MenuBarExtra
Rust binding	UniFFI Swift
Audio	AVAudioPlayer inside a dedicated MacPlaybackManager
Settings persistence	JSON file in Application Support
Lightweight preferences	SwiftUI Settings scene
Optional startup	SMAppService.mainApp launch-at-login
Optional media controls	MPNowPlayingInfoCenter + MPRemoteCommandCenter, best effort
AVAudioPlayer is appropriate because it is designed to play audio from a file or buffer and includes controls for volume and looping behavior.
 MenuBarExtra is a strong fit because Apple positions it for commonly used functionality that should remain accessible even when the app is not active.
 SwiftUI’s Settings scene is the native macOS way to expose a preferences/settings window.

2. Repository layout
Because the app name is now Cascade, I would rename the example waterfall-* package names before implementation. Renames are cheap now and annoying after UniFFI namespaces, bundles, and package IDs exist.

text
cascade/
├── crates/
│   └── cascade-core/
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── action.rs
│           ├── state.rs
│           ├── snapshot.rs
│           ├── effect.rs
│           ├── settings.rs
│           ├── timer.rs
│           └── reducer.rs
│
├── bindings/
│   ├── cascade-uniffi/
│   │   ├── Cargo.toml
│   │   ├── src/lib.rs
│   │   └── cascade.udl
│   │
│   └── cascade-wasm/
│       └── ...
│
├── apps/
│   ├── cascade-web/
│   ├── cascade-android/
│   └── cascade-macos/
│       ├── Cascade.xcodeproj
│       └── Cascade/
│           ├── CascadeApp.swift
│           ├── App/
│           │   ├── CascadeStore.swift
│           │   ├── CascadeCommands.swift
│           │   └── AppLifecycleObserver.swift
│           ├── Core/
│           │   ├── CascadeCoreBridge.swift
│           │   └── Generated/              # UniFFI Swift output
│           ├── Playback/
│           │   ├── MacPlaybackManager.swift
│           │   ├── AudioAssetResolver.swift
│           │   └── NowPlayingController.swift
│           ├── Persistence/
│           │   ├── SettingsStore.swift
│           │   └── ApplicationSupportPaths.swift
│           ├── UI/
│           │   ├── MainWindowView.swift
│           │   ├── MenuBarView.swift
│           │   ├── TimerPresetPicker.swift
│           │   ├── VolumeControl.swift
│           │   └── SettingsView.swift
│           └── Resources/
│               └── waterfalls.mp3
│
└── assets/
    └── cascade/
        └── waterfalls.mp3
I would use cascade-core, cascade-uniffi, cascade-web, cascade-android, and cascade-macos consistently.

3. macOS app shape
Main window
The main window is the full, calm daily-use interface:

text
┌─────────────────────────────────────┐
│ Cascade                             │
│ Waterfall focus sound               │
│                                     │
│          [  Play / Pause  ]          │
│                                     │
│ Session                             │
│ [∞] [30 min] [60 min] [8 hr] [Custom]│
│                                     │
│ Remaining: 42:17                    │
│ ███████████░░░░░░░░░                │
│                                     │
│ Volume                              │
│ ─────────●────────                  │
└─────────────────────────────────────┘
The main view should render only from AppSnapshot, not from raw platform state.

Menu bar extra
The menu bar extra is the daily-use fast path:

text
Cascade
Playing · 42:17 remaining

[Pause]
Session
  ○ Infinite
  ● 30 minutes
  ○ 60 minutes
  ○ 8 hours
  ○ Custom...

Volume  [────●────]

Open Cascade
Settings...
Quit
This makes Cascade useful without hunting for a Dock icon or window. MenuBarExtra is specifically intended for access to commonly used functionality even when your app is inactive, so it matches the “daily background utility” use case well.

Settings window
Use SwiftUI’s Settings scene for preferences because macOS wires it naturally into the app’s Settings/Preferences menu and ⌘, convention.

Settings should include:

Default session length: Infinite / 30 min / 60 min / 8 hr / Custom

Default volume

Launch at login

Show menu bar item

Show Dock icon — optional later; see note below

Reset settings JSON

Reveal settings file in Finder — useful for debugging

For launch at login, use SMAppService; modern macOS supports registering and unregistering the main app as a login item through this API.

4. Dock icon vs menu-bar-only
For v1, I recommend normal Dock app + menu bar extra.

Do not start with a menu-bar-only LSUIElement app. A normal app is easier to debug, easier to find, easier to quit, easier to manage during development, and still gives you menu bar convenience.

Later, add a setting:

text
Appearance:
[x] Show Dock icon
[x] Show menu bar item
Menu-bar-only macOS apps commonly remove the main window and set LSUIElement / “Application is agent” to hide the Dock icon, but that is a product-polish decision, not an architecture requirement.

5. Core boundary
The macOS app should consume the same conceptual core as web and Android:

rust
pub fn dispatch(action: AppAction) -> AppUpdate;
Where:

rust
pub struct AppUpdate {
    pub snapshot: AppSnapshot,
    pub effects: Vec<PlatformEffect>,
}
The macOS shell should never ask the Rust core tiny questions like:

rust
get_volume()
is_playing()
remaining_seconds()
Instead, the core returns an AppSnapshot, and SwiftUI renders from that.

6. Core model additions for macOS
Your core should be platform-agnostic, but the macOS app benefits from a few carefully chosen fields.

Actions
rust
pub enum AppAction {
    Play,
    Pause,
    TogglePlayback,

    SetVolume { percent: u8 },

    StartSession { duration: SessionDuration },
    CancelSession,

    Tick { now_ms: u64 },

    PlatformPlaybackStarted,
    PlatformPlaybackPaused,
    PlatformPlaybackFailed { message: String },

    SettingsLoaded { json: String },
    ResetSettings,
}
Session duration
rust
pub enum SessionDuration {
    Infinite,
    Minutes30,
    Minutes60,
    Hours8,
    Custom { duration_ms: u64 },
}
I would call this a session timer in the code rather than a strict “Pomodoro” timer, because your actual desired presets are 30 minutes, 60 minutes, 8 hours, and custom prayer/work sessions.

Snapshot
rust
pub struct AppSnapshot {
    pub app_title: String,
    pub sound_title: String,

    pub desired_playback: DesiredPlayback,
    pub platform_playback: PlatformPlaybackState,

    pub volume_percent: u8,

    pub selected_session: SessionDurationView,
    pub session_active: bool,
    pub session_started_at_ms: Option<u64>,
    pub session_ends_at_ms: Option<u64>,
    pub remaining_ms: Option<u64>,
    pub elapsed_ms: u64,
    pub progress_percent: Option<u8>,

    pub primary_button_label: String,
    pub status_line: String,
    pub error_message: Option<String>,
}
Effects
rust
pub enum PlatformEffect {
    StartPlayback {
        sound_id: String,
        loop_forever: bool,
        volume_percent: u8,
    },

    PausePlayback,

    SetVolume {
        volume_percent: u8,
    },

    PersistSettings {
        json: String,
    },

    ShowNotification {
        title: String,
        body: String,
    },
}
The key rule: Rust declares intent; macOS executes it.

7. macOS state flow
Use a single Swift object as the app coordinator.

swift
@MainActor
final class CascadeStore: ObservableObject {
    @Published private(set) var snapshot: AppSnapshot

    private let core: CascadeCoreBridge
    private let playback: MacPlaybackManager
    private let settings: SettingsStore
    private var tickTask: Task<Void, Never>?

    func dispatch(_ action: AppAction) {
        let update = core.dispatch(action)
        snapshot = update.snapshot
        execute(update.effects)
    }

    private func execute(_ effects: [PlatformEffect]) {
        for effect in effects {
            switch effect {
            case let .startPlayback(soundId, loopForever, volumePercent):
                playback.startLoop(soundId: soundId, volumePercent: volumePercent)
                dispatch(.platformPlaybackStarted)

            case .pausePlayback:
                playback.pause()
                dispatch(.platformPlaybackPaused)

            case let .setVolume(volumePercent):
                playback.setVolume(volumePercent)

            case let .persistSettings(json):
                settings.save(json)

            case let .showNotification(title, body):
                // Optional later
                break
            }
        }
    }
}
SwiftUI views depend on CascadeStore.snapshot, and user actions call store.dispatch(...).

That gives you the exact same mental model across platforms:

text
SwiftUI/MenuBar
  -> CascadeStore.dispatch(action)
  -> Rust core reducer
  -> AppSnapshot + PlatformEffect[]
  -> Swift executes effects
  -> SwiftUI re-renders snapshot
8. Timer architecture
The timer must be driven by explicit Tick events.

On macOS, the Swift shell owns the wall clock. The Rust core receives timestamps as data.

swift
private func startTickingIfNeeded() {
    tickTask?.cancel()

    tickTask = Task { [weak self] in
        while !Task.isCancelled {
            try? await Task.sleep(for: .seconds(1))
            await MainActor.run {
                self?.dispatch(.tick(nowMs: currentTimeMillis()))
            }
        }
    }
}
The Rust core decides:

how much time remains

whether the session expired

whether to emit PausePlayback

whether settings need persistence

This preserves your Clave-style determinism rule: the core is testable because time is an input, not an ambient dependency.

Timer behavior:

User chooses	Core behavior
Infinite	Play until manually paused
30 min	Set session_ends_at_ms = now + 30m
60 min	Set session_ends_at_ms = now + 60m
8 hr	Set session_ends_at_ms = now + 8h
Custom	Validate duration, set end time
Tick after end	Emit PausePlayback, mark session complete
For prayer/work sessions, I’d make timer expiration calm: stop the falls, maybe show a subtle notification, but do not play an alarm sound.

9. Audio playback manager
All audio APIs should live behind one protocol:

swift
protocol PlaybackManaging {
    func startLoop(soundId: String, volumePercent: UInt8) throws
    func pause()
    func setVolume(_ percent: UInt8)
    var isPlaying: Bool { get }
}
Implementation:

swift
final class MacPlaybackManager: NSObject, PlaybackManaging {
    private var player: AVAudioPlayer?
    private let resolver: AudioAssetResolver

    func startLoop(soundId: String, volumePercent: UInt8) throws {
        let url = try resolver.url(for: soundId)

        if player == nil {
            let p = try AVAudioPlayer(contentsOf: url)
            p.numberOfLoops = -1
            p.prepareToPlay()
            player = p
        }

        player?.volume = Float(volumePercent) / 100.0
        player?.play()
    }

    func pause() {
        player?.pause()
    }

    func setVolume(_ percent: UInt8) {
        player?.volume = Float(percent) / 100.0
    }

    var isPlaying: Bool {
        player?.isPlaying ?? false
    }
}
AVAudioPlayer is a good fit for this local-file use case because it handles file playback and exposes playback controls like volume and looping.
 The numberOfLoops property is the API surface used to control repeat behavior.

Do not add AVAudioSession code copied from iOS examples; AVAudioSession is not the right macOS abstraction and is unavailable in modern macOS versions such as macOS 14.

10. Audio asset strategy
For v1:

text
apps/cascade-macos/Cascade/Resources/waterfalls.mp3
Then:

swift
struct AudioAssetResolver {
    func url(for soundId: String) throws -> URL {
        switch soundId {
        case "waterfalls":
            guard let url = Bundle.main.url(
                forResource: "waterfalls",
                withExtension: "mp3"
            ) else {
                throw AudioAssetError.missingBundledAsset
            }
            return url

        default:
            throw AudioAssetError.unknownSoundId(soundId)
        }
    }
}
Bundling the file avoids sandbox complexity. If you later support user-selected audio files, then the sandboxed Mac app will need security-scoped bookmarks because sandboxed apps need user-granted access to external files and folders.

11. Settings persistence
You specifically decided on persisted settings JSON, so keep that as a first-class artifact rather than scattering settings across UserDefaults.

Use:

text
~/Library/Application Support/Cascade/settings.json
Schema owned by Rust:

rust
pub struct PersistedSettingsV1 {
    pub version: u32,
    pub default_volume_percent: u8,
    pub default_session: SessionDuration,
    pub last_session: Option<SessionDuration>,
    pub launch_at_login: bool,
}
Flow:

text
App launch
  -> macOS SettingsStore reads settings.json
  -> dispatch(SettingsLoaded { json })
  -> Rust validates/migrates/defaults
  -> returns snapshot + maybe PersistSettings(normalized_json)

User changes volume/session/defaults
  -> Rust updates state
  -> Rust emits PersistSettings { json }
  -> macOS writes settings.json
For purely macOS-only flags that are not part of the cross-platform core, such as “show Dock icon,” it is acceptable to use UserDefaults. But the shared app behavior settings should stay in the Rust-owned JSON.

12. Menu commands and keyboard shortcuts
Add native menu commands:

swift
.commands {
    CommandMenu("Cascade") {
        Button("Play/Pause") {
            store.dispatch(.togglePlayback)
        }
        .keyboardShortcut(.space, modifiers: [])

        Divider()

        Button("30 Minute Session") {
            store.dispatch(.startSession(.minutes30))
        }
        .keyboardShortcut("3", modifiers: [.command])

        Button("60 Minute Session") {
            store.dispatch(.startSession(.minutes60))
        }
        .keyboardShortcut("6", modifiers: [.command])

        Button("8 Hour Session") {
            store.dispatch(.startSession(.hours8))
        }
        .keyboardShortcut("8", modifiers: [.command])
    }
}
SwiftUI supports keyboard shortcuts on controls through the keyboardShortcut modifier.

Suggested shortcuts:

Shortcut	Action
Space	Play/Pause, when main window focused
⌘3	Start 30-minute session
⌘6	Start 60-minute session
⌘8	Start 8-hour session
⌘,	Settings
⌘Q	Quit
Avoid global hotkeys in v1. If you want user-customizable global shortcuts later, use a focused macOS package or a small native implementation; this should not go in Rust.

13. Now Playing and media keys
Treat Now Playing / media key support as Phase 2, not a v1 blocker.

If you add it, create:

swift
final class NowPlayingController {
    func update(snapshot: AppSnapshot) { ... }
    func registerRemoteCommands(store: CascadeStore) { ... }
}
Use:

MPNowPlayingInfoCenter to publish current title, playback state, duration, elapsed time

MPRemoteCommandCenter to receive play/pause commands

Apple’s media-control model requires syncing in both directions: the app updates Now Playing state, and remote controls send commands back to the app through MPRemoteCommandCenter.

However, macOS media-key behavior can be finicky and less deterministic than iOS; there are reports that background remote-command handling on macOS does not reliably put an app at the top of the system’s internal subscriber queue.
 So implement this as best-effort polish after the core app is already useful.

14. Launch at login
Add this in Settings:

swift
import ServiceManagement

func setLaunchAtLogin(_ enabled: Bool) throws {
    if enabled {
        try SMAppService.mainApp.register()
    } else {
        try SMAppService.mainApp.unregister()
    }
}
Modern macOS apps should use SMAppService to register or unregister login items.

This should be a platform-only setting effect:

text
User toggles launch at login
  -> Swift calls SMAppService
  -> Swift dispatches action to core if you want snapshot/settings to reflect the result
  -> Rust emits PersistSettings
Do not make Rust know what SMAppService is.

15. SwiftUI scene structure
swift
@main
struct CascadeApp: App {
    @StateObject private var store = CascadeStore.bootstrap()

    var body: some Scene {
        WindowGroup {
            MainWindowView()
                .environmentObject(store)
        }

        MenuBarExtra {
            MenuBarView()
                .environmentObject(store)
        } label: {
            Image(systemName: store.snapshot.desiredPlayback.isPlaying
                ? "water.waves"
                : "drop")
        }

        Settings {
            SettingsView()
                .environmentObject(store)
        }
    }
}
WindowGroup, Settings, and MenuBarExtra are all standard SwiftUI scene types for macOS apps.
 MenuBarExtra specifically gives you a status-item style interaction point in the macOS menu bar.

16. Suggested implementation phases
Phase 1 — Core compatibility
Rename crates to cascade-*

Ensure cascade-core has actions, snapshots, effects, timer presets, settings JSON

Add tests for:

play emits StartPlayback

pause emits PausePlayback

30/60/8h/custom sessions expire on Tick

volume clamps to 0..=100

settings JSON round-trips

Phase 2 — macOS binding spike
Build Rust for aarch64-apple-darwin

Generate UniFFI Swift bindings

Create blank SwiftUI macOS app

Call dispatch(.play) and print returned AppUpdate

Goal: prove the Swift binding path before building UI.

Phase 3 — Playback manager
Bundle waterfalls.mp3

Implement MacPlaybackManager

Execute StartPlayback, PausePlayback, and SetVolume

Keep playback alive when the main window closes

Phase 4 — Daily-use UI
Main window

Menu bar extra

Timer presets

Custom timer sheet

Volume slider

Settings JSON persistence

Phase 5 — macOS polish
Launch at login

Keyboard shortcuts

Now Playing/media keys, best effort

Optional notifications when session completes

Optional Dock/menu-bar behavior preference

17. The macOS-specific architecture in one diagram
text
                         ┌──────────────────────────┐
                         │     cascade-core          │
                         │ Rust state machine        │
                         │ actions -> update         │
                         │ settings JSON             │
                         │ no audio APIs             │
                         └────────────┬─────────────┘
                                      │ UniFFI Swift
                                      ▼
┌────────────────────────────────────────────────────────────────┐
│                    cascade-macos SwiftUI                        │
│                                                                │
│  ┌─────────────────┐      ┌──────────────────────────────┐     │
│  │ MainWindowView  │      │ MenuBarView                  │     │
│  │ SettingsView    │─────▶│ CascadeStore                 │     │
│  └─────────────────┘      │ - snapshot                   │     │
│                           │ - dispatch(action)           │     │
│                           │ - execute(effects)           │     │
│                           └──────────────┬───────────────┘     │
│                                          │                     │
│              ┌───────────────────────────┼──────────────────┐  │
│              ▼                           ▼                  ▼  │
│  ┌─────────────────────┐     ┌──────────────────┐  ┌──────────────┐
│  │ MacPlaybackManager  │     │ SettingsStore    │  │ NowPlaying   │
│  │ AVAudioPlayer       │     │ settings.json    │  │ optional     │
│  └─────────────────────┘     └──────────────────┘  └──────────────┘
└────────────────────────────────────────────────────────────────┘
Final recommendation
Build Cascade for macOS as:

text
SwiftUI app
+ MenuBarExtra
+ UniFFI Swift binding to cascade-core
+ AVAudioPlayer-based MacPlaybackManager
+ settings.json persistence
+ Tick-driven session timer
+ optional Now Playing and launch-at-login polish
The most important architectural rule is the same as Clave: the Rust core owns the app’s truth, but macOS owns the experience.

# Opus 4.7 Thinking

Cascade — macOS Architecture
The macOS client is the third native shell on top of cascade-core, alongside the React/Vite web client and the Compose/Media3 Android client. Per your Clave tech stack rules, it must be genuinely native to the Apple ecosystem: SwiftUI as the UI framework, UniFFI Swift bindings to the Rust core, AVFoundation for audio playback, and zero shared UI code with Web or Android.

This document covers only the macOS layer. The Rust core, action/snapshot shapes, and effect protocol are defined in the main Cascade architecture brief and consumed unchanged here.

1. Platform Decisions
Concern	Choice	Rationale
Language	Swift (latest stable)	Your Clave stack mandates Swift/SwiftUI for Apple platforms.
UI framework	SwiftUI	Native, declarative, identical paradigm to the future iOS client.
Minimum target	macOS 14 Sonoma	@Observable macro, modern SwiftUI window scenes, modern AVFoundation APIs.
Architecture pattern	MV (Model-View) with @Observable	Aligns with current Apple guidance; lighter than MVVM for an app this size.
Core access	UniFFI-generated Swift bindings, packaged as an XCFramework	Same XCFramework you'll later extend for iOS — zero binding rework when the iOS client comes.
Audio engine	AVAudioEngine + AVAudioPlayerNode (loop via scheduled buffer)	Sample-accurate gapless looping, proper system audio routing, AirPlay/Bluetooth/CoreAudio integration.
Background playback	MPNowPlayingInfoCenter + MPRemoteCommandCenter	Native lock-screen / Control Center / menu-bar Now Playing integration.
Sleep / focus prevention	IOPMAssertion (kIOPMAssertionTypePreventUserIdleDisplaySleep is too aggressive — use PreventUserIdleSystemSleep)	Keeps the Mac from sleeping mid-Pomodoro without forcing the display awake.
Settings persistence	Single JSON file in Application Support/Cascade/settings.json	Matches the cross-platform contract — same JSON the web and Android shells write.
Audio asset	Bundled in the app bundle as Resources/cascade.mp3	No download path, works offline forever, identical UX to your current Spotify-local-files setup minus the failure mode.
Distribution	Notarized .app in a DMG, optionally Mac App Store later	Notarized DMG is enough for a personal-use daily tool; MAS adds sandboxing complexity you don't need yet.
Window model	Single WindowGroup + a MenuBarExtra for quick play/pause	Mirrors macOS apps like Lungo, Bartender — perfect for a daily background-noise tool.
2. Repo Layout (macOS Slice)
text
cascade/
├── core/                          # Rust headless core (shared)
├── bindings/
│   ├── uniffi/                    # cascade.udl → Swift, Kotlin, (later C#)
│   └── wasm/                      # wasm-bindgen for web
└── apps/
    ├── cascade-web/
    ├── cascade-android/
    └── cascade-apple/             # ← macOS lives here, iOS will follow
        ├── CascadeShared/         # Swift package shared with future iOS
        │   ├── Sources/
        │   │   ├── Core/          # Wraps UniFFI types in idiomatic Swift
        │   │   ├── Audio/         # AVFoundation engine (macOS + iOS share)
        │   │   ├── Persistence/   # JSON settings store
        │   │   ├── Models/        # AppStore (the @Observable root)
        │   │   └── Views/         # SwiftUI views shared with iOS
        │   └── Package.swift
        ├── CascadeMac/            # macOS-only target
        │   ├── CascadeMacApp.swift
        │   ├── Views/             # MenuBarExtra, Settings window, mac-specific chrome
        │   ├── System/            # IOPMAssertion wrapper, dock badge, etc.
        │   ├── Resources/
        │   │   └── cascade.mp3
        │   └── Cascade.entitlements
        └── XCFrameworks/
            └── CascadeCore.xcframework   # built from cargo + uniffi-bindgen
The split between CascadeShared and CascadeMac is the same shape Clave plans to use for ClaveShared / ClaveMac / ClaveiOS. When you eventually add cascade-ios, ~80% of CascadeShared carries over unchanged.

3. The Rust → Swift Boundary
The UDL surface stays identical to web and Android — the macOS shell only consumes it differently.

text
// bindings/uniffi/cascade.udl  (excerpt — same file Android uses)

namespace cascade {
    [Throws=CoreError]
    CoreHandle create(string manifest_json, string? settings_json);
};

interface CoreHandle {
    [Throws=CoreError]
    AppUpdate dispatch(AppCommand command);

    AppSnapshot snapshot();

    [Throws=CoreError]
    string export_settings();
};

dictionary AppUpdate {
    AppSnapshot snapshot;
    sequence<PlatformEffect> effects;
};

[Enum]
interface AppCommand {
    Play();
    Pause();
    TogglePlayback();
    SetVolume(u8 percent);
    StartPomodoro(PomodoroPreset preset);
    StartCustomTimer(u32 minutes);
    CancelTimer();
    PlatformPlaybackStarted();
    PlatformPlaybackPaused();
    PlatformPlaybackFailed(string message);
    Tick(u64 now_ms);
};

enum PomodoroPreset {
    "Thirty",
    "Sixty",
    "EightHours",
};

[Enum]
interface PlatformEffect {
    StartPlayback(string sound_id, boolean loop_forever, u8 volume_percent, u32 fade_in_ms);
    PausePlayback(u32 fade_out_ms);
    SetPlatformVolume(u8 volume_percent);
    ScheduleTimerFire(u64 trigger_at_ms);
    CancelScheduledTimer();
    PersistSettings(string json);
    UpdateNowPlaying(string title, string subtitle, boolean is_playing);
};
UniFFI gives Swift idiomatic enums with associated values for AppCommand and PlatformEffect, so on the Swift side you switch over them with full pattern matching — the FFI feels like Swift, not like C.

Wrapping the core in a Swift class
swift
// CascadeShared/Sources/Core/CoreBridge.swift
import Cascade  // generated UniFFI module

@MainActor
final class CoreBridge {
    private let handle: CoreHandle

    init(manifest: SoundManifest, persistedSettings: String?) throws {
        let manifestJSON = try JSONEncoder().encode(manifest)
        self.handle = try Cascade.create(
            manifestJson: String(data: manifestJSON, encoding: .utf8)!,
            settingsJson: persistedSettings
        )
    }

    func dispatch(_ command: AppCommand) throws -> AppUpdate {
        try handle.dispatch(command: command)
    }

    func snapshot() -> AppSnapshot { handle.snapshot() }
    func exportSettings() throws -> String { try handle.exportSettings() }
}
UniFFI's generated types are Sendable where appropriate; the bridge is @MainActor because every effect ultimately drives UI or AVAudioEngine, which want main-thread coordination.

Building the XCFramework
bash
## Rust targets for macOS (Apple Silicon + Intel)
cargo build --release --target aarch64-apple-darwin
cargo build --release --target x86_64-apple-darwin

## (Same XCFramework will later add aarch64-apple-ios + simulator targets)

## Generate Swift bindings + assemble XCFramework
uniffi-bindgen generate bindings/uniffi/cascade.udl --language swift \
    --out-dir apps/cascade-apple/CascadeShared/Sources/Core/Generated

xcodebuild -create-xcframework \
    -library target/aarch64-apple-darwin/release/libcascade.a \
        -headers .../Generated/cascadeFFI.h \
    -library target/x86_64-apple-darwin/release/libcascade.a \
        -headers .../Generated/cascadeFFI.h \
    -output apps/cascade-apple/XCFrameworks/CascadeCore.xcframework
Wire this into a Makefile target (make xcframework) so a single command rebuilds Rust + bindings + XCFramework before each Xcode build.

4. App Composition (@Observable Root)
The single source of truth on the Swift side is an AppStore that owns the snapshot and forwards commands to the core.

swift
// CascadeShared/Sources/Models/AppStore.swift
import Observation

@MainActor
@Observable
final class AppStore {
    private(set) var snapshot: AppSnapshot
    private let core: CoreBridge
    private let audio: AudioEngine
    private let nowPlaying: NowPlayingController
    private let timer: TimerScheduler
    private let persistence: SettingsStore
    private let powerAssertion: PowerAssertionController  // macOS only

    init(...) throws {
        self.snapshot = core.snapshot()
        // ... wire up effect handler
    }

    func send(_ command: AppCommand) {
        do {
            let update = try core.dispatch(command)
            self.snapshot = update.snapshot
            for effect in update.effects {
                handle(effect)
            }
        } catch {
            // route through PlatformPlaybackFailed
        }
    }

    private func handle(_ effect: PlatformEffect) {
        switch effect {
        case .startPlayback(let id, let loop, let vol, let fade):
            audio.start(soundId: id, loop: loop, volumePercent: vol, fadeInMs: fade)
            powerAssertion.acquire()  // keep Mac awake during a session
        case .pausePlayback(let fadeOut):
            audio.pause(fadeOutMs: fadeOut)
            powerAssertion.release()
        case .setPlatformVolume(let v):
            audio.setVolume(percent: v)
        case .scheduleTimerFire(let at):
            timer.schedule(triggerAtMs: at) { [weak self] in
                self?.send(.tick(nowMs: UInt64(Date().timeIntervalSince1970 * 1000)))
            }
        case .cancelScheduledTimer:
            timer.cancel()
        case .persistSettings(let json):
            try? persistence.write(json)
        case .updateNowPlaying(let title, let subtitle, let playing):
            nowPlaying.update(title: title, subtitle: subtitle, isPlaying: playing)
        }
    }
}
Compose subscribes to snapshot reactively — every dispatch produces a fresh snapshot, every snapshot drives a re-render, and platform side effects flow exactly once per dispatch. This is the macOS realization of the same dispatch → {snapshot, effects} contract used in the Web and Android clients.

5. Audio Engine (AVAudioEngine)
You said you've logged thousands of hours on this MP3, so audio quality matters. AVAudioEngine with a scheduled buffer gives you sample-accurate gapless looping — far better than AVAudioPlayer.numberOfLoops = -1, which has audible seam issues on some macOS builds.

swift
// CascadeShared/Sources/Audio/AudioEngine.swift
import AVFoundation

@MainActor
final class AudioEngine {
    private let engine = AVAudioEngine()
    private let player = AVAudioPlayerNode()
    private let mixer: AVAudioMixerNode
    private var buffer: AVAudioPCMBuffer?
    private var isLoaded = false

    init() {
        mixer = engine.mainMixerNode
        engine.attach(player)
        engine.connect(player, to: mixer, format: nil)
    }

    func loadIfNeeded(soundId: String) throws {
        guard !isLoaded else { return }
        guard let url = Bundle.main.url(forResource: "cascade", withExtension: "mp3") else {
            throw AudioError.assetMissing
        }
        let file = try AVAudioFile(forReading: url)
        let buf = AVAudioPCMBuffer(
            pcmFormat: file.processingFormat,
            frameCapacity: AVAudioFrameCount(file.length)
        )!
        try file.read(into: buf)
        self.buffer = buf
        isLoaded = true
    }

    func start(soundId: String, loop: Bool, volumePercent: UInt8, fadeInMs: UInt32) {
        do {
            try loadIfNeeded(soundId: soundId)
            if !engine.isRunning { try engine.start() }
            guard let buffer else { return }

            mixer.outputVolume = 0
            player.scheduleBuffer(
                buffer,
                at: nil,
                options: loop ? .loops : []
            )
            player.play()
            fadeVolume(to: Float(volumePercent) / 100.0, durationMs: fadeInMs)
        } catch {
            // bubble back to core via PlatformPlaybackFailed
        }
    }

    func pause(fadeOutMs: UInt32) {
        fadeVolume(to: 0, durationMs: fadeOutMs) { [weak self] in
            self?.player.pause()
        }
    }

    func setVolume(percent: UInt8) {
        mixer.outputVolume = Float(percent) / 100.0
    }

    private func fadeVolume(to target: Float, durationMs: UInt32, completion: (() -> Void)? = nil) {
        // CADisplayLink-style ramp on a Timer; small, ~16ms steps
        // (omitted for brevity — straightforward linear ramp on mixer.outputVolume)
    }
}
Why AVAudioEngine rather than AVAudioPlayer:

AVAudioPlayerNode.scheduleBuffer(_:at:options:) with .loops is genuinely seamless because the buffer is scheduled once and the engine handles the wraparound at the sample level. No re-trigger gap.

The mixer node gives you a clean, redraw-free volume fade.

Routes through CoreAudio properly — AirPlay, USB DACs, Bluetooth headphones, output device switching mid-session all just work.

Forward-compatible with effects (low-pass filter, EQ) if you ever want to tame the high end of the falls.

You can keep MP3 as the bundled format; AVAudioFile decodes it on load, and once decoded into a PCM buffer the loop boundary is exactly the file's sample count — none of the MP3-padding gap issues that plague Android MediaPlayer. (If you do hear a tiny seam, the fix is converting the source file to a CAF or AIFF, but try MP3 first.)

6. The Pomodoro Timer
The core owns when the timer should fire (deterministic, time-passed-in via Tick); the macOS shell owns the actual wall-clock scheduling and waking the core back up.

On the Rust side (sketch — same logic Android and Web use)
rust
pub enum PomodoroPreset { Thirty, Sixty, EightHours }

// In the reducer:
AppCommand::StartPomodoro(preset) => {
    let minutes = match preset {
        PomodoroPreset::Thirty => 30,
        PomodoroPreset::Sixty => 60,
        PomodoroPreset::EightHours => 480,
    };
    state.session.timer = Some(Timer { remaining_ms: minutes * 60_000, ... });
    state.desired_playback = DesiredPlayback::Playing;
    update.effects.push(PlatformEffect::StartPlayback { ... });
    update.effects.push(PlatformEffect::ScheduleTimerFire {
        trigger_at_ms: now_ms + minutes * 60_000,
    });
}

AppCommand::Tick { now_ms } => {
    if let Some(timer) = &state.session.timer {
        if now_ms >= timer.fires_at_ms {
            state.session.timer = None;
            state.desired_playback = DesiredPlayback::Paused;
            update.effects.push(PlatformEffect::PausePlayback { fade_out_ms: 4000 });
        }
    }
}
On the macOS side
swift
// CascadeShared/Sources/System/TimerScheduler.swift
@MainActor
final class TimerScheduler {
    private var task: DispatchWorkItem?

    func schedule(triggerAtMs: UInt64, fire: @escaping () -> Void) {
        cancel()
        let nowMs = UInt64(Date().timeIntervalSince1970 * 1000)
        let delaySec = max(0, Double(triggerAtMs - nowMs) / 1000)
        let work = DispatchWorkItem(block: fire)
        DispatchQueue.main.asyncAfter(deadline: .now() + delaySec, execute: work)
        self.task = work
    }

    func cancel() {
        task?.cancel()
        task = nil
    }
}
For an 8-hour prayer/work session, also acquire an IOPMAssertion:

swift
// CascadeShared/Sources/System/PowerAssertionController.swift
import IOKit.pwr_mgt

final class PowerAssertionController {
    private var assertionID: IOPMAssertionID = 0
    private var held = false

    func acquire(reason: String = "Cascade focus session in progress") {
        guard !held else { return }
        let result = IOPMAssertionCreateWithName(
            kIOPMAssertPreventUserIdleSystemSleep as CFString,
            IOPMAssertionLevel(kIOPMAssertionLevelOn),
            reason as CFString,
            &assertionID
        )
        if result == kIOReturnSuccess { held = true }
    }

    func release() {
        guard held else { return }
        IOPMAssertionRelease(assertionID)
        held = false
    }
}
PreventUserIdleSystemSleep is the right level: the display can still dim, but the Mac won't fall asleep at minute 47 of an 8-hour session. Acquire on StartPlayback, release on PausePlayback — this is a pure macOS concern, so it lives in the shell, not the core.

7. UI Surface
Two windows — both are appropriate for a daily-use Mac app of this kind.

7.1 MenuBarExtra (always-available, primary surface)
This is how you'll actually use it 95% of the time: click the menu-bar icon, hit play, pick a duration, get back to work.

swift
// CascadeMac/Views/MenuBarRoot.swift
import SwiftUI

struct MenuBarRoot: Scene {
    @Environment(AppStore.self) private var store

    var body: some Scene {
        MenuBarExtra {
            VStack(alignment: .leading, spacing: 8) {
                PlayPauseRow()
                Divider()
                VolumeSlider()
                Divider()
                TimerSection()
                Divider()
                Button("Open Cascade…") { NSApp.activate(ignoringOtherApps: true) }
                Button("Quit") { NSApp.terminate(nil) }
            }
            .padding(12)
            .frame(width: 260)
        } label: {
            Image(systemName: store.snapshot.isPlayingIntended
                  ? "drop.fill"
                  : "drop")
        }
        .menuBarExtraStyle(.window)
    }
}

struct TimerSection: View {
    @Environment(AppStore.self) private var store

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text("Focus Session").font(.caption).foregroundStyle(.secondary)
            HStack {
                preset("30m", .thirty)
                preset("60m", .sixty)
                preset("8h", .eightHours)
            }
            CustomDurationField()
            if let label = store.snapshot.timerLabel {
                Text(label).font(.caption.monospacedDigit())
            }
        }
    }

    @ViewBuilder
    private func preset(_ label: String, _ preset: PomodoroPreset) -> some View {
        Button(label) { store.send(.startPomodoro(preset: preset)) }
            .buttonStyle(.bordered)
    }
}
7.2 Main WindowGroup (settings + status)
Larger view for when you want to see the timer countdown clearly, adjust default fade durations, or tweak the custom-timer default. Since you said this app is for prayer and work sessions, an unobtrusive full window with a big countdown and a single "End Session" button is well worth having for the prayer use case in particular.

swift
@main
struct CascadeMacApp: App {
    @State private var store: AppStore

    init() {
        // bootstrap: load manifest, read persisted JSON, build store
        _store = State(initialValue: try! AppStore.bootstrap())
    }

    var body: some Scene {
        WindowGroup("Cascade") {
            MainWindowView()
                .environment(store)
                .frame(minWidth: 420, minHeight: 320)
        }
        .windowResizability(.contentSize)
        .commands {
            CommandGroup(replacing: .newItem) {}
            CommandMenu("Session") {
                Button("Start 30 min")  { store.send(.startPomodoro(preset: .thirty)) }
                    .keyboardShortcut("1", modifiers: [.command, .shift])
                Button("Start 60 min")  { store.send(.startPomodoro(preset: .sixty)) }
                    .keyboardShortcut("2", modifiers: [.command, .shift])
                Button("Start 8 hours") { store.send(.startPomodoro(preset: .eightHours)) }
                    .keyboardShortcut("3", modifiers: [.command, .shift])
                Divider()
                Button("Toggle Playback") { store.send(.togglePlayback) }
                    .keyboardShortcut(.space, modifiers: [])
            }
        }

        MenuBarRoot()
            .environment(store)
    }
}
Key bits:

Spacebar toggles playback when the main window is focused — the canonical media-app gesture.

⌘⇧1 / ⌘⇧2 / ⌘⇧3 start the three preset durations from anywhere in the app.

The menu-bar icon changes between drop and drop.fill based on isPlayingIntended — Apple's SF Symbols give you both for free.

7.3 Now Playing integration
swift
// CascadeShared/Sources/System/NowPlayingController.swift
import MediaPlayer

@MainActor
final class NowPlayingController {
    init(send: @escaping (AppCommand) -> Void) {
        let center = MPRemoteCommandCenter.shared()
        center.playCommand.addTarget { _ in send(.play); return .success }
        center.pauseCommand.addTarget { _ in send(.pause); return .success }
        center.togglePlayPauseCommand.addTarget { _ in send(.togglePlayback); return .success }
    }

    func update(title: String, subtitle: String, isPlaying: Bool) {
        var info: [String: Any] = [:]
        info[MPMediaItemPropertyTitle] = title
        info[MPMediaItemPropertyArtist] = subtitle
        info[MPNowPlayingInfoPropertyPlaybackRate] = isPlaying ? 1.0 : 0.0
        MPNowPlayingInfoCenter.default().nowPlayingInfo = info
    }
}
This puts Cascade in Control Center's media controls, the menu-bar Now Playing widget, and on AirPods double-press / keyboard play-pause keys. For a daily-driver background-noise app, this single integration is what makes the app feel like it belongs on macOS.

8. Persistence
swift
// CascadeShared/Sources/Persistence/SettingsStore.swift
import Foundation

final class SettingsStore {
    private let url: URL

    init() throws {
        let support = try FileManager.default.url(
            for: .applicationSupportDirectory,
            in: .userDomainMask, appropriateFor: nil, create: true
        )
        let dir = support.appendingPathComponent("Cascade", isDirectory: true)
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        self.url = dir.appendingPathComponent("settings.json")
    }

    func read() -> String? {
        guard let data = try? Data(contentsOf: url) else { return nil }
        return String(data: data, encoding: .utf8)
    }

    func write(_ json: String) throws {
        try json.data(using: .utf8)?.write(to: url, options: [.atomic])
    }
}
The Rust core decides what to persist (volume, default fade durations, custom-timer last-used minutes, default preset); the Swift shell decides where — ~/Library/Application Support/Cascade/settings.json. The JSON shape is shared with the web (localStorage) and Android (DataStore) clients, so a future "sync via iCloud Drive" feature is just a file-location swap.

9. Entitlements & Distribution
xml
<!-- Cascade.entitlements -->
<key>com.apple.security.app-sandbox</key>
<true/>
<key>com.apple.security.files.user-selected.read-only</key>
<true/>
<key>com.apple.security.network.client</key>
<false/>  <!-- explicit: this app never makes network calls -->
Sandbox: yes. Audio playback, MPNowPlayingInfoCenter, IOPMAssertion, and Application Support writes are all sandbox-compatible.

No network entitlement. This is a strong correctness signal — you literally cannot run into the Spotify-doesn't-load problem because the binary has no networking capability.

Hardened Runtime + Notarization. Required for the DMG to open without the Gatekeeper "damaged" warning.

Bundle ID: page.stephens.cascade (matches your stephens.page domain reverse-DNS).

Build and ship:

bash
xcodebuild -workspace Cascade.xcworkspace -scheme CascadeMac \
    -configuration Release archive -archivePath build/Cascade.xcarchive
xcodebuild -exportArchive -archivePath build/Cascade.xcarchive \
    -exportOptionsPlist export-options.plist -exportPath build/export
xcrun notarytool submit build/export/Cascade.app --keychain-profile "AC_NOTARY" --wait
xcrun stapler staple build/export/Cascade.app
create-dmg --volname Cascade build/export/Cascade.app build/Cascade.dmg
For your own daily use you can skip the DMG step entirely and just drag the notarized .app into /Applications.

10. Testing
Most of the correctness work happens in Rust (timer math, snapshot transitions, settings round-trip). The macOS-specific tests cover only the Swift shell:

AudioEngine unit tests — load buffer, schedule with .loops, verify mixer volume changes after a fade.

SettingsStore round-trip test — write JSON, read it back, hand it to CoreBridge, snapshot matches.

TimerScheduler test — schedule 100ms in the future, verify fires within tolerance.

AppStore integration test — dispatch StartPomodoro(.thirty) against a mocked AudioEngine/TimerScheduler, assert exactly one StartPlayback and one ScheduleTimerFire effect were executed.

UI smoke test (XCUITest) — launch app, click play in MenuBarExtra, assert Now Playing info contains "Cascade".

Skip property tests on Swift; those live in Rust where they're cheaper.

11. Build Order Within the macOS Track
Once cascade-core and the Web client are working, the macOS track is roughly:

Step	Deliverable
M1	xcframework build pipeline; Swift can call core.snapshot() from a dummy SwiftUI app
M2	AudioEngine plays the bundled MP3 in a loop; volume slider works
M3	AppStore wired to CoreBridge; play/pause from a SwiftUI button drives the audio engine via effects
M4	MenuBarExtra with play/pause + volume + three timer presets
M5	Pomodoro timer end-to-end (Tick scheduling, fade-out on completion, custom duration)
M6	Persistence (Application Support JSON)
M7	MPNowPlayingInfoCenter + remote commands + IOPMAssertion
M8	Main window with countdown, command menu shortcuts, polish
M9	Notarized DMG; install on your Mac and replace Spotify for one full work week
M1–M5 is enough to use the app. M6–M9 are what make it feel native.

12. What This Validates for Clave
This macOS slice rehearses three Clave concerns that the Web and Android shells alone don't fully cover:

The Apple XCFramework binding path — exactly the binary artifact Clave's iOS and macOS clients will both consume.

The shared-Swift-package pattern — CascadeShared is the dress rehearsal for Clave's ClaveShared, including how much UI you can genuinely share between iOS and macOS via #if os(macOS).

System integration as effects — MPNowPlayingInfoCenter, IOPMAssertion, and MenuBarExtra are all driven by PlatformEffects emitted from the core. If that pattern stays clean here, it'll stay clean when Clave needs to drive Multipeer Connectivity, Apple Game Center, and the macOS Game Bar from the same shape.

The Clave plan currently has macOS as Phase 7, "near-free after iOS." This Cascade slice validates that bet by proving the Apple binding and shared-package patterns work end-to-end before any Clave game-state code is ever written for them.

# Gemini 3.1 Pro Thinking

Here is the macOS architecture for Cascade, strictly following your Clave boundary rules.

Since this is a focus and utility app you will use daily for long stretches, the most native macOS paradigm is a Menu Bar App (a utility that lives in the top right of your screen and stays out of your Dock).

1. Repository Structure
We will add the macOS Xcode project alongside your web and Android folders, consuming the exact same UniFFI bindings that Android uses.

text
cascade/
  core/                     # Rust headless core
  apps/
    web/                    # React / Web Audio
    android/                # Compose / Media3
    macos/                  # SwiftUI / AVAudioPlayer
      Cascade.xcodeproj
      Cascade/
        CascadeApp.swift    # @main entry point (MenuBarExtra)
        CascadeViewModel.swift # Bridges Swift and Rust
        AudioEngine.swift   # Wraps AVAudioPlayer
        ContentView.swift   # The UI popup
        Assets/
          waterfall.wav     # The uncompressed audio file
2. The Golden Rule for macOS Audio: Convert to .wav
You noted logging thousands of hours on your specific 13-minute .mp3. However, mathematically, MP3s cannot loop seamlessly on Apple platforms. The MP3 encoder pads the beginning and end of the file with silent frames, which causes a noticeable micro-stutter every 13 minutes when using Apple's AVAudioPlayer.

The fix: Convert your 13-minute MP3 to a .wav or .caf file before dragging it into Xcode. AVAudioPlayer handles .wav looping perfectly and gaplessly.

3. The SwiftUI Menu Bar App (UI Layer)
macOS 13+ provides MenuBarExtra, which makes building menu bar utilities native and simple.

In your Info.plist, set Application is agent (UIElement) to YES. This hides Cascade from the Dock and the Cmd+Tab switcher, keeping it purely in the menu bar.

swift
@main
struct CascadeApp: App {
    @StateObject private var viewModel = CascadeViewModel()

    var body: some Scene {
        MenuBarExtra("Cascade", systemImage: viewModel.snapshot.isPlayingIntended ? "speaker.wave.3.fill" : "speaker.slash.fill") {
            ContentView(viewModel: viewModel)
        }
        .menuBarExtraStyle(.window) // Opens as a small popover window instead of a standard menu list
    }
}
4. The View Model (The FFI Bridge)
The ViewModel is the translator between your Swift UI and the UniFFI-generated Rust code. It holds the AppSnapshot, captures user intents, sends them to Rust, and executes the resulting PlatformEffects.

swift
class CascadeViewModel: ObservableObject {
    private var core: CascadeCore // Generated by UniFFI
    private let audioEngine = AudioEngine()
    private var tickTimer: Timer?
    
    @Published var snapshot: AppSnapshot
    
    init() {
        // 1. Load persisted settings from macOS UserDefaults
        let savedSettings = UserDefaults.standard.string(forKey: "cascade_settings")
        self.core = CascadeCore(settingsJson: savedSettings)
        self.snapshot = self.core.currentSnapshot()
        
        // 2. Start the 1-second wall-clock tick for the Pomodoro/Sleep Timer
        self.tickTimer = Timer.scheduledTimer(withTimeInterval: 1.0, repeats: true) { [weak self] _ in
            let nowMs = UInt64(Date().timeIntervalSince1970 * 1000)
            self?.dispatch(action: .tick(nowMs: nowMs))
        }
    }
    
    // UI Calls this
    func dispatch(action: AppCommand) {
        let update = core.dispatch(command: action)
        self.snapshot = update.snapshot
        execute(effects: update.effects)
    }
    
    private func execute(effects: [PlatformEffect]) {
        for effect in effects {
            switch effect {
            case .startPlayback(let volume):
                audioEngine.play(volume: volume)
            case .pausePlayback:
                audioEngine.pause()
            case .setPlatformVolume(let volume):
                audioEngine.setVolume(volume)
            case .persistSettings(let json):
                UserDefaults.standard.set(json, forKey: "cascade_settings")
            }
        }
    }
}
5. The Native Audio Engine (Platform Side-Effect)
Per your Clave rules, Rust knows nothing about audio. The macOS AudioEngine class blindly obeys the StartPlayback and PausePlayback effects.

swift
import AVFoundation

class AudioEngine {
    private var player: AVAudioPlayer?
    
    init() {
        guard let url = Bundle.main.url(forResource: "waterfall", withExtension: "wav") else { 
            print("Asset not found")
            return 
        }
        do {
            player = try AVAudioPlayer(contentsOf: url)
            player?.numberOfLoops = -1 // Native gapless infinite loop
            player?.prepareToPlay()
        } catch {
            print("Failed to initialize AVAudioPlayer: \(error)")
        }
    }
    
    func play(volume: Float) {
        player?.volume = volume
        player?.play()
    }
    
    func pause() {
        player?.pause()
    }
    
    func setVolume(_ volume: Float) {
        player?.volume = volume
    }
}
6. The Pomodoro Timer & App Nap
Because this is a Pomodoro timer, you want options for 30m, 60m, 8h, or custom durations.

The architectural boundary:

You click "30 Minutes" in the SwiftUI popover.

Swift calls dispatch(action: .setSleepTimer(minutes: 30)).

Rust calculates the target end time internally and updates the AppSnapshot to show "29:59 remaining".

Swift's Timer.scheduledTimer ticks every second, passing the current Unix timestamp to Rust via .tick(nowMs).

When Rust sees nowMs >= target_end_time, the Rust reducer outputs a PlatformEffect::PausePlayback.

Swift receives the effect and stops the AVAudioPlayer.

Important macOS Gotcha (App Nap):
macOS puts background apps to sleep ("App Nap") to save battery, which will throttle your Swift Timer to tick only once every 10+ seconds. To prevent this, when the timer is active, you must tell macOS that your app is doing important work:

swift
var activity: NSObjectProtocol?

func preventAppNap() {
    activity = ProcessInfo.processInfo.beginActivity(
        options: [.userInitiatedAllowingIdleSystemSleep, .latencyCritical], 
        reason: "Pomodoro Timer Tracking"
    )
}

func allowAppNap() {
    if let activity = activity {
        ProcessInfo.processInfo.endActivity(activity)
    }
}
Note: Audio playback naturally prevents the Mac from sleeping, but explicitly requesting latencyCritical ensures your 1-second UI ticks don't lag when the popover is closed.

Summary of macOS Parity
This beautifully mirrors your Clave architecture:

UI: MenuBarExtra is the macOS equivalent of Jetpack Compose.

Audio: AVAudioPlayer is the macOS equivalent of Android's Media3 ExoPlayer.

State: Rust is completely unaware it is running on a Mac. It just consumes .tick() and spits out .startPlayback or .pausePlayback via UniFFI.

Storage: Rust outputs raw JSON strings, and macOS simply shovels them into UserDefaults.