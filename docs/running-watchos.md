# Running Cascade on Apple Watch

The architecture is in [watchos-architecture.md](./watchos-architecture.md);
the shared layout is in [apps/apple/README.md](../apps/apple/README.md).
This is the runbook for the Mode A (thin remote) v1.

The watch app is **embedded inside the iOS app**, not installed
independently. It's a SwiftUI watchOS 10 target that talks to the iPhone
over `WCSession`. No Rust core on the watch, no audio engine on the
watch — every command goes to the phone and the phone pushes a compact
snapshot back.

---

## 1. Setup additions

If you already have iOS building, the only watch-specific addition is
the watchOS deployment target in Xcode (which `xcodegen` writes for you)
and an Apple Watch paired to your iPhone. No new Rust targets are
needed — the watch is pure Swift.

## 2. Build

```bash
cd apps/apple
./scripts/build.sh         # rebuilds the project with all three targets
open Cascade.xcodeproj
```

In Xcode:

1. Pick the **CascadeiOS** scheme.
2. Set the destination to your physical iPhone (the simulator can run
   the watch app but pairing-flow testing is sketchy on the sim).
3. **Signing & Capabilities** → set a Team on both **CascadeiOS** and
   **CascadeWatch**. Free Apple-ID signing works.
4. ⌘R. The iOS app installs first; the Watch app shows up under the
   companion **Watch** app on iPhone → **Available Apps** → **Install**.

## 3. What you should see

On the watch (vertical-paged scroll between three pages):

1. **Player** — status line on top ("Playing · 42:17 left"), oversized
   play/pause in the middle, volume readout at the bottom. Digital
   Crown adjusts volume.
2. **Session** — three preset rows (30 min / 60 min / 8 hours). Tap one
   to start a focus session; the iPhone runs the timer and pushes the
   countdown back. A "Cancel timer" button appears while one is active.
3. **Status** — green iPhone glyph + "Connected" or grey + "iPhone
   unreachable", plus a Refresh button to pull a fresh snapshot.

Haptics fire on every tap so the wrist confirms each action.

## 4. Day-to-day iteration

| You changed... | Run |
|---|---|
| A `CascadeWatch/` Swift file | ⌘R on the **CascadeiOS** scheme — Xcode rebuilds the embedded watch app and pushes it. |
| `CascadeShared/Watch/WatchProtocol.swift` | Both targets rebuild — make sure the Codable shape on both sides stays in sync (same file is compiled into both). |
| `CascadeiOS/Connectivity/` | Rebuild iOS only; the watch keeps using the cached snapshot until the next push. |

## 5. Troubleshooting

**Watch shows "Connecting…" forever**
`PhoneConnectivityService` isn't activated, or `WCSession` activation
hasn't completed. Open Console.app, filter for "Cascade" and look for
`WCSession activation failed` logs. Most common cause is the iOS app
hasn't been launched once since install — the watch can't talk to a
suspended companion app, so launch CascadeiOS once to wake the session.

**Commands work but the snapshot is stale**
`onSnapshotChanged` isn't being called. Check
`apps/apple/CascadeShared/App/AppStore.swift` — `apply(_:)` should call
`onSnapshotChanged?(update.snapshot)` on every update. If you added a
new dispatch path, make sure it routes through `apply`.

**Volume crown rotates but doesn't change anything**
The setVolume push hit the loop guard — the iPhone replied with the
same percent we already had locally, so the
`guard intValue != conn.snapshot.volumePercent` early-returns. This
only matters during initial sync; in normal use it prevents an infinite
echo. If real changes don't propagate, the iPhone-side
`PhoneConnectivityService.handle(.setVolume)` isn't dispatching — check
the `Command.setVolume(percent:)` case there.

**Watch app doesn't appear in the Watch app on iPhone**
Xcodegen embedded the watch target via the iOS dependency, but Xcode
sometimes needs a clean rebuild to write the right
`WKCompanionAppBundleIdentifier`. Try **Product → Clean Build Folder**
(⇧⌘K) and rebuild. If that doesn't fix it, open the iOS target's Build
Phases → confirm `CascadeWatch.app` is listed under **Embed Watch
Content**.

**`No such module 'WatchConnectivity'`** at Swift compile time
Only `CascadeShared/Watch/WatchProtocol.swift` is compiled into the
watch target — the rest of `CascadeShared/` pulls in IOKit/AVFoundation
headers that aren't on watchOS. The `sources:` list in `project.yml`
points specifically at `CascadeShared/Watch`, not the whole shared
tree. If you added a watch source somewhere else, move it into
`CascadeShared/Watch/` or into `CascadeWatch/`.

## 6. What's NOT here yet

- **Standalone audio on the watch** (Mode B from the architecture doc).
  Would need:
  - Rust core cross-compiled for `aarch64-apple-watchos` (+ sim slices).
  - `WKBackgroundModes: audio` in the watch Info.plist.
  - `AVAudioSession.RouteSharingPolicy.longFormAudio`.
  - A bundled audio asset in `CascadeWatch/Resources/`.
- **WidgetKit complications** — glanceable session state on the watch
  face / Smart Stack via App Group `UserDefaults`. Phase w3 in the
  architecture doc.
- **App Intents** for one-tap "Start 30 min focus session" from the
  watch face Smart Stack.
- **`UNUserNotificationCenter` scheduling** so the iPhone can wake the
  watch with a session-complete haptic when audio happens to be paused.
