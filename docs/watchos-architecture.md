# Perplexity Model Council Synthesis

### 1. Where Models Agree

| Finding | GPT-5.5 Thinking | Claude Opus 4.7 Thinking | Gemini 3.1 Pro Thinking | Evidence |
|---------|-----------|-----------|-----------|----------|
| watchOS app should be **native SwiftUI** and optimized for wrist interactions | ✓ | ✓ | ✓ | Modern watch apps are built with SwiftUI; watch UX should be glanceable and crown-first. [perplexity](https://www.perplexity.ai/search/b0a0932b-d65b-4c69-99a8-ac6d9b31cb49) |
| For **standalone audio** on Watch, use **Background Audio** capability and an **AVAudioSession playback** configuration | ✓ | ✓ | ✓ | Apple’s watchOS background audio guidance requires enabling background audio; AVAudioSession is the OS interface for audio behavior. [ppl-ai-file-upload.s3.us-east-1.amazonaws](https://ppl-ai-file-upload.s3.us-east-1.amazonaws.com/web/direct-files/attachments/130224663/e826d6d9-002d-4f5c-8a8c-2b9f967bd923/clave-tech-stack.md?X-Amz-Algorithm=AWS4-HMAC-SHA256&X-Amz-Checksum-Mode=ENABLED&X-Amz-Credential=ASIA2F3EMEYERJGFTQTK%2F20260529%2Fus-east-1%2Fs3%2Faws4_request&X-Amz-Date=20260529T023648Z&X-Amz-Expires=3600&X-Amz-Security-Token=IQoJb3JpZ2luX2VjEPL%2F%2F%2F%2F%2F%2F%2F%2F%2F%2FwEaCXVzLWVhc3QtMSJHMEUCIC%2FwdwA3qkVNay%2Fr%2F%2F%2BSDYDIIvRimdRwYO%2BZCpBs%2FXGgAiEAvtZeKbRimF%2B%2BJmfcVY%2Fy4bfarYKM%2Fqn0U7BxPoi9SUQq9AQIu%2F%2F%2F%2F%2F%2F%2F%2F%2F%2F%2FARABGgw2OTk3NTMzMDk3MDUiDOzfvbgeYQgkYZDNaSrIBAZHbhGiCq%2BkYRSJPtpWkwuw8HznPkNaha8P9Xwswu%2BtPFj7OA8pQ9%2F8bakDzhbBPFOrsG8eYpIadnSwVyf8nRA9h9ikuFfav7HBCw2rz8fMEOtCS8ZoR%2FIjmHJ0KlmdJBVDBeZ9WBq%2FuoVMhM6pUi%2B7coMCKjkmEWemD%2Bv0gqK6mIzpMOmuwhDwE4zZNUN4nOoj%2FFd5iMfxFZUJO%2FUuvtwE7Pu5X47ru2oV4ZrSHT5vYVmviSEd1nhSa%2F6WoArZ1ikCVpKAgiT0jJSURCbSir1NdVenktORY3fJrHTYYOEMBOboBFNU1JCDmrkBZd5EgT%2F9BBltAC5tfY4e7gidKxeZRH5p8CiyXJm206qVc72uItgfWzcRJHXqrOTCdp4THJ71rTullSYLXFtAbvknrsVpgpABIY6Y5oJX08GhRMbu%2FriKgn5SQ6j82A5kOnkPFuRFPWA8qAVgT1sI%2B0HR%2B4g3%2BIpRmBiflhXYlnYu%2BPfqia8ijMQ3F64KWbdvRjcgWX00IOnGeryn%2F5pEuFyIwx0s0ll96Ns7%2BlshjUm1mgIbMJfneWJVl5sjt4kUvbijR76LDHm8pyfQOy%2FxSBPWiDUW6SYWXeLBO9XEPPmlDeu3W5GCX%2F5Sn16s%2B9JVW6nUsP1gtF60suT3eJYsTmG4XQkRrQwCFZhCS024W4rJpe9ADsuV6oQlS6i1RS5xkYbXDNS5c51ZfAc7sNLJ2Z3v4Uj6vvn4s2CT8YIZIu51Lbs9DScddvN00c133kyzRz0lLxJsFHm5c6WEMLLm49AGOpgBMmP7O%2BsdyCv6dHfpQVY20A1BOhDfz1dplCQQoi37luvmjcJLIUytNBEbZ4sTrVnwMnWTu1bPi5352sT2Hp5OfC3cLyipMyEaFNN7KHVY0mjvAapPubu%2FLrLiP3h5npVHxW6nzDUzEkeDbhjn7xaF8GReG5arO3eAaYFtWMTiImQwitLdBvuGyXMoO%2Bj9z2%2BtclN1mEH%2FW%2F0%3D&X-Amz-SignedHeaders=host&x-id=GetObject&X-Amz-Signature=13f48742ae6b2f0f3fa193ea663c77ebf8f26f36d4d3611923e7970419b3ef87) |
| For looping playback, **AVQueuePlayer + AVPlayerLooper** is a strong default (gapless, OS-managed looping) | ✓ |  | ✓ | AVPlayerLooper loops a single AVPlayerItem with AVQueuePlayer. [b4x](https://www.b4x.com/android/forum/threads/gapless-playback-with-android-mediaplayer.62420/) |
| Provide **complications / widgets** via WidgetKit for glanceable status (e.g., “playing / time remaining”) | ✓ | ✓ | ✓ | watch complications are built with WidgetKit; they’re timeline-driven rather than real-time. [youtube](https://www.youtube.com/watch?v=3yHSLXmc6JM) |
| Use **WatchConnectivity** for phone↔watch sync (state, commands); don’t assume shared storage | ✓ | ✓ |  | UserDefaults aren’t shared; WatchConnectivity is the standard paired-device transfer path. [perplexity](https://www.perplexity.ai/search/c38a5812-9eae-481c-bffe-3b19e90dbbcf) |

***

### 2. Where Models Disagree

| Topic | GPT-5.5 Thinking | Claude Opus 4.7 Thinking | Gemini 3.1 Pro Thinking | Why They Differ |
|-------|-----------|-----------|-----------|-----------------|
| Primary product mode | **Hybrid**: Remote-control iPhone + optional standalone playback | **Remote-control iPhone first** (ship v1), standalone later | **Standalone watch app** as the primary | Different assumptions about what you want from watchOS: quickest usefulness (remote) vs phone-free audio (standalone). |
| Shared “core truth” location | Prefers Swift-only on watch v1; iPhone owns truth in remote mode | Strongly prefers watch as a **thin remote** (no core on watch v1) | Prefers shared core on watch too | GPT/Claude optimize for shipping + reduced watch constraints; Gemini optimizes for “watch is a full client.” |
| Audio session policy details | Emphasizes “Bluetooth route required for long-form” UX states | Focuses on remote mode; avoids watch audio constraints | Calls out `.longFormAudio` route-sharing policy as the key to background playback | Gemini anchors on the `.longFormAudio` policy docs; GPT anchors on Apple’s statement that watchOS needs an eligible (often Bluetooth) route for long-form background audio. [ppl-ai-file-upload.s3.us-east-1.amazonaws](https://ppl-ai-file-upload.s3.us-east-1.amazonaws.com/web/direct-files/attachments/130224663/e826d6d9-002d-4f5c-8a8c-2b9f967bd923/clave-tech-stack.md?X-Amz-Algorithm=AWS4-HMAC-SHA256&X-Amz-Checksum-Mode=ENABLED&X-Amz-Credential=ASIA2F3EMEYERJGFTQTK%2F20260529%2Fus-east-1%2Fs3%2Faws4_request&X-Amz-Date=20260529T023648Z&X-Amz-Expires=3600&X-Amz-Security-Token=IQoJb3JpZ2luX2VjEPL%2F%2F%2F%2F%2F%2F%2F%2F%2F%2FwEaCXVzLWVhc3QtMSJHMEUCIC%2FwdwA3qkVNay%2Fr%2F%2F%2BSDYDIIvRimdRwYO%2BZCpBs%2FXGgAiEAvtZeKbRimF%2B%2BJmfcVY%2Fy4bfarYKM%2Fqn0U7BxPoi9SUQq9AQIu%2F%2F%2F%2F%2F%2F%2F%2F%2F%2F%2FARABGgw2OTk3NTMzMDk3MDUiDOzfvbgeYQgkYZDNaSrIBAZHbhGiCq%2BkYRSJPtpWkwuw8HznPkNaha8P9Xwswu%2BtPFj7OA8pQ9%2F8bakDzhbBPFOrsG8eYpIadnSwVyf8nRA9h9ikuFfav7HBCw2rz8fMEOtCS8ZoR%2FIjmHJ0KlmdJBVDBeZ9WBq%2FuoVMhM6pUi%2B7coMCKjkmEWemD%2Bv0gqK6mIzpMOmuwhDwE4zZNUN4nOoj%2FFd5iMfxFZUJO%2FUuvtwE7Pu5X47ru2oV4ZrSHT5vYVmviSEd1nhSa%2F6WoArZ1ikCVpKAgiT0jJSURCbSir1NdVenktORY3fJrHTYYOEMBOboBFNU1JCDmrkBZd5EgT%2F9BBltAC5tfY4e7gidKxeZRH5p8CiyXJm206qVc72uItgfWzcRJHXqrOTCdp4THJ71rTullSYLXFtAbvknrsVpgpABIY6Y5oJX08GhRMbu%2FriKgn5SQ6j82A5kOnkPFuRFPWA8qAVgT1sI%2B0HR%2B4g3%2BIpRmBiflhXYlnYu%2BPfqia8ijMQ3F64KWbdvRjcgWX00IOnGeryn%2F5pEuFyIwx0s0ll96Ns7%2BlshjUm1mgIbMJfneWJVl5sjt4kUvbijR76LDHm8pyfQOy%2FxSBPWiDUW6SYWXeLBO9XEPPmlDeu3W5GCX%2F5Sn16s%2B9JVW6nUsP1gtF60suT3eJYsTmG4XQkRrQwCFZhCS024W4rJpe9ADsuV6oQlS6i1RS5xkYbXDNS5c51ZfAc7sNLJ2Z3v4Uj6vvn4s2CT8YIZIu51Lbs9DScddvN00c133kyzRz0lLxJsFHm5c6WEMLLm49AGOpgBMmP7O%2BsdyCv6dHfpQVY20A1BOhDfz1dplCQQoi37luvmjcJLIUytNBEbZ4sTrVnwMnWTu1bPi5352sT2Hp5OfC3cLyipMyEaFNN7KHVY0mjvAapPubu%2FLrLiP3h5npVHxW6nzDUzEkeDbhjn7xaF8GReG5arO3eAaYFtWMTiImQwitLdBvuGyXMoO%2Bj9z2%2BtclN1mEH%2FW%2F0%3D&X-Amz-SignedHeaders=host&x-id=GetObject&X-Amz-Signature=13f48742ae6b2f0f3fa193ea663c77ebf8f26f36d4d3611923e7970419b3ef87) |
| How much to rely on complications for countdown | Coarse updates only | Coarse updates only | Coarse updates only | Mostly aligned; minor framing differences. Complications can’t be relied on for real-time countdown. [stackoverflow](https://stackoverflow.com/questions/29882907/how-to-seamlessly-loop-sound-with-web-audio-api) |

***

### 3. Unique Discoveries

| Model | Unique Finding | Why It Matters |
|-------|----------------|----------------|
| Gemini 3.1 Pro Thinking | Use `AVAudioSession.RouteSharingPolicy.longFormAudio` for watch background audio | This is a watchOS-specific lever that can be the difference between “keeps playing wrist-down” and “gets suspended.” |
| Claude Opus 4.7 Thinking | Make v1 a **thin remote**: watch shows snapshot + sends commands; avoid duplicated timer logic | Minimizes battery/toolchain/review risk while still delivering the “wrist control” value immediately. [perplexity](https://www.perplexity.ai/search/c38a5812-9eae-481c-bffe-3b19e90dbbcf) |

***

### 4. Comprehensive Analysis

**High-Confidence Findings.** All models converge that a watchOS version of Cascade must be a **native SwiftUI** experience designed around ultra-fast interactions: big play/pause, crown-driven adjustments, and glanceable status rather than a shrunken phone UI. If you support **standalone playback on the Watch**, you must treat Apple Watch like a constrained audio device: enable **Background Audio** on the watch target and configure **AVAudioSession** for playback so audio can continue when the wrist drops. For looping, the safest modern approach is **AVQueuePlayer + AVPlayerLooper**, which is purpose-built to loop a single media item without manual “restart at end” glue. Finally, complications/widgets should be built with **WidgetKit** and designed for coarse status, because complications are timeline-based and not guaranteed to update continuously like a second-by-second timer. [developer.android](https://developer.android.com/media/media3/exoplayer/playlists)

**Areas of Divergence.** The key product/architecture decision is whether watchOS Cascade is primarily (A) a **remote control** for the iPhone app, (B) a **standalone watch audio player**, or (C) a **hybrid**. Claude Opus 4.7 argues strongly for Mode A first: it’s the lowest-risk way to get real daily value—one-tap wrist control of the iPhone session—while avoiding watchOS audio routing constraints and battery impact. GPT-5.5 also likes the hybrid concept, but still positions “remote-first” as the pragmatic default and makes the Bluetooth-route constraints explicit as a UI state you must handle if you add standalone mode. Gemini 3.1, by contrast, treats watchOS as a full client and recommends implementing the shared reducer/effects model on-watch, with standalone audio as the main feature. [ppl-ai-file-upload.s3.us-east-1.amazonaws](https://ppl-ai-file-upload.s3.us-east-1.amazonaws.com/web/direct-files/attachments/130224663/e826d6d9-002d-4f5c-8a8c-2b9f967bd923/clave-tech-stack.md?X-Amz-Algorithm=AWS4-HMAC-SHA256&X-Amz-Checksum-Mode=ENABLED&X-Amz-Credential=ASIA2F3EMEYERJGFTQTK%2F20260529%2Fus-east-1%2Fs3%2Faws4_request&X-Amz-Date=20260529T023648Z&X-Amz-Expires=3600&X-Amz-Security-Token=IQoJb3JpZ2luX2VjEPL%2F%2F%2F%2F%2F%2F%2F%2F%2F%2FwEaCXVzLWVhc3QtMSJHMEUCIC%2FwdwA3qkVNay%2Fr%2F%2F%2BSDYDIIvRimdRwYO%2BZCpBs%2FXGgAiEAvtZeKbRimF%2B%2BJmfcVY%2Fy4bfarYKM%2Fqn0U7BxPoi9SUQq9AQIu%2F%2F%2F%2F%2F%2F%2F%2F%2F%2F%2FARABGgw2OTk3NTMzMDk3MDUiDOzfvbgeYQgkYZDNaSrIBAZHbhGiCq%2BkYRSJPtpWkwuw8HznPkNaha8P9Xwswu%2BtPFj7OA8pQ9%2F8bakDzhbBPFOrsG8eYpIadnSwVyf8nRA9h9ikuFfav7HBCw2rz8fMEOtCS8ZoR%2FIjmHJ0KlmdJBVDBeZ9WBq%2FuoVMhM6pUi%2B7coMCKjkmEWemD%2Bv0gqK6mIzpMOmuwhDwE4zZNUN4nOoj%2FFd5iMfxFZUJO%2FUuvtwE7Pu5X47ru2oV4ZrSHT5vYVmviSEd1nhSa%2F6WoArZ1ikCVpKAgiT0jJSURCbSir1NdVenktORY3fJrHTYYOEMBOboBFNU1JCDmrkBZd5EgT%2F9BBltAC5tfY4e7gidKxeZRH5p8CiyXJm206qVc72uItgfWzcRJHXqrOTCdp4THJ71rTullSYLXFtAbvknrsVpgpABIY6Y5oJX08GhRMbu%2FriKgn5SQ6j82A5kOnkPFuRFPWA8qAVgT1sI%2B0HR%2B4g3%2BIpRmBiflhXYlnYu%2BPfqia8ijMQ3F64KWbdvRjcgWX00IOnGeryn%2F5pEuFyIwx0s0ll96Ns7%2BlshjUm1mgIbMJfneWJVl5sjt4kUvbijR76LDHm8pyfQOy%2FxSBPWiDUW6SYWXeLBO9XEPPmlDeu3W5GCX%2F5Sn16s%2B9JVW6nUsP1gtF60suT3eJYsTmG4XQkRrQwCFZhCS024W4rJpe9ADsuV6oQlS6i1RS5xkYbXDNS5c51ZfAc7sNLJ2Z3v4Uj6vvn4s2CT8YIZIu51Lbs9DScddvN00c133kyzRz0lLxJsFHm5c6WEMLLm49AGOpgBMmP7O%2BsdyCv6dHfpQVY20A1BOhDfz1dplCQQoi37luvmjcJLIUytNBEbZ4sTrVnwMnWTu1bPi5352sT2Hp5OfC3cLyipMyEaFNN7KHVY0mjvAapPubu%2FLrLiP3h5npVHxW6nzDUzEkeDbhjn7xaF8GReG5arO3eAaYFtWMTiImQwitLdBvuGyXMoO%2Bj9z2%2BtclN1mEH%2FW%2F0%3D&X-Amz-SignedHeaders=host&x-id=GetObject&X-Amz-Signature=13f48742ae6b2f0f3fa193ea663c77ebf8f26f36d4d3611923e7970419b3ef87)

Those different recommendations come from different risk models. A remote-first watch app depends on **WatchConnectivity**: the watch sends commands, the phone owns playback/timers/settings, and the watch renders a “snapshot.” This aligns with the reality that iPhone and Watch storage aren’t shared and syncing must be explicit. A standalone watch app avoids phone reachability issues, but it forces you to solve the hardest watchOS problem immediately: long-form background playback and routing. Apple’s guidance notes that watchOS long-form background audio requires an eligible route (commonly Bluetooth), and you should design the app to prompt/handle that rather than failing silently. Gemini adds a crucial detail: using the `.longFormAudio` route-sharing policy is specifically intended for watchOS apps that need background playback, and it explicitly ties permission to the ability to activate an eligible route. [perplexity](https://www.perplexity.ai/search/c38a5812-9eae-481c-bffe-3b19e90dbbcf)

**Unique Insights Worth Noting.** Gemini’s `.longFormAudio` callout is worth elevating: even if you only ship standalone mode later, design your audio session service so you can switch policies cleanly for watchOS. Claude’s “thin remote” approach is also strategically strong if your goal is to ship something you’ll actually use quickly: it lets the iPhone remain the single source of truth for timer math, audio policy, and settings persistence, while the watch becomes the fastest control surface.

**Recommendations.** Ship watchOS Cascade in two phases:

1) **v1: Remote-control companion (recommended starting point).**  
SwiftUI watch app + WatchConnectivity. Watch sends `play/pause/toggle/startTimer/cancel/setVolume` commands; iPhone returns a compact snapshot. Don’t duplicate timer logic on the watch; just display it. Use WidgetKit complications fed by the last-known snapshot. [youtube](https://www.youtube.com/watch?v=3yHSLXmc6JM)

2) **v2: Optional standalone playback mode.**  
Add a watch-bundled low-bitrate audio asset, implement `AVAudioSession` playback with `.longFormAudio` policy, enable background audio, and play via `AVQueuePlayer + AVPlayerLooper`. Add explicit UX for “no eligible audio route” (e.g., “Connect AirPods”). [b4x](https://www.b4x.com/android/forum/threads/gapless-playback-with-android-mediaplayer.62420/)

# GPT 5.5 Thinking

Recommendation
Architect Cascade for watchOS as a native SwiftUI Apple Watch app with two modes:

Companion Remote Mode — the Watch controls Cascade running on the iPhone.

Standalone Watch Playback Mode — the Watch plays a local low-bitrate waterfall audio asset directly through AirPods / Bluetooth audio.

That gives the best watchOS experience because Apple Watch is both a wrist remote and an independent audio device, but it has stricter background/audio-route constraints than iPhone or Mac. In particular, Apple’s watchOS background-audio guidance says long-form audio requires a Bluetooth audio route, so the watchOS app must gracefully handle “Connect AirPods or Bluetooth audio” as a first-class state.

The architectural rule should stay consistent with your macOS/iOS/Windows Cascade direction:

Cascade’s state machine owns truth; watchOS owns Watch-specific playback, routing, haptics, complications, and phone sync.

1. Product Shape
The watchOS app should not try to be a shrunken iPhone app. It should be optimized for wrist interactions:

text
Main screen:
  Cascade
  [ Play / Pause ]

  Mode:
    iPhone
    Watch

  Timer:
    ∞   30   60   8h

  Status:
    Playing on iPhone
    or
    Playing on Watch · AirPods connected
Key use cases:

Start/pause Cascade without opening the phone.

Start a 30/60/8h session from the wrist.

See remaining time as a complication / Smart Stack widget.

Use the Watch as a standalone waterfall player when away from the phone.

Avoid awkward failure modes when no Bluetooth audio route is available.

2. Platform Stack
Concern	Choice
UI	SwiftUI for watchOS
State	@Observable / ObservableObject store with dispatch(action) -> snapshot + effects
Companion sync	WatchConnectivity
Standalone audio	AVFoundation: AVQueuePlayer + AVPlayerLooper, or AVAudioPlayer if you choose simplicity
Background audio	Watch app Background Modes: audio
Audio session	AVAudioSession with playback category
Route handling	Require / request Bluetooth output before long-form standalone playback
Settings	Local watch settings cache; sync from iPhone via WatchConnectivity
Complications	WidgetKit watch complications
Haptics	WKInterfaceDevice.current().play(...) behind a haptics service
Notifications	Local notification for session complete, if useful
Rust core	Optional; I recommend not embedding Rust in watchOS v1 unless you specifically want this to rehearse Clave-style full-platform core reuse
3. Big Architectural Decision: Companion-First, Standalone-Second
Mode A — Companion Remote Mode
This should be the default mode.

text
Watch UI
  -> send command to iPhone via WatchConnectivity
  -> iPhone Cascade dispatches action
  -> iPhone returns latest snapshot
  -> Watch renders snapshot
This mode has major advantages:

No duplicated audio asset required on Watch.

No Bluetooth route requirement on Watch.

Better battery life.

Plays through whatever iPhone is already using: phone speaker, Bluetooth speaker, AirPods, HomePod, etc.

Works naturally with the iOS app’s existing audio session and Now Playing integration.

Use WatchConnectivity for paired-device data transfer; Apple positions WatchConnectivity as the mechanism for transferring data between paired iPhone and Watch apps.
 Do not expect iPhone and Watch UserDefaults to be shared; the Watch app is sandboxed separately, so settings/snapshots must be explicitly synchronized.

Mode B — Standalone Watch Playback Mode
This is the “phone-free” mode.

text
Watch UI
  -> dispatch action locally
  -> WatchPlaybackService activates AVAudioSession
  -> route selection / Bluetooth validation
  -> play bundled waterfall_watch.m4a in a loop
This mode is valuable, but it needs clear constraints:

It requires AirPods or another Bluetooth audio route for long-form audio.

It should use a Watch-optimized audio file, e.g. HE-AAC / M4A at 48–64 kbps.

It should be built around background audio mode in the Watch target.

It should prioritize battery and reliability over perfect fidelity.

Apple’s watchOS audio-streaming guidance says Watch apps configured for background audio need the Audio background mode on the WatchKit extension target, and playback should use AVFoundation once enough local/streamed content is available.
 For Cascade, prefer local bundled audio instead of streaming, because the waterfall loop is fixed and a local asset avoids network stalls.

4. Recommended Repo Layout
Assuming your Apple apps are grouped together:

text
cascade/
├── apps/
│   └── cascade-apple/
│       ├── Cascade.xcworkspace
│       │
│       ├── CascadeShared/
│       │   ├── Package.swift
│       │   └── Sources/CascadeShared/
│       │       ├── Core/
│       │       │   ├── CascadeAction.swift
│       │       │   ├── CascadeSnapshot.swift
│       │       │   ├── CascadeEffect.swift
│       │       │   ├── CascadeSettings.swift
│       │       │   └── CascadeReducer.swift
│       │       ├── Views/
│       │       │   ├── PlayPauseGlyph.swift
│       │       │   ├── TimerPreset.swift
│       │       │   └── SharedFormatting.swift
│       │       └── Services/
│       │           └── SettingsCodec.swift
│       │
│       ├── CascadeiOS/
│       │   ├── CascadeiOSApp.swift
│       │   ├── Services/
│       │   │   ├── iOSPlaybackService.swift
│       │   │   ├── PhoneConnectivityService.swift
│       │   │   └── NowPlayingService.swift
│       │   └── Resources/
│       │       └── waterfall.mp3
│       │
│       ├── CascadeWatch/
│       │   ├── CascadeWatchApp.swift
│       │   ├── WatchStore.swift
│       │   ├── Views/
│       │   │   ├── WatchMainView.swift
│       │   │   ├── WatchModePicker.swift
│       │   │   ├── WatchTimerPresetBar.swift
│       │   │   ├── RouteUnavailableView.swift
│       │   │   └── WatchSettingsView.swift
│       │   ├── Services/
│       │   │   ├── WatchEffectExecutor.swift
│       │   │   ├── WatchPlaybackService.swift
│       │   │   ├── WatchAudioSessionService.swift
│       │   │   ├── WatchConnectivityService.swift
│       │   │   ├── WatchSettingsStore.swift
│       │   │   ├── WatchHapticsService.swift
│       │   │   └── WatchComplicationBridge.swift
│       │   └── Resources/
│       │       └── waterfall_watch.m4a
│       │
│       └── CascadeWatchComplications/
│           ├── CascadeComplicationBundle.swift
│           ├── CascadeComplicationWidget.swift
│           └── CascadeComplicationProvider.swift
Use CascadeShared for pure Swift types and view fragments that can safely run on iOS, macOS, and watchOS. Keep all Watch-specific APIs—WatchConnectivity, complications, haptics, Watch audio route handling—inside CascadeWatch.

5. Should watchOS Use the Rust Core?
My recommendation: No Rust in watchOS v1
For iOS/macOS/Windows, a Rust core can be worth it because the platforms have fewer binary-size and build-complexity constraints. For watchOS, Cascade is simple enough that v1 should avoid Rust unless you specifically want to prove a full Clave-style watch target.

Use a small Swift reducer that mirrors the same action/snapshot/effect architecture:

swift
dispatch(WatchAction) -> WatchUpdate(snapshot, effects)
Reasons:

Faster to ship.

Smaller Watch binary.

Less toolchain friction.

Easier App Store review/debugging.

Better fit for Watch’s highly constrained UI.

Companion mode can delegate actual “truth” to the iPhone anyway.

When to embed Rust
Embed Rust only if you want the Watch app to become a fully independent Cascade client whose timer/settings/session logic must be byte-for-byte identical to iOS/Android/Windows.

If you do that, treat it as a separate spike:

text
Phase W0.5:
  Build cascade-core for watchOS device + simulator.
  Generate UniFFI Swift bindings.
  Package watchOS slices into the Apple XCFramework.
  Call dispatch(.togglePlayback) from a blank Watch app.
Do not let that block the first useful Watch release.

6. Core State Model
Even if you implement the Watch state machine in Swift, keep the same conceptual shape as the rest of Cascade.

swift
enum WatchCascadeMode: String, Codable {
    case phoneRemote
    case watchStandalone
}

struct WatchSnapshot: Codable, Equatable {
    var mode: WatchCascadeMode
    var isPlaying: Bool
    var activeDeviceLabel: String
    var volumePercent: Int
    var selectedTimer: TimerPreset
    var sessionEndsAt: Date?
    var remainingText: String?
    var routeState: WatchRouteState
    var phoneReachability: PhoneReachability
    var errorMessage: String?
}

enum WatchRouteState: Codable, Equatable {
    case unknown
    case available
    case needsBluetoothAudio
}

enum PhoneReachability: Codable, Equatable {
    case reachable
    case notReachable
    case notPaired
}
Actions:

swift
enum WatchAction: Equatable {
    case appLaunched
    case becameActive
    case wentInactive

    case togglePlaybackTapped
    case playTapped
    case pauseTapped

    case modeSelected(WatchCascadeMode)
    case timerSelected(TimerPreset)
    case volumeChanged(Int)

    case tick(Date)

    case phoneSnapshotReceived(PhoneSnapshot)
    case phoneReachabilityChanged(PhoneReachability)
    case phoneCommandFailed(String)

    case bluetoothRouteAvailable
    case bluetoothRouteUnavailable
    case standalonePlaybackStarted
    case standalonePlaybackPaused
    case standalonePlaybackFailed(String)
}
Effects:

swift
enum WatchEffect: Equatable {
    case sendPhoneCommand(PhoneCommand)
    case requestPhoneSnapshot

    case configureAudioSession
    case requestAudioRoute
    case startStandalonePlayback(soundID: String, volumePercent: Int)
    case pauseStandalonePlayback
    case setStandaloneVolume(Int)

    case persistSettings(WatchSettings)
    case updateComplication(WatchSnapshot)
    case haptic(WatchHaptic)
    case showLocalNotification(title: String, body: String)
}
The key rule remains:

text
Views dispatch actions.
Reducer updates state and emits effects.
Services execute effects.
Views never call AVFoundation or WatchConnectivity directly.
7. WatchConnectivity Architecture
Create a connectivity service on both iPhone and Watch.

On iPhone
swift
final class PhoneWatchConnectivityService: NSObject, WCSessionDelegate {
    func publishSnapshot(_ snapshot: CascadeSnapshot) {
        let payload = SnapshotCodec.encode(snapshot)

        try? WCSession.default.updateApplicationContext([
            "snapshot": payload
        ])
    }

    func handleWatchCommand(_ command: PhoneCommand) {
        cascadeStore.dispatch(command.toCascadeAction())
    }
}
On Watch
swift
final class WatchConnectivityService: NSObject, WCSessionDelegate {
    var onSnapshot: ((PhoneSnapshot) -> Void)?
    var onReachabilityChanged: ((PhoneReachability) -> Void)?

    func activate() {
        guard WCSession.isSupported() else { return }
        WCSession.default.delegate = self
        WCSession.default.activate()
    }

    func send(_ command: PhoneCommand) {
        if WCSession.default.isReachable {
            WCSession.default.sendMessage(
                ["command": command.encoded()],
                replyHandler: nil,
                errorHandler: { error in
                    // dispatch .phoneCommandFailed(...)
                }
            )
        } else {
            // For play/pause, don't queue stale commands.
            // Instead update reachability/error state.
        }
    }
}
Use:

sendMessage for immediate foreground play/pause commands.

updateApplicationContext for latest snapshot/settings.

transferUserInfo for non-urgent queued changes.

transferFile only if you later support custom user audio assets; Apple’s API transfers local files asynchronously on a background thread.

Do not queue play/pause commands when the phone is unreachable. A stale “pause” command arriving ten minutes later is worse than dropping it.

8. Standalone Watch Audio
Capability
Enable Background Modes → Audio on the Watch target. Apple’s watchOS audio-streaming guidance says this is the WatchKit extension capability required for background audio playback.
 Older tutorials show this as adding UIBackgroundModes with audio to the Watch app plist, which is the same underlying capability.

Bluetooth route requirement
Before starting standalone playback, check/request an output route. Apple’s background-audio documentation specifically notes that watchOS requires a Bluetooth audio route for long-form audio.

Architecturally, that means this is a valid Watch state:

text
Mode: Watch
Status: Connect AirPods or Bluetooth audio to play Cascade on Apple Watch.
[Choose Audio Output]
Playback service
Use AVQueuePlayer + AVPlayerLooper for the standalone player. Apple’s AVPlayerLooper is designed to loop a single AVPlayerItem through an AVQueuePlayer.
 AVPlayer has been available on watchOS since watchOS 6, while the old WKAudioFilePlayer family was deprecated, making AVFoundation the right modern playback layer.

swift
import AVFoundation

@MainActor
final class WatchPlaybackService {
    private var queuePlayer: AVQueuePlayer?
    private var looper: AVPlayerLooper?

    func startLoop(soundID: String, volumePercent: Int) throws {
        guard let url = Bundle.main.url(
            forResource: soundID,
            withExtension: "m4a"
        ) else {
            throw WatchPlaybackError.assetMissing
        }

        let item = AVPlayerItem(url: url)
        let player = AVQueuePlayer(playerItem: item)

        self.looper = AVPlayerLooper(
            player: player,
            templateItem: item
        )

        player.volume = Float(volumePercent) / 100.0

        self.queuePlayer = player
        player.play()
    }

    func pause() {
        queuePlayer?.pause()
    }

    func setVolume(_ percent: Int) {
        queuePlayer?.volume = Float(percent) / 100.0
    }

    func stop() {
        queuePlayer?.pause()
        queuePlayer?.removeAllItems()
        queuePlayer = nil
        looper = nil
    }
}

enum WatchPlaybackError: Error {
    case assetMissing
}
Keep strong references to both AVQueuePlayer and AVPlayerLooper; otherwise looping may stop unexpectedly.

9. Audio Session Service
swift
import AVFoundation

@MainActor
final class WatchAudioSessionService {
    func prepareForPlayback() async throws {
        let session = AVAudioSession.sharedInstance()

        try session.setCategory(
            .playback,
            mode: .default,
            options: []
        )

        // On watchOS, call route-selection preparation before activation
        // if you need the user to choose AirPods / Bluetooth output.
        await prepareRouteSelectionIfAvailable(session)

        try session.setActive(true)
    }

    private func prepareRouteSelectionIfAvailable(
        _ session: AVAudioSession
    ) async {
        await withCheckedContinuation { continuation in
            session.prepareRouteSelectionForPlayback { _, _ in
                continuation.resume()
            }
        }
    }

    func deactivate() {
        try? AVAudioSession.sharedInstance().setActive(false)
    }
}
Do not activate the Watch audio session at app launch. Apple’s watchOS audio-streaming guidance recommends activating the audio session only once the app has enough information/content to begin playback, because activating audio can disrupt the user experience.

10. Watch UI
Root view
swift
struct WatchMainView: View {
    @Environment(WatchStore.self) private var store

    var body: some View {
        VStack(spacing: 10) {
            Text("Cascade")
                .font(.headline)

            Text(store.snapshot.activeDeviceLabel)
                .font(.caption2)
                .foregroundStyle(.secondary)

            Button {
                store.dispatch(.togglePlaybackTapped)
            } label: {
                Image(systemName: store.snapshot.isPlaying
                      ? "pause.fill"
                      : "play.fill")
                    .font(.system(size: 34, weight: .semibold))
            }
            .buttonStyle(.borderedProminent)
            .clipShape(Circle())

            WatchTimerPresetBar(
                selected: store.snapshot.selectedTimer
            ) { preset in
                store.dispatch(.timerSelected(preset))
            }

            if let remaining = store.snapshot.remainingText {
                Text(remaining)
                    .font(.system(.title3, design: .monospaced))
                    .monospacedDigit()
            }
        }
        .padding()
    }
}
Route unavailable view
swift
struct RouteUnavailableView: View {
    var body: some View {
        VStack(spacing: 8) {
            Image(systemName: "airpods")
                .font(.title)

            Text("Connect AirPods")
                .font(.headline)

            Text("Apple Watch needs Bluetooth audio for long Cascade playback.")
                .font(.caption2)
                .multilineTextAlignment(.center)
        }
    }
}
The Watch should never silently fail to play. If standalone playback is selected and no valid route exists, show the route state directly.

11. Mode Selection UX
Use a simple mode picker:

swift
enum WatchCascadeMode {
    case phoneRemote
    case watchStandalone
}
UI:

text
Play on
[ iPhone ] [ Watch ]
Behavior:

Situation	Recommended behavior
iPhone reachable	Default to iPhone Remote
iPhone playing Cascade	Mirror phone state immediately
iPhone unreachable + Bluetooth route available	Offer Watch Standalone
iPhone unreachable + no Bluetooth route	Show connect-audio state
User explicitly chooses Watch	Keep that preference locally
User explicitly chooses iPhone	Send commands to phone only
This prevents user confusion: “Play” always means “play on the selected device.”

12. Settings Persistence
Because iPhone and Watch defaults are not shared, the Watch must persist its own local settings and sync intentionally.

Recommended Watch settings:

swift
struct WatchSettings: Codable, Equatable {
    var preferredMode: WatchCascadeMode = .phoneRemote
    var defaultTimer: TimerPreset = .infinite
    var standaloneVolumePercent: Int = 60
    var hapticsEnabled: Bool = true
}
For tiny Watch-only preferences, UserDefaults / @AppStorage is acceptable; UserDefaults is designed for app-specific persistent settings.
 If you want exact parity with the rest of Cascade, store a local watch_settings.json in Application Support and sync the phone’s normalized settings JSON through WatchConnectivity.

13. Complications / Smart Stack
Add a WidgetKit complication extension for quick access and status.

Apple’s current guidance is to create watchOS complications using WidgetKit / SwiftUI views rather than the older ClockKit path.

Recommended complication states:

text
Idle:
  Cascade

Playing:
  Cascade · 42m

Paused timer:
  Paused · 42m

Route needed:
  AirPods
Do not attempt second-by-second complication updates. Complications are timeline-driven and not guaranteed to refresh in real time; design them for coarse status and quick launch, not precise countdown rendering.

Complication bridge:

swift
final class WatchComplicationBridge {
    func update(_ snapshot: WatchSnapshot) {
        // Persist compact complication state to App Group/UserDefaults.
        // Ask WidgetCenter to reload timelines when meaningful state changes.
    }
}
Update complications only on meaningful changes:

play/pause

timer selected

session ended

mode changed

route unavailable

14. Haptics
Haptics are a Watch-specific strength. Add light haptics for:

Event	Haptic
Play	click/start
Pause	click/stop
Timer started	success
Session complete	notification
Route unavailable	failure
Keep haptics behind an effect:

swift
enum WatchEffect {
    case haptic(WatchHaptic)
}
This keeps views declarative and avoids sprinkling WatchKit calls across the UI.

15. Notifications
For Companion Remote Mode, the iPhone can own session-complete notifications and they may naturally appear on Watch depending on notification settings.

For Standalone Watch Mode, schedule a local Watch notification when a timer starts:

swift
case .timerSelected(.minutes60):
    effects.append(.showLocalNotification(
        title: "Cascade",
        body: "Your 60 minute session is complete."
    ))
But if audio is actively playing in background mode, the app should also stop playback itself when the timer expires. The notification is user feedback, not the source of truth.

16. Extended Runtime Sessions
Do not use extended runtime sessions as the primary way to keep Cascade alive for 30/60/8h audio. watchOS has extended runtime sessions for specific cases like communicating with devices, processing data, sounds, or haptics after the screen turns off, but long-form audio should be handled as a proper background-audio app instead.

Use extended runtime only for non-audio short flows if you later add something like:

breathing haptic guidance

silent timer-only focus mode

short “self-care” session UI

For the waterfall audio loop, background audio is the correct model.

17. Build Phases
Phase W1 — Companion Remote MVP
Deliverable:

text
Watch app shows iPhone Cascade state.
Play/pause from Watch controls iPhone.
Timer presets send commands to iPhone.
No Watch audio yet.
Implement:

SwiftUI Watch app

WatchConnectivity on iPhone + Watch

Snapshot sync

Immediate commands via sendMessage

Haptics

Minimal complication that launches the app

This is the fastest useful Watch version.

Phase W2 — Standalone Watch Playback
Deliverable:

text
Watch can play Cascade locally through AirPods/Bluetooth.
Implement:

waterfall_watch.m4a

Background audio capability

WatchAudioSessionService

WatchPlaybackService

Bluetooth route unavailable state

Local timer/tick handling

Phase W3 — Complications + Smart Stack
Deliverable:

text
Complication shows Cascade status and remaining time roughly.
Implement:

WidgetKit complication

App Group / shared compact state

meaningful timeline reloads

Phase W4 — Settings Sync
Deliverable:

text
Phone and Watch agree on default timer, volume, and preferred mode.
Implement:

settings JSON sync via WatchConnectivity

local Watch override for standalone volume

conflict rules: phone owns global settings, Watch owns Watch-only settings

Phase W5 — Optional Rust Core Spike
Deliverable:

text
Blank Watch app calls cascade-core dispatch through UniFFI.
Only do this if you decide Cascade should become a true full-platform Rust-core rehearsal project.

18. Testing Matrix
Test on a real Apple Watch, not just the simulator.

Companion Remote Mode
iPhone foreground, Watch foreground.

iPhone locked, Watch controls play/pause.

iPhone app killed, Watch command behavior is sane.

Watch out of range.

Watch reconnects and receives latest snapshot.

Stale play/pause commands are not queued.

Standalone Mode
AirPods connected: play, lock/lower wrist, verify audio continues.

No Bluetooth route: app shows route-needed state.

AirPods disconnect during playback: pause and show error.

30-minute timer ends while wrist down.

8-hour session battery behavior.

Low Power Mode.

Watch charging vs not charging.

App relaunched while previous session state exists.

Complications
Idle complication.

Playing complication.

Timer remaining updates at coarse intervals.

Tap complication opens app.

Complication does not pretend to be real-time.

19. Non-Goals
For watchOS v1, explicitly avoid:

text
No WebView.
No React Native / shared web UI.
No streaming from cascade.stephens.page.
No Rust core unless a dedicated spike proves it is worth it.
No second-by-second complication countdown.
No trying to play long-form audio through the Watch speaker.
No queuing stale play/pause commands for the phone.
No direct AVFoundation calls from SwiftUI views.
Final Architecture
text
CascadeWatch
  SwiftUI watchOS app

  WatchStore
    snapshot
    dispatch(action)

  Reducer
    pure watch state transitions
    emits WatchEffects

  Services
    WatchConnectivityService
      remote-controls iPhone Cascade
      receives phone snapshots/settings

    WatchAudioSessionService
      configures AVAudioSession
      requests/validates Bluetooth route

    WatchPlaybackService
      AVQueuePlayer + AVPlayerLooper
      loops waterfall_watch.m4a locally

    WatchSettingsStore
      local Watch settings cache

    WatchComplicationBridge
      writes compact state for WidgetKit complications

    WatchHapticsService
      executes haptic effects

  WidgetKit Complication
    coarse status + quick launch
The most important design choice is dual mode:

text
iPhone Remote Mode = default, battery-friendly, fastest to ship.
Watch Standalone Mode = premium/native, requires Bluetooth audio route.
That gives Cascade a watchOS version that feels genuinely Apple Watch-native without overcomplicating the first release.

# Claude Opus 4.7 Thinking

Cascade — watchOS Architecture
A watchOS Cascade should be a tiny, focused companion: glanceable session control on the wrist, with audio playing from the most appropriate device (paired iPhone, AirPods, or the Watch itself). Below is a recommended architecture grounded in Apple’s watchOS audio, connectivity, and complications APIs.

Note: I couldn't fetch the GitHub repo or the deployed web app in this environment, and the APK isn't directly inspectable here. This architecture builds on the iOS architecture from the previous turn and your existing Cascade reducer/state-machine/effects model.

1. Strategic Question: What Is Watch-Cascade For?
Before architecture, pick a product mode. The architecture differs sharply.

Mode	Description	Audio output	Complexity
A. Remote Control (recommended v1)	Watch is a remote to the iPhone Cascade app. Audio plays from iPhone/AirPods.	iPhone	Low
B. Standalone Watch Audio	Audio plays directly from Apple Watch speaker or AirPods paired to the Watch. Works even if iPhone is absent.	Watch / AirPods (via Watch)	High
C. Hybrid	Remote-control by default; standalone fallback when iPhone unreachable.	Either	Highest
Recommendation: ship Mode A first. It’s small, useful, low-risk, and matches Cascade’s actual use case (you’re likely already wearing AirPods or near your phone while working/praying). Mode B is a nice future addition; Mode C is over-engineered for v1.

2. Platform Decisions
Concern	Choice	Rationale
Min watchOS	watchOS 10+	Required for the redesigned NavigationSplitView-style layout, Smart Stack widgets, and modern complications via WidgetKit.
UI framework	SwiftUI	Only sane choice on modern watchOS; matches Cascade iOS.
Pattern	@Observable ViewModel with snapshot-driven UI	Same shape as iOS Cascade; trivially small on the Watch.
iPhone communication	WatchConnectivity (WCSession)	Apple's first-party, low-overhead bidirectional channel between paired iPhone and Watch apps.
Core access (Mode A)	None on Watch. Watch is a thin remote; Rust core stays on iPhone.	Avoids cross-compiling Rust to arm64-apple-watchos and shipping a second XCFramework.
Core access (Mode B/C)	UniFFI XCFramework with arm64-apple-watchos + sim slices	Necessary if Watch must independently run sessions/timers/settings.
Audio (Mode A)	iPhone owns audio. Watch sends Play/Pause/SetVolume actions over WCSession.	Simple; reuses existing iOS audio engine.
Audio (Mode B)	AVAudioSession + AVAudioPlayer/AVAudioEngine with WKAudioSessionCategory.playback and the watchOS Background Audio mode	Standalone audio on Apple Watch is supported but tightly constrained — must use proper background audio configuration.
Now Playing (Mode B)	MPNowPlayingInfoCenter + MPRemoteCommandCenter as on iOS	Surfaces play/pause on the Watch's Now Playing screen.
Complications	WidgetKit-based complications (watchOS 10+)	Glanceable watch-face presence: "Cascade running, 42:17 left."
Smart Stack widget	WidgetKit timeline showing remaining session	Auto-surfaces during active sessions.
Haptics	WKInterfaceDevice.current().play(_:)	Subtle wrist tap on session start/end.
Persistence	Tiny — UserDefaults is fine on Watch for last-known snapshot	No need for a JSON settings file on the Watch; iPhone owns settings.
Bundle ID	page.stephens.cascade.watchkitapp	Standard companion-app naming.
Distribution	Bundled with the iOS app via the Watch target	Standard watchOS distribution path.
3. Repo Layout — Watch Slice
Adding to the existing cascade-apple/ tree from the iOS architecture:

text
apps/cascade-apple/
├── CascadeShared/              # SwiftPM package (existing)
│   └── Sources/CascadeShared/
│       ├── Protocol/
│       │   └── WatchMessage.swift     # NEW: shared Codable types for WCSession
│       └── ...
├── CascadeiOS/                 # existing iOS app
│   └── Connectivity/
│       └── PhoneConnectivityService.swift   # NEW: WCSession on iPhone side
├── CascadeMac/                 # existing
└── CascadeWatch/               # NEW
    ├── CascadeWatchApp.swift
    ├── Info.plist                  # WKBackgroundModes if Mode B/C
    ├── Cascade.entitlements
    ├── ContentView.swift
    ├── Views/
    │   ├── WatchMainView.swift
    │   ├── WatchTimerPicker.swift
    │   └── WatchVolumeCrown.swift
    ├── ViewModels/
    │   └── WatchAppViewModel.swift     # @Observable; thin
    ├── Services/
    │   ├── WatchConnectivityService.swift
    │   └── WatchHaptics.swift
    ├── Complications/
    │   ├── CascadeComplication.swift   # WidgetKit
    │   └── CascadeWidgetBundle.swift
    └── Resources/
        └── Assets.xcassets/
            ├── AppIcon.appiconset
            └── ComplicationGlyph.imageset
4. The iPhone ↔ Watch Protocol (Mode A)
Define a small, versioned, Codable message protocol in CascadeShared so both targets compile against the same types.

swift
// CascadeShared/Sources/CascadeShared/Protocol/WatchMessage.swift

public enum WatchToPhoneCommand: Codable, Sendable {
    case requestSnapshot
    case togglePlayback
    case play
    case pause
    case setVolume(percent: UInt8)
    case startSession(WatchSessionPreset)
    case cancelSession
}

public enum WatchSessionPreset: String, Codable, Sendable {
    case minutes30, minutes60, hours8, infinite
}

public struct PhoneSnapshotForWatch: Codable, Sendable, Equatable {
    public var isPlaying: Bool
    public var volumePercent: UInt8
    public var session: WatchSessionState?
    public var statusLine: String     // pre-formatted by iPhone, e.g. "42:17 left"
}

public struct WatchSessionState: Codable, Sendable, Equatable {
    public var preset: WatchSessionPreset
    public var startedAt: Date
    public var endsAt: Date
    public var progressPercent: Double  // 0..1
}
The Watch sends commands; the iPhone sends snapshots. The Watch never reasons about timer math or settings — it just renders what it receives. This mirrors the same "one source of truth, snapshot-driven UI" rule used on every other Cascade client.

iPhone side (additions to existing iOS app)
swift
// CascadeiOS/Connectivity/PhoneConnectivityService.swift
import WatchConnectivity

final class PhoneConnectivityService: NSObject, WCSessionDelegate {
    static let shared = PhoneConnectivityService()
    private let session = WCSession.default

    func activate(dispatch: @escaping (AppCommand) async -> Void,
                  snapshotProvider: @escaping () -> PhoneSnapshotForWatch) {
        guard WCSession.isSupported() else { return }
        self.dispatch = dispatch
        self.snapshotProvider = snapshotProvider
        session.delegate = self
        session.activate()
    }

    /// Called from EffectExecutor whenever AppSnapshot changes.
    func pushSnapshot(_ s: PhoneSnapshotForWatch) {
        guard session.isReachable else {
            // Use applicationContext for "last known good"
            try? session.updateApplicationContext(["snapshot": try! JSONEncoder().encode(s)])
            return
        }
        let data = try! JSONEncoder().encode(s)
        session.sendMessageData(data, replyHandler: nil)
    }

    func session(_ session: WCSession,
                 didReceiveMessageData messageData: Data,
                 replyHandler: @escaping (Data) -> Void) {
        guard let cmd = try? JSONDecoder().decode(WatchToPhoneCommand.self, from: messageData) else {
            return
        }
        Task { @MainActor in
            await self.handle(cmd)
            let snap = self.snapshotProvider()
            replyHandler((try? JSONEncoder().encode(snap)) ?? Data())
        }
    }

    @MainActor
    private func handle(_ cmd: WatchToPhoneCommand) async {
        switch cmd {
        case .requestSnapshot:        break
        case .togglePlayback:         await dispatch?(.togglePlayback)
        case .play:                   await dispatch?(.play)
        case .pause:                  await dispatch?(.pause)
        case .setVolume(let v):       await dispatch?(.setVolume(percent: v))
        case .startSession(let p):    await dispatch?(.startPomodoro(p.toCore()))
        case .cancelSession:          await dispatch?(.cancelTimer)
        }
    }

    private var dispatch: ((AppCommand) async -> Void)?
    private var snapshotProvider: (() -> PhoneSnapshotForWatch)?

    // Required WCSessionDelegate stubs
    func session(_ s: WCSession, activationDidCompleteWith state: WCSessionActivationState, error: Error?) {}
    func sessionDidBecomeInactive(_ session: WCSession) {}
    func sessionDidDeactivate(_ session: WCSession) { session.activate() }
}
Wire iPhone EffectExecutor to add a new effect arm:

swift
case .updateWatchSnapshot(let snap):
    PhoneConnectivityService.shared.pushSnapshot(snap)
Or: piggyback on existing snapshot updates by calling pushSnapshot from AppViewModel.dispatch after each reducer step. Either works; the effect-arm version is more consistent with your architecture.

Watch side
swift
// CascadeWatch/Services/WatchConnectivityService.swift
import WatchConnectivity

@Observable
final class WatchConnectivityService: NSObject, WCSessionDelegate {
    static let shared = WatchConnectivityService()

    private(set) var snapshot: PhoneSnapshotForWatch = .placeholder
    private(set) var isReachable: Bool = false

    private let session = WCSession.default

    func activate() {
        guard WCSession.isSupported() else { return }
        session.delegate = self
        session.activate()
    }

    func send(_ cmd: WatchToPhoneCommand) {
        guard let data = try? JSONEncoder().encode(cmd) else { return }
        if session.isReachable {
            session.sendMessageData(data, replyHandler: { [weak self] reply in
                if let snap = try? JSONDecoder().decode(PhoneSnapshotForWatch.self, from: reply) {
                    Task { @MainActor in self?.snapshot = snap }
                }
            }, errorHandler: nil)
        } else {
            // queue for next reachability via transferUserInfo
            session.transferUserInfo(["queuedCommand": data])
        }
    }

    // Receive proactive snapshot pushes from iPhone
    func session(_ s: WCSession, didReceiveMessageData messageData: Data) {
        if let snap = try? JSONDecoder().decode(PhoneSnapshotForWatch.self, from: messageData) {
            Task { @MainActor in self.snapshot = snap }
        }
    }

    func session(_ s: WCSession, didReceiveApplicationContext applicationContext: [String : Any]) {
        if let data = applicationContext["snapshot"] as? Data,
           let snap = try? JSONDecoder().decode(PhoneSnapshotForWatch.self, from: data) {
            Task { @MainActor in self.snapshot = snap }
        }
    }

    func session(_ s: WCSession, activationDidCompleteWith state: WCSessionActivationState, error: Error?) {
        Task { @MainActor in self.isReachable = s.isReachable }
        send(.requestSnapshot)
    }
    func sessionReachabilityDidChange(_ s: WCSession) {
        Task { @MainActor in self.isReachable = s.isReachable }
    }
}
Use sendMessageData for live foreground commands, updateApplicationContext for last-known-good state when the apps aren’t reachable, and transferUserInfo as a guaranteed-delivery queue for commands the user issued while the iPhone app was suspended.

5. Watch UI
Three screens, all tiny. The watch face is not for elaborate UI.

MainView
swift
struct WatchMainView: View {
    @Environment(WatchConnectivityService.self) private var conn
    @State private var crownVolume: Double = 70

    var body: some View {
        VStack(spacing: 8) {
            Text(conn.snapshot.statusLine)         // e.g. "42:17 left" or "Paused"
                .font(.headline)
                .monospacedDigit()
                .lineLimit(1)
                .minimumScaleFactor(0.6)

            Button {
                WKInterfaceDevice.current().play(.click)
                conn.send(.togglePlayback)
            } label: {
                Image(systemName: conn.snapshot.isPlaying ? "pause.fill" : "play.fill")
                    .font(.system(size: 36, weight: .bold))
                    .frame(maxWidth: .infinity, minHeight: 64)
            }
            .buttonStyle(.borderedProminent)
            .tint(.blue)

            HStack(spacing: 4) {
                NavigationLink("Timer") { WatchTimerPicker() }
                    .buttonStyle(.bordered)
                NavigationLink("Vol")   { WatchVolumeCrown(volume: $crownVolume) }
                    .buttonStyle(.bordered)
            }
            .font(.caption)
        }
        .padding(.horizontal, 4)
        .focusable()
        .digitalCrownRotation(
            $crownVolume,
            from: 0, through: 100, by: 1,
            sensitivity: .medium,
            isContinuous: false,
            isHapticFeedbackEnabled: true
        )
        .onChange(of: crownVolume) { _, new in
            conn.send(.setVolume(percent: UInt8(new)))
        }
        .onAppear {
            crownVolume = Double(conn.snapshot.volumePercent)
        }
    }
}
Key watchOS idioms in use here:

Digital Crown for volume — the Watch's defining input affordance.

Haptic on tap via WKInterfaceDevice.

minimumScaleFactor for the small status line — text must fit on a 41mm/45mm screen.

System materials and SF Symbols — no custom drawing.

WatchTimerPicker
swift
struct WatchTimerPicker: View {
    @Environment(WatchConnectivityService.self) private var conn
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        List {
            preset("∞ Infinite",   .infinite)
            preset("30 min",       .minutes30)
            preset("60 min",       .minutes60)
            preset("8 hours",      .hours8)
            if conn.snapshot.session != nil {
                Button("Cancel session", role: .destructive) {
                    conn.send(.cancelSession); dismiss()
                }
            }
        }
        .navigationTitle("Session")
    }

    private func preset(_ label: String, _ p: WatchSessionPreset) -> some View {
        Button(label) {
            conn.send(.startSession(p))
            WKInterfaceDevice.current().play(.start)
            dismiss()
        }
    }
}
6. Complications & Smart Stack
This is where watchOS earns its keep for Cascade. Use WidgetKit complications (watchOS 10+ unified the old ClockKit complication families with Widgets).

swift
// CascadeWatch/Complications/CascadeComplication.swift
import WidgetKit
import SwiftUI

struct CascadeComplicationProvider: TimelineProvider {
    func placeholder(in context: Context) -> CascadeEntry {
        CascadeEntry(date: .now, isPlaying: false, statusLine: "Cascade")
    }

    func getSnapshot(in context: Context, completion: @escaping (CascadeEntry) -> Void) {
        completion(load())
    }

    func getTimeline(in context: Context,
                     completion: @escaping (Timeline<CascadeEntry>) -> Void) {
        let entry = load()
        // If there's an active session, schedule a refresh at end-of-session
        let next = entry.endsAt ?? Date().addingTimeInterval(15 * 60)
        completion(Timeline(entries: [entry], policy: .after(next)))
    }

    private func load() -> CascadeEntry {
        // Read last-known snapshot from App Group UserDefaults shared with the Watch app
        let d = UserDefaults(suiteName: "group.page.stephens.cascade")!
        // ...decode and return
        return CascadeEntry(date: .now, isPlaying: false, statusLine: "Cascade")
    }
}

struct CascadeComplicationView: View {
    let entry: CascadeEntry

    var body: some View {
        ZStack {
            Image(systemName: entry.isPlaying ? "water.waves" : "water.waves.slash")
        }
        .containerBackground(.blue.gradient, for: .widget)
        .widgetLabel(entry.statusLine)         // shown in corner/circular complications
    }
}

@main
struct CascadeWidgetBundle: WidgetBundle {
    var body: some Widget {
        CascadeComplication()
    }
}

struct CascadeComplication: Widget {
    var body: some WidgetConfiguration {
        StaticConfiguration(kind: "CascadeComplication",
                            provider: CascadeComplicationProvider()) { entry in
            CascadeComplicationView(entry: entry)
        }
        .supportedFamilies([
            .accessoryCircular,
            .accessoryCorner,
            .accessoryRectangular,
            .accessoryInline
        ])
        .configurationDisplayName("Cascade")
        .description("Glance at your active Cascade session.")
    }
}
Use an App Group (group.page.stephens.cascade) so the Watch app and the Widget extension can share UserDefaults containing the most recent snapshot. The Watch app writes whenever it receives a snapshot from iPhone; the Widget reads on timeline refresh and asks the system to reload via WidgetCenter.shared.reloadAllTimelines() after meaningful changes.

For active sessions, also surface a Smart Stack widget with accessoryRectangular family showing remaining time and a play/pause control via App Intents (so a single tap on the Smart Stack pauses without launching the app).

7. Lifecycle, Reachability, and "What If My Phone Is Dead?"
Watch apps are aggressively suspended. Treat the Watch as stateless except for the last snapshot:

On onAppear, call WatchConnectivityService.shared.send(.requestSnapshot).

Cache the most recent snapshot in App Group UserDefaults with a receivedAt: Date.

If the snapshot is older than ~2 minutes and the iPhone is unreachable, render a "Disconnected" state with a Retry button.

Never simulate timer countdown locally in Mode A — request a fresh snapshot when the user opens the app or rotates the wrist.

If you eventually want Mode B (standalone audio):

Cross-compile cascade-core for arm64-apple-watchos and add it to the XCFramework.

Add <key>WKBackgroundModes</key><array><string>audio</string></array> to the Watch Info.plist.

Use AVAudioSession.sharedInstance().setCategory(.playback) exactly like iOS.

Bundle the cascade.mp3 (or a smaller compressed version — Watch storage matters) inside the Watch app.

Add MPNowPlayingInfoCenter integration so the Watch's Now Playing screen shows Cascade.

Be prepared for sharply different battery characteristics — long sessions on Watch alone will hammer battery.

8. Settings on the Watch
Don't store user-editable settings on the Watch. The iPhone owns settings.json; the Watch app simply reflects current state from the latest snapshot. If someone changes the default volume on the Watch via the Crown, that flows to iPhone via setVolume → reducer → updated settings.json → snapshot pushed back. One source of truth, one persistence location.

The only Watch-local persistence is:

Last-known snapshot in App Group UserDefaults (for instant cold start and complications)

Selected Watch theme/accent color, if you offer one (Watch-only UX preference)

9. Implementation Phases
Phase	Deliverable	Why
w1	Watch app target with WCSession-based remote control of iPhone Cascade. Play/Pause + status text only.	Validates the connectivity protocol and "thin remote" model end-to-end.
w2	Volume via Digital Crown + session presets (30/60/8h/Infinite + Cancel).	Functional parity with iPhone session controls.
w3	App Group + WidgetKit complications (circular, corner, rectangular).	Glanceability — the watchOS superpower.
w4	Smart Stack rectangular widget with App Intents for one-tap play/pause without launching the app.	Reduces friction to near-zero.
w5	Haptics polish (start/end of session) + better disconnected-state UX.	Matches Apple Watch interaction expectations.
w6 (optional)	Mode B standalone audio: Rust core for arm64-apple-watchos, WKBackgroundModes: audio, AirPods-paired-to-Watch playback.	Only if you actually need iPhone-free use.
10. Non-Goals
No independent timer state on the Watch in Mode A (don't run a parallel countdown — display only).

No writes to settings.json from the Watch.

No custom audio engine on the Watch in v1 — the iPhone is the audio source of truth.

No complicated navigation hierarchy — two clicks max to any control.

No WebView, no remote rendering.

No Rust cross-compile to arm64-apple-watchos until Mode B is actually needed.

Final Recommendation
Ship Cascade for watchOS as:

text
apps/cascade-apple/CascadeWatch
  SwiftUI watchOS 10+ app
  @Observable WatchConnectivityService over WCSession
  Digital Crown for volume; tap for play/pause
  WidgetKit complications + Smart Stack widget with App Intents
  Shared Codable WatchToPhoneCommand / PhoneSnapshotForWatch in CascadeShared
  No Rust core on watchOS (v1)
  No local audio engine (v1) — iPhone owns playback
The architectural rule, identical to iOS and Windows: the Rust core (via iPhone) owns Cascade's truth; watchOS owns Cascade's wrist-ness. The Watch is a glanceable remote that converts taps and crown rotations into commands and renders a snapshot back. Everything that is Cascade — timers, sessions, settings, audio — lives where it already lives. The Watch just makes it one tap closer.

If you can paste your current iOS AppCommand / AppSnapshot types and any Watch design preferences (single-screen vs paged, complication families you actually use on your watch face), I can tailor the WatchMessage protocol and complication families exactly.

# Gemini 3.1 Pro Thinking

Cascade — watchOS Architecture (SwiftUI + Shared Core)
The watchOS version of Cascade should be a standalone native SwiftUI app. Because Cascade’s primary feature is a long-running audio loop with timers, it is perfectly suited for the Apple Watch.

You can reuse the exact same headless core (whether that is your cascade-core Rust crate compiled for aarch64-apple-watchos or your CascadeShared Swift state machine) that you use for iOS. The architecture rule remains: The core owns Cascade's truth; watchOS owns Cascade's watchOS-ness.

Here is the architecture for Cascade on watchOS.

1. Platform Decisions & watchOS Nuances
watchOS is the most aggressively resource-constrained Apple platform. Apps are suspended almost immediately when the wrist drops unless they are running an active background session.

Concern	Choice	Rationale
UI Framework	SwiftUI	Mandatory for modern watchOS apps.
Min OS	watchOS 10	Unlocks the new SwiftUI NowPlayingView, vertical pagination design language, and modern @Observable state.
Core Access	Shared Core (CascadeShared or Rust XCFramework)	Use the exact same dispatch(action) -> {snapshot, effects} pattern as iOS.
Audio Engine	AVQueuePlayer + AVPlayerLooper	Safest API for background gapless looping on watchOS.
Audio Routing	AVAudioSession.RouteSharingPolicy.longFormAudio	Critical watchOS detail: This specific policy is required by Apple to permit background audio playback on the Watch.
Speaker Playback	Handled by watchOS	Watch Series 10 & Ultra 2 can play media via internal speakers. Older watches will automatically prompt the user to connect Bluetooth headphones.
Hardware Controls	Digital Crown	Controls system volume automatically when an audio session is active.
Lock Screen/Face	MPNowPlayingInfoCenter	Integrates with the "Now Playing" icon at the top of the watch face.
Haptics	WKInterfaceDevice.current().play(...)	watchOS-specific haptic engine for timer completion (e.g., .success or .stop).
Timers	UNUserNotificationCenter	Emits a local notification if the timer finishes while the audio is paused/app is suspended.
2. Repo Layout — watchOS Slice
You can house the watchOS app in the same Xcode workspace as iOS, sharing the CascadeShared package.

text
apps/
└── cascade-apple/
    ├── CascadeShared/                    # Shared core, VMs, state, settings
    ├── CascadeiOS/
    ├── CascadeMac/
    └── CascadeWatch/                     # ← NEW
        ├── CascadeWatchApp.swift
        ├── Info.plist                    # UIBackgroundModes: audio
        ├── Views/
        │   ├── WatchRootView.swift
        │   ├── WatchPlayerView.swift
        │   └── WatchTimerView.swift
        └── Services/                     # watchOS-specific Effect Executors
            ├── WatchAudioSession.swift   # Uses .longFormAudio
            ├── WatchHaptics.swift        # WKInterfaceDevice
            └── WatchEffectExecutor.swift # Wires effects to Watch services
3. The watchOS Audio Session (The Critical Path)
To keep the waterfall running when the user lowers their wrist, you must use the .longFormAudio routing policy. If you use standard .playback without it, watchOS will likely suspend the app.

swift
import AVFoundation

final class WatchAudioSessionService {
    func configure() throws {
        let session = AVAudioSession.sharedInstance()
        
        // .longFormAudio tells watchOS this is a primary background audio app 
        // (like a podcast or white noise app) and keeps the process alive.
        try session.setCategory(
            .playback,
            mode: .default,
            policy: .longFormAudio, 
            options: []
        )
        
        try session.setActive(true)
        
        // Observe route changes (e.g., AirPods disconnected)
        NotificationCenter.default.addObserver(
            forName: AVAudioSession.routeChangeNotification,
            object: session,
            queue: .main
        ) { note in
            // Dispatch to core to handle pause logic
        }
    }
}
Note on Speakers: When you call player.play(), watchOS automatically handles the routing. If the user has an older watch, the OS will pop up a native "Choose Bluetooth Device" screen. If they have a Series 10/Ultra 2, it will play out of the watch speaker natively. Your code does not need to manage this UI.

4. UI Architecture (watchOS 10 SwiftUI)
watchOS 10 introduced a new design language based on vertical pagination (TabView with .tabViewStyle(.verticalPage)).

Your UI should be split into distinct pages:

Player Page: Big play/pause button, time remaining.

Timer Page: Presets (30m, 60m, 8h).

Now Playing Page: Native system controls.

swift
import SwiftUI

struct WatchRootView: View {
    @Environment(AppViewModel.self) private var app
    
    var body: some View {
        TabView {
            // Page 1: Main Cascade Player
            WatchPlayerView()
                .containerBackground(.blue.gradient, for: .tabView)
            
            // Page 2: Timer Presets
            WatchTimerView()
                .containerBackground(.blue.gradient.opacity(0.5), for: .tabView)
            
            // Page 3: System Now Playing
            // watchOS 10 natively exposes the system media controls!
            NowPlayingView() 
        }
        .tabViewStyle(.verticalPage)
        .onAppear {
            Task { await app.dispatch(.appLaunched) }
        }
    }
}
The Player View
Keep it extremely glanceable. The Digital Crown handles volume, so you don't need a volume slider on the UI.

swift
struct WatchPlayerView: View {
    @Environment(AppViewModel.self) private var app
    
    var body: some View {
        VStack {
            if let remaining = app.snapshot.remainingTimeStr {
                Text(remaining)
                    .font(.system(.title2, design: .rounded, weight: .bold))
                    .foregroundStyle(.cyan)
            }
            
            Spacer()
            
            Button {
                Task { await app.dispatch(.togglePlayback) }
            } label: {
                Image(systemName: app.snapshot.isPlaying ? "pause.fill" : "play.fill")
                    .font(.system(size: 40))
            }
            .buttonStyle(.plain)
            .frame(width: 80, height: 80)
            .background(Circle().fill(.white.opacity(0.2)))
            
            Spacer()
            
            Text("Waterfall")
                .font(.footnote)
                .foregroundStyle(.secondary)
        }
    }
}
5. Background Timers & Execution
Because of aggressive watchOS backgrounding rules:

If audio is playing: The app remains alive in the background. The Tick(now_ms) dispatches normally. When the core determines the timer is done, it emits PausePlayback, Haptic, and UpdateNowPlaying.

If audio is paused: The app will be suspended.

To handle case #2, your StartTimer effect must always schedule a local notification.

swift
// In WatchEffectExecutor.swift
case let .scheduleTimerFinishedNotification(date):
    let content = UNMutableNotificationContent()
    content.title = "Cascade"
    content.body = "Focus session complete."
    content.sound = UNNotificationSound.default
    
    let trigger = UNCalendarNotificationTrigger(
        dateMatching: Calendar.current.dateComponents([.hour, .minute, .second], from: date),
        repeats: false
    )
    
    let request = UNNotificationRequest(identifier: "cascade.timer", content: content, trigger: trigger)
    try? await UNUserNotificationCenter.current().add(request)
6. Haptics (watchOS Specific)
Apple Watch relies heavily on physical feedback. When a session starts or completes, do not rely on audio chimes. Use WKInterfaceDevice.

swift
final class WatchHapticsService {
    func play(_ kind: HapticKind) {
        let device = WKInterfaceDevice.current()
        switch kind {
        case .sessionStarted:
            device.play(.start)
        case .sessionFinished:
            device.play(.stop)      // or .success
        case .buttonTap:
            device.play(.click)
        }
    }
}
Your WatchEffectExecutor handles the mapping: case let .haptic(kind): haptics.play(kind).

7. App Lifecycle Integration
Similar to iOS, you must register remote commands so the Now Playing screen (and the physical side button) can play/pause your app.

swift
final class WatchRemoteCommandService {
    func register(dispatch: @escaping (AppCommand) async -> Void) {
        let cc = MPRemoteCommandCenter.shared()
        
        cc.playCommand.addTarget { _ in Task { await dispatch(.play) }; return .success }
        cc.pauseCommand.addTarget { _ in Task { await dispatch(.pause) }; return .success }
        cc.togglePlayPauseCommand.addTarget { _ in Task { await dispatch(.togglePlayback) }; return .success }
    }
}
8. Implementation Phases
Phase	Deliverable	Why
w1	Standalone Target	Add watchOS target. Verify shared core/ViewModels compile.
w2	Audio Pipeline	Implement AVQueuePlayer + .longFormAudio session. Verify it plays in the background with screen off.
w3	UI & Media Controls	Implement WatchRootView with pagination. Connect MPNowPlayingInfoCenter and test SwiftUI NowPlayingView.
w4	Timers & Haptics	Wire up WKInterfaceDevice for taps and completion. Test UNUserNotificationCenter for suspended app timers.
w5	Smart Stack (Optional)	Build an AppIntent/Widget to quickly start a 30m waterfall session directly from the watch face Smart Stack.
Final Recommendation
Build apps/cascade-apple/CascadeWatch as a Standalone watchOS 10 app that shares your core logic via CascadeShared.

The secret to watchOS audio success is using AVAudioSession.RouteSharingPolicy.longFormAudio. Combine this with a simple paged SwiftUI UI, AVQueuePlayer for gapless playback, and WKInterfaceDevice for haptics. This provides a highly premium, computer-free focus tool that perfectly extends the Cascade ecosystem.