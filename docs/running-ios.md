# Running Cascade on iOS

Runbook for the iPhone / iPad target. The shared layout is in
[apps/apple/README.md](../apps/apple/README.md); the architecture is in
[ios-app-architecture.md](./ios-app-architecture.md).

The iOS app sits alongside the macOS app in the same Xcode project, sharing
every line of non-UI code through `CascadeShared/`. If you already
[built CascadeMac](./running-macos.md), the only new piece is selecting the
iOS scheme.

---

## 1. Setup additions

If you only set up the macOS path before, add the iOS Rust targets:

```bash
rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios
```

Everything else (Xcode, ffmpeg, xcodegen) is already in place from the
macOS instructions.

## 2. Build

```bash
cd apps/apple
./scripts/build.sh ios     # only the iOS Rust slices + asset + xcodegen
# or
./scripts/build.sh         # macOS + iOS
open Cascade.xcodeproj
```

In Xcode:

1. Switch the scheme to **CascadeiOS**.
2. Pick a destination — an iOS simulator (e.g. *iPhone 16 Pro*) or your
   physical iPhone if it's plugged in.
3. **Signing & Capabilities** → set a Team. Free Apple-ID signing is fine
   for personal-device install. Bundle ID is `page.stephens.cascade`.
4. ⌘R.

## 3. What you should see

- Edge-to-edge dark backdrop, big circular Play button in the middle,
  volume slider below it.
- "Focus session" row: 30 min / 60 min / 8 hr buttons.
- "Sleep timer" row: 15 / 30 / 60 min buttons. "Cancel timer" appears
  while one is running.
- Lock the phone or background the app — audio keeps playing.
  Lock-screen and Control Center show "Cascade — Waterfall focus sound"
  with play/pause buttons.
- AirPods double-tap / headphone play-pause toggles playback.

## 4. Day-to-day iteration

| You changed... | Run |
|---|---|
| Anything iOS-specific in `CascadeiOS/` | ⌘R |
| Anything shared in `CascadeShared/` | ⌘R (both targets rebuild) |
| Rust core / bindings | `./scripts/build-rust.sh ios` then ⌘R |
| `waterfall.ogg` | `./scripts/build-asset.sh` then ⌘R |

## 5. Installing on your phone for daily use

Free signing (Apple-ID Team) gives you a 7-day install. Plug the phone
in, ⌘R from Xcode, and the app sticks until the signature expires.

For a permanent install you need a paid developer account ($99/year) and
TestFlight:

```bash
xcodebuild -project apps/apple/Cascade.xcodeproj \
    -scheme CascadeiOS \
    -configuration Release \
    -archivePath /tmp/CascadeiOS.xcarchive archive
xcodebuild -exportArchive \
    -archivePath /tmp/CascadeiOS.xcarchive \
    -exportOptionsPlist apps/apple/ios-export.plist \
    -exportPath /tmp/CascadeiOS-export
xcrun altool --upload-app -f /tmp/CascadeiOS-export/Cascade.ipa \
    --apiKey $ASC_KEY_ID --apiIssuer $ASC_ISSUER_ID
```

## 6. Troubleshooting

**`No such module 'cascade_uniffiFFI'`** at Swift compile time
The iOS-specific include path didn't pick up `CascadeShared/Generated/`.
Confirm `SWIFT_INCLUDE_PATHS` in `project.yml` (under the iOS target's
`settings.base`) is `$(SRCROOT)/CascadeShared/Generated`.

**`Undefined symbol: _cascade_new`** (or other C-ABI symbols) at link time
The SDK-specific `libcascade_uniffi.a` didn't land in the expected place.
Re-run `./scripts/build-rust.sh ios` and check:
- `apps/apple/build/rust/ios-device/libcascade_uniffi.a` for on-device
  builds.
- `apps/apple/build/rust/ios-sim/libcascade_uniffi.a` for simulator
  builds. Apple Silicon simulators link the arm64-ios-sim slice;
  pre-Apple-Silicon Macs link the x86_64-ios slice. The build script
  lipos both into one universal archive.

**Audio cuts out the moment the screen locks**
`UIBackgroundModes: audio` is missing from `CascadeiOS/Info.plist`, or
the iOS AVAudioSession category isn't `.playback`. Both should be there;
double-check the file before reaching for `xcodegen generate` (which
rewrites the Xcode project but not the Info.plist).

**`AVAudioFile not found` / silence on first play**
The bundled asset isn't in the Copy Bundle Resources phase. Open the
CascadeiOS target → Build Phases → Copy Bundle Resources, and make sure
`waterfall.m4a` is listed. If not, re-run `./scripts/build-asset.sh` and
`xcodegen generate`.

**Lock screen doesn't show Cascade in Now Playing**
This is "best-effort" on iOS — if other audio apps (Spotify, Podcasts)
have recently held the Now Playing slot, the system can take a few
seconds to switch over. Pausing and resuming Cascade usually wins it
back.
