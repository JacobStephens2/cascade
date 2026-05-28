# Cascade — macOS

Native SwiftUI shell around `cascade-core`, mirroring the web and Android
clients. Same JSON wire shape across all three platforms.

## What's here

```
apps/macos/
├── project.yml                      # XcodeGen project spec
├── scripts/
│   ├── build.sh                     # one-shot: assets + Rust + Xcode project
│   ├── build-asset.sh               # OGG → M4A via ffmpeg
│   └── build-rust.sh                # cargo-ndk-style universal lipo + bindings
└── CascadeMac/
    ├── CascadeMacApp.swift          # @main + WindowGroup + MenuBarExtra + Settings
    ├── App/
    │   ├── AppStore.swift           # @Observable single source of truth
    │   ├── CoreBridge.swift         # wraps UniFFI bridge in typed Swift
    │   └── Dto.swift                # Codable mirrors of cascade-core JSON
    ├── Audio/AudioEngine.swift      # AVAudioEngine + sample-accurate loop
    ├── Persistence/SettingsStore.swift   # ~/Library/Application Support/Cascade/settings.json
    ├── System/
    │   ├── PowerAssertion.swift     # IOPMAssertion (no sleep during sessions)
    │   ├── AppNapGuard.swift        # ProcessInfo.beginActivity (timer ticks survive backgrounding)
    │   └── NowPlayingController.swift  # MPNowPlayingInfoCenter + remote commands
    ├── Views/
    │   ├── MainWindowView.swift
    │   ├── MenuBarRoot.swift        # the 95%-case daily surface
    │   └── SettingsView.swift       # launch-at-login + settings.json revealer
    ├── Generated/                   # UniFFI Swift bindings (committed)
    ├── Resources/                   # waterfall.m4a (built, gitignored)
    ├── Info.plist
    └── Cascade.entitlements         # sandboxed, no network
```

## Build prerequisites (Mac)

```bash
# Xcode command-line tools (gives you swiftc, lipo, codesign, xcodebuild)
xcode-select --install

# Homebrew tooling
brew install xcodegen ffmpeg

# Rust + Apple targets
brew install rustup-init && rustup-init -y
rustup target add aarch64-apple-darwin x86_64-apple-darwin
```

## First build

```bash
cd apps/macos
./scripts/build.sh
open Cascade.xcodeproj
```

In Xcode:

1. Select the **Cascade** scheme.
2. **Signing & Capabilities** → pick a Team. (Personal team is fine; the
   sandbox entitlement requires *some* signing identity.)
3. ⌘B to build, ⌘R to run.

After the first build, day-to-day iteration is just ⌘R from Xcode unless
you change Rust code; then re-run `./scripts/build-rust.sh`.

## What lives where

| Concern | Owner |
|---|---|
| State machine, timer math, settings schema, snapshots | `cascade-core` (Rust, shared across all platforms) |
| Audio playback | `AudioEngine.swift` — AVAudioEngine + PCM buffer loop |
| Persistence path | `SettingsStore.swift` — Application Support JSON |
| Power / App Nap | `PowerAssertion.swift`, `AppNapGuard.swift` |
| Media keys | `NowPlayingController.swift` — `MPNowPlayingInfoCenter` |
| UI | SwiftUI views in `Views/` |

The Rust core declares **intent** (`Effect.startPlayback { volumePercent }`);
the macOS shell decides **how** (AVAudioEngine, with a square-law volume
curve identical to the web shell).

## Why these choices

- **AVAudioEngine + scheduleBuffer(.loops)** instead of `AVAudioPlayer.numberOfLoops = -1`:
  the buffer is decoded once into PCM and looped at the sample boundary, so
  the seam is silent regardless of any AAC/MP3 padding in the source. You
  said you've logged thousands of hours on this file — that 13-minute seam
  matters.
- **Application Support JSON** instead of `UserDefaults`: keeps the settings
  blob a real, inspectable artifact that round-trips through the same Rust
  schema the web (`localStorage`) and Android (`DataStore`) clients already
  use.
- **MenuBarExtra + main window** rather than menu-bar-only: easier to debug
  and discover during development. Convert to `LSUIElement` later once the
  workflow's settled.
- **Sandboxed, no network entitlement**: this app can't make network calls.
  That's a defensive guarantee against the Spotify-can't-load failure mode
  the project exists to solve.

## Known follow-ups

- Apple Team ID is empty in `project.yml`; fill it in after first build so
  CI / fresh checkouts pick up signing.
- The custom-timer input in the main window is a minimal text field; not
  validated beyond `Int(_) > 0`.
- Now Playing remote-command priority on macOS is best-effort.
- No notarized DMG packaging yet — for personal use, drag the built
  `Cascade.app` into `/Applications`.
