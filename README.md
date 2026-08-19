# Cascade

A waterfall white-noise player with **one headless Rust core driving six
native shells** — web, Android, macOS, Windows, iOS, and watchOS. Built as a
"Clave architecture kata": the Rust core owns all app behavior (state,
pomodoro/sleep timers, volume, serialized settings, and time via explicit
`Tick` events) and never touches an audio API. Each platform shell owns the
side effects — audio playback, OS integration, and UI.

The sound is a 13-minute recording of a waterfall. Cascade exists to play that
one file, reliably, everywhere — without depending on a streaming service.

**Live web app: <https://cascade.stephens.page>** (installable PWA).
Pushes to `main` deploy it via [`.github/workflows/deploy-web.yml`](.github/workflows/deploy-web.yml).

**Get Cascade (end-user installs): <https://cascade.stephens.page/apps>**

[![Windows build](https://github.com/JacobStephens2/cascade/actions/workflows/windows.yml/badge.svg)](https://github.com/JacobStephens2/cascade/actions/workflows/windows.yml)
[![Apple build](https://github.com/JacobStephens2/cascade/actions/workflows/apple.yml/badge.svg)](https://github.com/JacobStephens2/cascade/actions/workflows/apple.yml)

## Platforms

| Platform | Stack | Rust binding | Status |
|---|---|---|---|
| **Web** | React + Vite PWA, Web Audio API | `wasm-bindgen` | Live player ([cascade.stephens.page](https://cascade.stephens.page)); installable PWA |
| **Android** | Kotlin + Compose, Media3 `MediaSessionService` | UniFFI (Kotlin) | Sideload APK ([get it](https://cascade.stephens.page/apps)) |
| **Windows** | WinUI 3 / C# / .NET, `MediaPlayer` + SMTC | C ABI + P/Invoke | Unsigned zip ([get it](https://cascade.stephens.page/apps)) |
| **macOS** | SwiftUI, `MenuBarExtra`, AVAudioEngine | UniFFI (Swift) | Notarized DMG ([get it](https://cascade.stephens.page/apps)) |
| **iOS** | SwiftUI, AVAudioSession background audio | UniFFI (Swift) | TestFlight beta ([get it](https://cascade.stephens.page/apps)) |
| **watchOS** | SwiftUI thin remote over WatchConnectivity | — (talks to iPhone) | Comes with the iPhone app |

End-user installs live on [cascade.stephens.page/apps](https://cascade.stephens.page/apps).
The web app is deployed; native apps are GitHub (or TestFlight) releases, not
store listings. CI still compiles every shell (see the badges). Building from
source is the section below.

## Features

- Play / pause a seamless waterfall loop
- Volume with a perceptual (square-law) curve, consistent across platforms
- **Focus sessions** (30 / 60 min, 8 hr) and **sleep timers** (15 / 30 / 60 min)
- **Custom durations** — pick any length with a Focus/Sleep mode toggle
- Persisted settings (same JSON schema on every platform)
- Native media integration per platform: lock-screen / Now Playing /
  System Media Transport Controls, and sleep-prevention during long sessions

## Architecture (the Clave kata)

![Cascade architecture - one Rust core driving six native shells over a single dispatch(Command) -> { Snapshot, Effects } boundary](docs/cascade-core-architecture.png)

- **Headless core.** `cascade-core` (Rust) owns intent and state; platforms own
  side effects (audio output, OS integration, UI). The core has no audio,
  filesystem, or clock dependencies.
- **Coarse-grained API.** One `dispatch(Command) -> Update { snapshot, effects }`
  call. No chatty getters across the FFI boundary.
- **Snapshots out, commands in.** UIs render from a `Snapshot`; user actions and
  platform events flow back as `Command`s.
- **Effects, not callbacks.** The core returns a list of `Effect`s the platform
  executes — easy to test, replay, and reason about.
- **Time is an input.** The core never reads the system clock; the UI ticks it
  with wall-clock deltas.
- **One JSON wire shape, multiple bindings.** `wasm-bindgen` (web), UniFFI
  (Kotlin/Swift), and a hand-rolled C ABI (C#) all serialize the same commands,
  effects, and snapshots.

```
            ┌───────────────────────────┐
            │        cascade-core        │  Rust: state machine, reducer,
            │   Command → {Snapshot,     │  timers, settings. No I/O.
            │              Effect[]}     │
            └─────────────┬─────────────┘
         wasm-bindgen │ UniFFI │ C ABI
   ┌────────────┬─────┴──┬────┴────────┬──────────────┐
  Web        Android   macOS/iOS/    Windows      (watchOS talks to
  (PWA)     (Compose)  watchOS       (WinUI 3)     iPhone via
                       (SwiftUI)                   WatchConnectivity)
```

## Repo layout

```
cascade/
├── crates/
│   ├── cascade-core/      # Rust state machine + reducer + snapshot + timers
│   ├── cascade-wasm/      # wasm-bindgen wrapper (web)
│   └── cascade-uniffi/    # UniFFI bridge (Kotlin/Swift) + C ABI (C#)
├── apps/
│   ├── web/               # React + Vite PWA, Web Audio engine
│   ├── android/           # Kotlin + Compose + Media3
│   ├── apple/             # SwiftUI: CascadeShared + CascadeMac/iOS/Watch
│   └── windows/           # WinUI 3 / C# / .NET
├── docs/                  # architecture briefs + per-platform runbooks
├── deploy/                # web build + Apache vhost for cascade.stephens.page
└── .github/workflows/     # windows.yml, apple.yml CI
```

## Quick start (web)

```bash
# Build WASM bindings (whenever the core changes), then run the dev server
cd crates/cascade-wasm
wasm-pack build --target web --out-dir ../../apps/web/src/wasm
cd ../../apps/web && npm install && npm run dev
```

## Building the other platforms

- **Android** — `apps/android`, open in Android Studio or
  `./gradlew :app:assembleDebug` (needs the Android SDK + NDK).
- **Windows** — `apps/windows`, run `scripts/build.ps1` then open
  `Cascade.sln`. See [`apps/windows/README.md`](apps/windows/README.md).
- **macOS / iOS / watchOS** — `apps/apple`, run `scripts/build.sh` then open
  the generated `Cascade.xcodeproj`. See [`apps/apple/README.md`](apps/apple/README.md)
  and the runbooks in `docs/`.

## Documentation

- Architecture diagram: [`docs/cascade-core-architecture.svg`](docs/cascade-core-architecture.svg) - the rendered overview above (PNG alongside for embedding).
- [`docs/architecture-brief-web-and-android.md`](docs/architecture-brief-web-and-android.md) — original design rationale
- Per-platform architecture: [macOS](docs/macos-app-architecture.md) ·
  [Windows](docs/windows-app-architecture.md) · [iOS](docs/ios-app-architecture.md) ·
  [watchOS](docs/watchos-architecture.md)
- Runbooks: [macOS](docs/running-macos.md) · [iOS](docs/running-ios.md) ·
  [watchOS](docs/running-watchos.md)
