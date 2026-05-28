# Running Cascade on macOS

Step-by-step guide to going from a fresh `git clone` to a running app.

The architecture is in [macos-app-architecture.md](./macos-app-architecture.md);
the project layout is in [apps/macos/README.md](../apps/macos/README.md). This
doc is just the runbook.

---

## 1. One-time setup

Run on your Mac (Apple Silicon or Intel).

```bash
# Xcode command-line tools — gives you swiftc, lipo, codesign, xcodebuild.
# If Xcode.app is already installed you can skip this.
xcode-select --install

# Rust toolchain + Apple cross-compile targets
brew install rustup-init
rustup-init -y
rustup target add aarch64-apple-darwin x86_64-apple-darwin

# Project-build tooling
brew install xcodegen ffmpeg
```

You also need a full Xcode install (not just CLT) to actually open
`Cascade.xcodeproj` and run the app. Install from the App Store, then run
`sudo xcode-select -s /Applications/Xcode.app/Contents/Developer` once so the
command-line tools point at it.

## 2. Build

```bash
git clone git@github.com:JacobStephens2/cascade.git    # if you don't already have it
cd cascade/apps/macos
./scripts/build.sh
```

What that script does, in order:

1. `build-asset.sh` — converts `apps/web/public/sounds/waterfall.ogg` to
   `CascadeMac/Resources/waterfall.m4a` via ffmpeg. Cached: if the M4A is
   newer than the OGG it's a no-op.
2. `build-rust.sh` — `cargo build --release` for `aarch64-apple-darwin` and
   `x86_64-apple-darwin`, `lipo`s them into a universal
   `apps/macos/build/rust/libcascade_uniffi.a`, then regenerates the Swift
   bindings into `CascadeMac/Generated/`. Subsequent runs hit cargo's cache.
3. `xcodegen generate` — rebuilds `Cascade.xcodeproj` from `project.yml`.

First build takes ~3–5 minutes (most of that is cargo compiling the Rust
crates twice). After that, incremental rebuilds are 5–20 seconds.

## 3. Open and run in Xcode

```bash
open Cascade.xcodeproj
```

Before you hit ⌘R the first time:

1. Select the **Cascade** target in the file navigator.
2. **Signing & Capabilities** tab.
3. Pick a Team from the dropdown. Your personal Apple-ID team is fine — the
   sandbox + hardened-runtime entitlements just need *some* signing identity.
   You don't need a paid developer account for local-only running.
4. ⌘R.

The first launch hits Gatekeeper since the bundle is locally signed and not
notarized. If macOS refuses with "Cascade cannot be opened," open
**System Settings → Privacy & Security**, scroll to the bottom, click
**Open Anyway**. (Subsequent runs are fine.)

## 4. What you should see

- A drop icon in the menu bar (top-right). Click it for the daily-use
  surface: play/pause, volume, three focus presets (30 min / 60 min / 8 hr),
  three sleep presets (15 / 30 / 60 min), and a "Cancel timer" link when one
  is running.
- A main window opens automatically. It mirrors the menu-bar controls plus a
  large countdown when a timer is active.
- A **Settings** scene under `Cascade ▸ Settings…` (⌘,) — toggle
  "Launch at login" and reveal the persisted `settings.json` in Finder.

## 5. Day-to-day iteration

| You changed... | Run |
|---|---|
| A Swift file | ⌘R in Xcode |
| A Rust file (`cascade-core`, `cascade-uniffi`) | `./scripts/build-rust.sh` then ⌘R |
| The Rust public API (added a command/effect/snapshot field) | `./scripts/build-rust.sh` (regenerates Swift bindings) then update `Dto.swift` to match, then ⌘R |
| `waterfall.ogg` | `./scripts/build-asset.sh` then ⌘R |
| `project.yml` | `xcodegen generate` then reopen the project |

## 6. Installing for daily use

For a one-Mac-only install (no notarization required):

```bash
# In Xcode: Product ▸ Archive → Distribute App → Copy App
# Or from CLI:
xcodebuild -project apps/macos/Cascade.xcodeproj \
    -scheme Cascade \
    -configuration Release \
    -archivePath /tmp/Cascade.xcarchive archive
xcodebuild -exportArchive \
    -archivePath /tmp/Cascade.xcarchive \
    -exportOptionsPlist apps/macos/export-options.plist \
    -exportPath /tmp/Cascade-export

cp -R /tmp/Cascade-export/Cascade.app /Applications/
```

(You'll need to write your own `export-options.plist`; the Xcode UI is the
easier path.)

Then drag `/Applications/Cascade.app` into **System Settings → General →
Login Items** if you want it to come up on boot, or toggle it from inside
the app's Settings scene.

## 7. Troubleshooting

**`error: failed to find tool 'cc' for target aarch64-apple-darwin`**
You're missing the Xcode command-line tools or `xcode-select` is pointing
somewhere wrong. Re-run `sudo xcode-select -s /Applications/Xcode.app/Contents/Developer`.

**`error: linker cc not found` during cargo build**
Same fix.

**`No such module 'cascade_uniffiFFI'`** at Swift compile time
The modulemap isn't on Swift's include path. Check that
`CascadeMac/Generated/cascade_uniffiFFI.modulemap` exists and that
`SWIFT_INCLUDE_PATHS` in `project.yml` still points at
`$(SRCROOT)/CascadeMac/Generated`. If you renamed anything, regenerate
with `xcodegen generate`.

**`Undefined symbol: _uniffi_cascade_uniffi_fn_...`** at link time
`libcascade_uniffi.a` didn't get linked. Check:
- `apps/macos/build/rust/libcascade_uniffi.a` exists and is non-empty
  (`file` should report it as a Mach-O universal archive).
- `OTHER_LDFLAGS` in `project.yml` still has `-L$(SRCROOT)/build/rust
  -lcascade_uniffi`.

**`Cascade.app is damaged and can't be opened`** at launch
The bundle is unsigned or the signature is broken. Re-run from Xcode (it
re-signs on every build). If you copied a Release build to `/Applications`,
ad-hoc sign it: `codesign --force --deep --sign - /Applications/Cascade.app`.

**`Couldn't update login item`** in Settings
Launch-at-login via `SMAppService.mainApp` requires the app to be in
`/Applications` (not running from Xcode's DerivedData). Build a Release,
move it to `/Applications`, run it once, then toggle the setting.

**No sound, no error message**
Open Console.app, filter for "Cascade". The `AudioEngine` logs to
`os_log`. Most likely cause is the bundled asset wasn't found — check that
`CascadeMac/Resources/waterfall.m4a` exists and is referenced by the Cascade
target's Build Phases ▸ Copy Bundle Resources.
