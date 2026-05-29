# Cascade — Apple (macOS + iOS + watchOS)

Native SwiftUI shells around `cascade-core`. macOS and iOS share every
line of non-UI code through `CascadeShared/`. watchOS is a thin remote
(Mode A) that talks to the iPhone over `WCSession`; it doesn't link the
Rust core.

## Layout

```
apps/apple/
├── project.yml                        # XcodeGen spec, three targets
├── scripts/
│   ├── build.sh                       # one-shot: assets + Rust + xcodegen
│   ├── build-asset.sh                 # OGG → M4A via ffmpeg (writes both targets)
│   └── build-rust.sh [all|macos|ios]  # cargo cross-compile per Apple triple
├── CascadeShared/                     # Compiled into macOS + iOS targets
│   ├── App/                           # AppStore, CoreBridge, Dto
│   ├── Audio/AudioEngine.swift        # AVAudioEngine + sample-accurate loop
│   ├── Persistence/SettingsStore.swift
│   ├── System/                        # PowerAssertion, AppNapGuard, NowPlayingController
│   ├── Watch/WatchProtocol.swift      # WCSession Codable types (compiled into all 3 targets)
│   └── Generated/                     # UniFFI Swift + header + modulemap (committed)
├── CascadeMac/                        # macOS-only target
│   ├── CascadeMacApp.swift            # @main + WindowGroup + MenuBarExtra + Settings
│   └── Views/{MainWindowView,MenuBarRoot,SettingsView}.swift
├── CascadeiOS/                        # iOS-only target
│   ├── CascadeiOSApp.swift            # @main + AVAudioSession + Watch wiring
│   ├── Views/CascadeScreen.swift
│   └── Connectivity/                  # PhoneConnectivityService + WatchSnapshotMapper
└── CascadeWatch/                      # watchOS-only target (embedded in CascadeiOS)
    ├── CascadeWatchApp.swift
    ├── Views/                         # Vertical-paged player / session / status
    └── Services/                      # WatchConnectivityClient + WatchHaptics
```

## Build prerequisites (Mac)

```bash
# Xcode command-line tools (swiftc, lipo, codesign, xcodebuild)
xcode-select --install

# Homebrew tooling
brew install xcodegen ffmpeg

# Rust + every Apple target you want to build for
brew install rustup-init && rustup-init -y
rustup target add \
    aarch64-apple-darwin x86_64-apple-darwin \
    aarch64-apple-ios \
    aarch64-apple-ios-sim x86_64-apple-ios
```

You also need a full Xcode install (App Store) to build / run on the
iOS simulator or sign on-device.

## First build

```bash
cd apps/apple
./scripts/build.sh        # all of: asset, macOS Rust, iOS Rust, bindings, project
open Cascade.xcodeproj
```

In Xcode:

1. Pick the scheme: **CascadeMac**, **CascadeiOS**, or **CascadeWatch**.
2. **Signing & Capabilities** → pick your Team (the iOS target's Team is
   inherited by the embedded watch app).
3. ⌘R.

The watch app is bundled inside the iOS app — there's no separate
install path. Build/run **CascadeiOS** on your iPhone with your Apple
Watch paired, and the Watch app shows up in the Watch app on iPhone
under "Available Apps" → tap Install.

For iOS-only iteration on a Mac that's only used to develop iOS:

```bash
./scripts/build.sh ios
```

## Architectural choices

| Concern | Owner |
|---|---|
| State machine, timer math, settings schema, snapshots | `cascade-core` (Rust, shared) |
| Audio playback | `AudioEngine` — AVAudioEngine + scheduleBuffer(.loops). Configures `AVAudioSession.playback` on iOS; no-op on macOS. |
| Persistence | `Application Support/Cascade/settings.json` (same path both platforms). |
| Power / wake lock | `PowerAssertion`: `IOPMAssertion` on macOS, `isIdleTimerDisabled` on iOS. |
| Tick survival | `AppNapGuard`: `ProcessInfo.beginActivity` on macOS; no-op on iOS (UIBackgroundModes: audio handles it). |
| Media keys | `NowPlayingController` — `MPNowPlayingInfoCenter` + `MPRemoteCommandCenter`. |
| UI | `CascadeMac/Views/` (MenuBarExtra + main window + Settings); `CascadeiOS/Views/CascadeScreen.swift` (single full-screen view); `CascadeWatch/Views/` (vertical-paged player + session + status). |
| Watch ↔ Phone | `WCSession`. The watch sends `WatchToPhoneCommand`s; the iPhone replies with `PhoneSnapshotForWatch`. iPhone owns all state. |

## Day-to-day iteration

| You changed... | Run |
|---|---|
| A Swift file | ⌘R in Xcode |
| A Rust file in `cascade-core` / `cascade-uniffi` | `./scripts/build-rust.sh` then ⌘R |
| `waterfall.ogg` | `./scripts/build-asset.sh` then ⌘R |
| `project.yml` | `xcodegen generate` then reopen the project |

## Known follow-ups

- iOS lacks Live Activities / Dynamic Island integration — phase 2
  polish per the architecture council doc.
- The iOS app has no separate Settings UI; macOS still has the standard
  Settings scene.
- No App Store / TestFlight pipeline yet — for personal-device install,
  Xcode's free signing is enough.
