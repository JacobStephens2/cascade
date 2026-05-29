# Architecture Brief 

## Prompt

I want to make an Android app that plays this 13 minute mp3 of waterfalls on loop as white noise to cover. I want to use it as background noise to work. Right now I'm using the local files section of Spotify, but sometimes that doesn't load. I also want to have a web app version of the app. Maybe we could even have a headless core architecture, where there is a Rust core and then native versions of the app, such as a web native version, and an Android native version, but the Rust core keeps them all unified. I sort of want to do this as a way to test out this architecture I'm planning for the Clave app I'm working on. I attached the tech stack description doc for that game which goes into this.

Describe a good architecture for this simple app which lets me test out the design I'm planning for Clave.

## My Decisions

This should be an app I can use daily. I've logged thousands of hours listening to this mp3 file over the past several years, and I want clean ways to listen to it.

The name of this app is Cascade.

Add a pomodoro timer which can set the falls sound to play for either 30 minutes, 60 minutes, 8 hours, or a custom length of time. I'll use this for prayer and work sessions.

Implement (1) play/pause, (2) volume, (3) sleep timer via Tick events, and (4) persisted settings JSON

Follow this recommendation:

Build a tiny repo that mirrors Clave’s shape and boundary rules:

- waterfall-core (Rust, headless): state machine + settings + dispatch(action) -> {snapshot, effects}; no audio APIs.

- waterfall-web (React/Vite): wasm-bindgen + Web Audio loop + PWA caching of the audio file.

- waterfall-android (Compose): Media3 ExoPlayer hosted in a MediaSessionService for background playback + system controls.

## Perplexity Model Council Synthesis on Architecture

### 1. Where Models Agree

| Finding | GPT-5.5 Thinking | Claude Opus 4.7 Thinking | Gemini 3.1 Pro Thinking | Evidence |
| :-- | :-- | :-- | :-- | :-- |
| Use the Clave pattern: **headless Rust core + native shells** (web + Android) | ✓ | ✓ | ✓ | Your Clave stack’s “headless Rust core + native UIs” principle[^1_1] |
| Keep **audio playback out of Rust**; do it platform-native | ✓ | ✓ | ✓ | Clave doc: audio playback is a platform concern, not core-owned[^1_1] |
| Make the Rust API **coarse-grained** and drive UI via **snapshots** | ✓ | ✓ | ✓ | Clave doc: coarse-grained FFI + pass whole snapshots[^1_1] |
| Build **web-first** to pressure-test the API through WASM/TS | ✓ | ✓ | ✓ | Clave doc: web-first, wasm-bindgen is most demanding boundary[^1_1] |
| On Android, use **Media3 (ExoPlayer) + MediaSessionService** for reliable background playback | ✓ | ✓ |  | Media3 playback components overview; background playback with MediaSessionService |
| On web, prefer **Web Audio API** for robust looping rather than relying only on `<audio loop>` | ✓ | ✓ | ✓ | AudioBufferSourceNode looping semantics; practical Web Audio loop approach |


***

### 2. Where Models Disagree

| Topic | GPT-5.5 Thinking | Claude Opus 4.7 Thinking | Gemini 3.1 Pro Thinking | Why They Differ |
| :-- | :-- | :-- | :-- | :-- |
| Should the core compute “loop points / seamless looping” details? | Keep core about intent/state; platform implements looping | Core can model loop points + loop events as “visual hints” | Focuses on format choice (OGG/WAV) more than core loop math | Claude weights “test the timing boundary” more; GPT-5.5 prefers minimal shared logic; Gemini emphasizes codec realities for gapless playback. |
| Add extra features to better exercise architecture (e.g., sleep timer, fades) | Yes—sleep timer/fades/settings are good “architecture kata” | Yes—strictly as state-machine rehearsal | Suggests adding a focus/pomodoro timer to make core meaningful | All want “more than a play button,” but differ on *which* complexity best validates Clave. |
| Audio format: keep MP3 vs convert | Not central; can work with MP3 | Not central; can ship with MP3 | Strongly recommends OGG/WAV for true gapless loops | Gemini prioritizes gapless fidelity; others prioritize architecture validation regardless of tiny loop artifacts. |


***

### 3. Unique Discoveries

| Model | Unique Finding | Why It Matters |
| :-- | :-- | :-- |
| Gemini 3.1 Pro Thinking | Proposes adding a **focus/pomodoro timer** so the Rust core has “time/state” substance | Lets you rehearse Clave’s “core keeps time, UI supplies ticks” rule on something useful daily. |


***

### 4. Comprehensive Analysis

**High-Confidence Findings.** All three models converge on the same core idea: treat this waterfall player as a miniature “Clave architecture kata,” not as an audio-engine project. The architecture you described in your Clave tech stack—**one headless Rust core, multiple genuinely native clients**—maps cleanly to a white-noise app as long as you keep the boundary honest. In practice that means the Rust crate owns the *app’s behavior* (state machine, settings, timers, “what should happen next”), while each platform shell owns the *side effects* (audio playback, OS integration, UI rendering).[^1_1]

They also agree on the most important constraint: **do not put audio playback/decoding in Rust**. GPT-5.5 Thinking and Claude Opus 4.7 both frame this as the key rehearsal: Clave’s document explicitly places audio playback in the platform layer, even if the core emits timing-related events. If you “cheat” and build a cross-platform Rust audio engine here, you’ll learn a different lesson than the one Clave is designed around—and you’ll likely pay extra complexity in WASM size and platform quirks.[^1_1]

A second strong consensus is **API shape**: coarse-grained commands in, snapshots out. Your Clave doc is explicit: avoid tiny getters across the FFI boundary; instead pass a whole snapshot that contains everything the UI needs to render a frame. This project is perfect for practicing that discipline across both UniFFI (Android/Kotlin) and wasm-bindgen (Web/TypeScript). The web-first sequencing is also aligned with your Clave plan: wasm-bindgen + TS tends to expose awkward API shapes early, before you harden them into generated bindings.[^1_1]

**Areas of Divergence.** The models differ mainly on *how much timing/audio sophistication should live in the core*. Claude Opus 4.7 suggests you can use this project to test loop-boundary events and optional loop-point computation inside Rust as “visual hints,” analogous to Clave’s “core describes what happened; UI decides how to animate.” GPT-5.5 Thinking pushes a more minimal stance: keep the shared core about *intent* (“play waterfall loop at 42% volume”) and let the platform’s audio engine handle the mechanics, reporting success/failure back into the core as events. Both are valid; for Clave parity, GPT-5.5’s “effects out / events back” pattern is especially transferable because Clave will also need to separate “intended state” from “platform actually succeeded” (networking, timers, audio start failures, etc.).[^1_1]

There’s also a difference in emphasis around audio format. Gemini 3.1 Pro strongly recommends converting MP3 to OGG/WAV for truly seamless loops, whereas GPT-5.5 and Claude treat that as optional relative to the architectural goal. If your primary goal is “replace Spotify reliably,” format may matter; if the goal is “validate bindings + snapshots + effect dispatch,” you can start with the MP3 and swap formats later without changing the architecture.

**Unique Insights Worth Noting.** Gemini’s suggestion to add a small focus/pomodoro timer is a good way to “force” the Clave-style time boundary: the core should never call system time; instead the UI supplies wall-clock ticks, and the core updates deterministic state. That’s directly aligned with your Clave determinism rules and is an easy feature to test with unit/property tests.[^1_1]

**Recommendations.** Build a tiny repo that mirrors Clave’s shape and boundary rules:

- **`waterfall-core` (Rust, headless):** state machine + settings + `dispatch(action) -> {snapshot, effects}`; no audio APIs.[^1_1]
- **`waterfall-web` (React/Vite):** wasm-bindgen + Web Audio loop + PWA caching of the audio file.
- **`waterfall-android` (Compose):** Media3 ExoPlayer hosted in a `MediaSessionService` for background playback + system controls.

If you want maximum Clave-learning-per-hour, implement **(1) play/pause, (2) volume, (3) sleep timer via Tick events, (4) persisted settings JSON**—then write down what felt awkward across wasm-bindgen vs UniFFI and adjust your “core API rules” for Clave accordingly.[^1_1]
<span style="display:none">[^1_10][^1_11][^1_12][^1_13][^1_14][^1_15][^1_16][^1_17][^1_18][^1_19][^1_2][^1_20][^1_21][^1_22][^1_23][^1_24][^1_25][^1_26][^1_27][^1_28][^1_29][^1_3][^1_30][^1_31][^1_32][^1_33][^1_34][^1_35][^1_36][^1_37][^1_38][^1_39][^1_4][^1_40][^1_41][^1_42][^1_43][^1_44][^1_45][^1_46][^1_47][^1_48][^1_49][^1_5][^1_50][^1_51][^1_52][^1_53][^1_54][^1_55][^1_56][^1_57][^1_58][^1_59][^1_6][^1_60][^1_61][^1_62][^1_63][^1_64][^1_65][^1_66][^1_67][^1_68][^1_69][^1_7][^1_70][^1_71][^1_72][^1_73][^1_74][^1_75][^1_76][^1_77][^1_78][^1_79][^1_8][^1_80][^1_81][^1_82][^1_83][^1_84][^1_85][^1_86][^1_87][^1_88][^1_89][^1_9][^1_90]</span>

<div align="center">⁂</div>

[^1_1]: clave-tech-stack.md

[^1_2]: https://developer.android.com/media/media3

[^1_3]: https://developer.android.com/media/media3/session/background-playback

[^1_4]: https://stackoverflow.com/questions/46926033/create-seamless-loop-of-audio-web

[^1_5]: https://developer.mozilla.org/en-US/docs/Web/API/BaseAudioContext/decodeAudioData

[^1_6]: https://github.com/jfversluis/Plugin.Maui.Audio/issues/44

[^1_7]: https://www.reddit.com/r/androiddev/comments/6559lp/gappless_sound_loop_on_android_and_what_to_do/

[^1_8]: https://developer.android.com/media/platform/mediaplayer

[^1_9]: https://www.youtube.com/watch?v=VhBgYMIvKCg

[^1_10]: https://discussions.unity.com/t/is-gapless-audio-looping-possible-in-unity/496401

[^1_11]: https://www.kevssite.com/seamless-audio-looping/

[^1_12]: https://www.reddit.com/r/androiddev/comments/1j4d078/help_finding_right_audio_format_for_gapless_loops/

[^1_13]: https://support.google.com/googleplay/android-developer/answer/13392821?hl=en

[^1_14]: https://developer.mozilla.org/en-US/docs/Web/API/AudioBufferSourceNode/loop

[^1_15]: https://jackyef.com/posts/building-an-audio-loop-player-on-the-web

[^1_16]: https://crates.io/crates/wasm-bindgen

[^1_17]: https://stackoverflow.com/questions/44217063/exoplayer-not-looping-the-video

[^1_18]: https://github.com/androidx/media/issues/2219

[^1_19]: https://developer.android.com/media/media3/exoplayer/playlists

[^1_20]: https://www.b4x.com/android/forum/threads/gapless-playback-with-android-mediaplayer.62420/

[^1_21]: https://www.youtube.com/watch?v=3yHSLXmc6JM

[^1_22]: https://stackoverflow.com/questions/29882907/how-to-seamlessly-loop-sound-with-web-audio-api

[^1_23]: https://www.reddit.com/r/androiddev/comments/14am1ys/exoplayer_vs_mediaplayer_for_playing_audio_only/

[^1_24]: https://community.appinventor.mit.edu/t/mediaservice-foreground-service-keeps-playback-active-even-when-the-app-is-running-in-the-background/160554

[^1_25]: https://dev.to/myougatheaxo/media3-exoplayer-compose-video-audio-player-implementation-3k3l

[^1_26]: https://devforum.zoom.us/t/app-update-rejected-due-to-foreground-service-permissions/103026

[^1_27]: https://github.com/androidx/media/issues/2382

[^1_28]: https://www.reddit.com/r/androiddev/comments/17hj2d2/forms_for_foreground_service_permissions_are/

[^1_29]: https://www.reddit.com/r/rust/comments/ctf2ks/rust_android_and_webkit/

[^1_30]: https://static.sched.com/hosted_files/wasmcon2023/e9/slides.pdf

[^1_31]: https://red-badger.com/crux

[^1_32]: https://docs.rs/crux_core/latest/crux_core/

[^1_33]: https://github.com/rust-headless-chrome/rust-headless-chrome

[^1_34]: https://sal.dev/android/intro-rust-android-uniffi/

[^1_35]: https://oneuptime.com/blog/post/2026-02-01-rust-webassembly-wasm/view

[^1_36]: https://users.rust-lang.org/t/android-base-easy-graphics-development-on-android-using-rust/23561

[^1_37]: https://lib.rs/crates/gobley-uniffi-bindgen

[^1_38]: https://rustwasm.github.io/docs/wasm-bindgen/

[^1_39]: https://www.mux.com/blog/practical-client-side-rust-for-android-ios-and-web

[^1_40]: https://mozilla.github.io/uniffi-rs/latest/kotlin/gradle.html

[^1_41]: https://yew.rs/docs/0.20/concepts/basic-web-technologies/wasm-bindgen

[^1_42]: https://github.com/mozilla/uniffi-rs

[^1_43]: https://www.reddit.com/r/rust/comments/8azjth/example_shared_model_clientserver_rust_web/

[^1_44]: https://github.com/androidx/media/issues/1592

[^1_45]: https://stackoverflow.com/questions/79510578/android-media3-mediasessionservice-does-not-produce-a-notification

[^1_46]: https://proandroiddev.com/rise-of-jetpack-media-3-revolutionising-media-playback-on-android-45686bdb648

[^1_47]: https://www.youtube.com/watch?v=DH4JSrz6-8I

[^1_48]: https://progressier.com/pwa-capabilities/audio-player-pwa

[^1_49]: https://android-developers.googleblog.com/2023/03/media3-is-ready-to-play.html

[^1_50]: https://stuff.mit.edu/afs/sipb/project/android/docs/training/managing-audio/audio-focus.html

[^1_51]: https://whatpwacando.today/audio/

[^1_52]: https://academy.droidcon.com/course/android-audio-and-video-building-a-music-player-and-video-player-app-with-jetpack-compose

[^1_53]: https://play.google.com/store/apps/details?id=mysterymagination.tools.audiohog\&hl=en_US

[^1_54]: https://meta.discourse.org/t/media-playback-with-pwa-keep-playing-when-phone-locked/182219

[^1_55]: https://www.youtube.com/watch?v=XrcmjIW45u8

[^1_56]: https://discussions.unity.com/t/how-to-manage-audio-focus-on-android-using-unity-solved/624967

[^1_57]: https://www.reddit.com/r/rust/comments/2vn0xx/rustaudio_a_collection_of_crates_for_audio_and/

[^1_58]: https://docs.rs/tinyaudio

[^1_59]: https://github.com/RustAudio/rodio

[^1_60]: https://crates.io/crates/music-player

[^1_61]: https://crates.io/crates/tinyaudio

[^1_62]: https://stackoverflow.com/questions/31868062/android-exoplayer-does-it-solve-gapless-seamless-playback-issue-that-is-brok

[^1_63]: https://arewegameyet.rs/ecosystem/audio/

[^1_64]: https://www.youtube.com/watch?v=0dun8xS-Pqg

[^1_65]: https://stackoverflow.com/questions/65169407/streaming-large-looping-audio-files-with-web-audio-api

[^1_66]: https://lib.rs/multimedia/audio

[^1_67]: https://crates.io/crates/web-audio-api

[^1_68]: https://www.youtube.com/watch?v=G3WWr4uhP0I

[^1_69]: https://www.reddit.com/r/googlehome/comments/1bkwvcc/has_anyone_found_a_good_sleep_noise_solution/

[^1_70]: https://github.com/wasm-bindgen/wasm-bindgen

[^1_71]: https://users.rust-lang.org/t/where-to-do-things-between-wasm-and-javascript/84171

[^1_72]: https://dzfrias.dev/blog/rust-wasm-minimal-setup/

[^1_73]: https://jans.io/docs/v1.7.0/cedarling/uniffi/cedarling-android/

[^1_74]: https://dev.to/willir/one-more-plugin-to-build-rust-for-android-125h

[^1_75]: https://users.rust-lang.org/t/cross-compilation-for-android/107849

[^1_76]: https://rustwasm.github.io/docs/wasm-bindgen/print.html

[^1_77]: https://github.com/mozilla/rust-android-gradle

[^1_78]: https://plugins.gradle.org/search?term=cargo

[^1_79]: https://stackoverflow.com/questions/60826594/android-how-to-play-gapless-audio-loop-in-mediaplayer

[^1_80]: https://github.com/libgdx/libgdx/issues/6622

[^1_81]: https://www.youtube.com/watch?v=VA6nzy2oWQs

[^1_82]: https://www.youtube.com/watch?v=FpcgIkhUwPo

[^1_83]: https://www.reddit.com/r/learnrust/comments/fp9tfk/working_with_sound/

[^1_84]: https://raphlinus.github.io/personal/2018/05/08/ecs-ui.html

[^1_85]: https://www.toptal.com/developers/webassembly/webassembly-rust-tutorial-web-audio

[^1_86]: https://community.kodular.io/t/1-exoplayer-is-no-longer-looping-gapless-2-app-crashes-at-exoplayer-completed-event/41264

[^1_87]: https://github.com/google/ExoPlayer/issues/920

[^1_88]: https://www.reddit.com/r/webdev/comments/10mxodj/as_of_2023_is_there_any_way_to_loop_an_audio_file/

[^1_89]: https://github.com/WebAudio/web-audio-api/issues/1305

[^1_90]: https://github.com/WebAudio/web-audio-api-v2/issues/18

## Model Council Individual Responses on Architecture

### GPT 5.5 Thinking

Recommendation: build this as a tiny “Clave architecture kata”
Yes: this is a good small project for validating the Clave architecture — as long as the Rust core stays headless and does not try to play audio itself.

For this waterfall app, the equivalent of Clave’s “rules engine” is not the MP3 decoder. The useful shared core is the app/session state machine:

What sound is selected?

Is playback intended to be playing or paused?

What loop mode is active?

What volume/fade/sleep-timer settings are active?

What should the UI render?

What command just happened?

What event should the platform layer perform?

The actual audio playback should remain platform-native:

Android: Kotlin + Jetpack Compose + Media3/ExoPlayer + MediaSessionService.

Web: React + TypeScript + Vite + Rust WASM + Web Audio API or <audio> depending on desired loop precision.

Rust core: shared command reducer, state, settings schema, asset manifest, snapshots, tests, and bindings.

This mirrors your Clave doc very closely: one Rust core, native UI per platform, coarse-grained FFI APIs, snapshots rather than field getters, platform-owned audio, and web-first validation through WASM. Your Clave tech stack explicitly says the Rust core owns rules/state/protocol, while platform layers own UI, DOM, gestures, permissions, and audio playback
. Android Media3 is the right native playback stack because it provides Player, ExoPlayer, MediaSession, MediaController, and MediaSessionService as standard playback components
. For background playback on Android, Google specifically recommends putting the Player and MediaSession inside a MediaSessionService so playback can continue while the app is not foregrounded
.

High-level architecture
text
waterfall-noise/
  core/
    waterfall-core/              # Rust headless app core
      src/
        lib.rs
        state.rs
        commands.rs
        reducer.rs
        snapshot.rs
        settings.rs
        manifest.rs
        events.rs
      tests/
        reducer_tests.rs
        snapshot_tests.rs

  bindings/
    uniffi/
      waterfall.udl              # Kotlin bindings for Android
    wasm/
      src/lib.rs                 # wasm-bindgen exports for web

  apps/
    waterfall-web/
      React + TypeScript + Vite
      Web Audio API / HTMLAudioElement
      PWA service worker
      imports Rust WASM core

    waterfall-android/
      Kotlin + Jetpack Compose
      Media3 ExoPlayer
      MediaSessionService
      MediaController
      UniFFI-generated Kotlin bindings
The architecture should deliberately look like a miniature version of Clave:

text
                ┌────────────────────────────┐
                │     waterfall-core Rust     │
                │  state, reducer, settings   │
                │  commands, snapshots, tests │
                └─────────────┬──────────────┘
                              │
          ┌───────────────────┴───────────────────┐
          │                                       │
  wasm-bindgen                              UniFFI Kotlin
          │                                       │
          ▼                                       ▼
┌─────────────────────┐               ┌─────────────────────┐
│ Web React PWA        │               │ Android Compose      │
│ Web Audio / <audio>  │               │ Media3 ExoPlayer     │
│ Media Session API    │               │ MediaSessionService  │
└─────────────────────┘               └─────────────────────┘
The key discipline: Rust decides what should happen; the platform decides how to make it happen.

What the Rust core should own
For this app, the Rust core should own:

Pure app state

rust
pub struct AppState {
    pub selected_sound_id: SoundId,
    pub desired_playback: DesiredPlayback,
    pub loop_mode: LoopMode,
    pub volume_percent: u8,
    pub fade_in_ms: u32,
    pub fade_out_ms: u32,
    pub sleep_timer: Option<SleepTimer>,
    pub session_started_at_ms: Option<u64>,
    pub last_error: Option<AppError>,
}
Commands

rust
pub enum AppCommand {
    Play,
    Pause,
    TogglePlayback,
    SetVolume { percent: u8 },
    SetFadeIn { ms: u32 },
    SetFadeOut { ms: u32 },
    SetSleepTimer { minutes: u32 },
    ClearSleepTimer,
    SelectSound { sound_id: String },
    PlatformPlaybackStarted,
    PlatformPlaybackPaused,
    PlatformPlaybackEndedUnexpectedly,
    PlatformError { message: String },
    Tick { now_ms: u64 },
}
Reducer

rust
pub fn reduce(state: AppState, command: AppCommand) -> AppUpdate
Platform effects

rust
pub struct AppUpdate {
    pub state: AppState,
    pub effects: Vec<PlatformEffect>,
    pub snapshot: AppSnapshot,
}

pub enum PlatformEffect {
    StartPlayback {
        sound_id: String,
        loop_forever: bool,
        volume_percent: u8,
        fade_in_ms: u32,
    },
    PausePlayback {
        fade_out_ms: u32,
    },
    SetPlatformVolume {
        volume_percent: u8,
    },
    ScheduleSleepTimer {
        trigger_at_ms: u64,
    },
    CancelSleepTimer,
    PersistSettings {
        json: String,
    },
}
UI snapshot

rust
pub struct AppSnapshot {
    pub title: String,
    pub subtitle: String,
    pub is_playing_intended: bool,
    pub selected_sound_name: String,
    pub volume_percent: u8,
    pub sleep_timer_label: Option<String>,
    pub primary_button_label: String,
    pub error_message: Option<String>,
}
Settings serialization

rust
pub fn serialize_settings(state: &AppState) -> Result<String, CoreError>;
pub fn restore_from_settings(json: String) -> Result<AppState, CoreError>;
Sound manifest

rust
pub struct SoundManifest {
    pub sounds: Vec<SoundDefinition>,
}

pub struct SoundDefinition {
    pub id: String,
    pub display_name: String,
    pub duration_ms: u32,
    pub asset_key: String,
    pub default_loop: bool,
}
This gives you a real shared core, but keeps it honest. The Rust crate is not doing Android service lifecycle management, Web Audio scheduling, notification permissions, or file decoding.

That is exactly the separation you want for Clave: the core owns domain truth, while each platform owns native behavior.

What the Rust core should not own
Do not put these in Rust:

MP3 decoding.

Audio output.

Android foreground service code.

Web Audio API calls.

Browser autoplay policy handling.

Notification/lock-screen controls.

Android permissions.

Compose or React UI state primitives.

File pickers.

Bluetooth/media-button integration.

This matches your Clave principle that audio playback belongs in the platform layer, while the core may expose timing or state events
. It also keeps the app useful as a Clave rehearsal: Clave should not put Android Nearby Connections, Web Audio API, DOM manipulation, or Compose concepts in the Rust core either
.

Android app architecture
Use:

Kotlin

Jetpack Compose

Media3 ExoPlayer

MediaSessionService

MediaController

UniFFI-generated Kotlin bindings

Bundled MP3 in app assets or res/raw

Android should look like this:

text
MainActivity / Compose UI
        │
        ▼
WaterfallViewModel
        │
        ├── calls Rust core reducer through UniFFI
        │
        ├── exposes AppSnapshot as Compose state
        │
        └── dispatches PlatformEffect to Android services
                         │
                         ▼
              PlaybackService : MediaSessionService
                         │
                         ▼
                   Media3 ExoPlayer
Google’s Media3 docs describe ExoPlayer as the default Player implementation and MediaSession as the standard way for an app to advertise playback and receive external playback commands
. For background playback, the player and media session should live inside a MediaSessionService, not directly in the activity
.

Android playback service
The Android layer should implement a PlaybackService:

kotlin
class PlaybackService : MediaSessionService() {
    private var mediaSession: MediaSession? = null

    override fun onCreate() {
        super.onCreate()

        val player = ExoPlayer.Builder(this).build().apply {
            repeatMode = Player.REPEAT_MODE_ONE
        }

        mediaSession = MediaSession.Builder(this, player).build()
    }

    override fun onDestroy() {
        mediaSession?.run {
            player.release()
            release()
        }
        mediaSession = null
        super.onDestroy()
    }

    override fun onGetSession(
        controllerInfo: MediaSession.ControllerInfo
    ): MediaSession? = mediaSession
}
The Media3 background playback guide says onCreate() is the right place to build the Player and MediaSession, and onDestroy() should release both resources
. Media3’s MediaSessionService also automatically creates a media notification that stays updated with media session state and exposes playback controls
.

Android manifest
You will need the foreground service permissions and service declaration:

xml
<uses-permission android:name="android.permission.FOREGROUND_SERVICE" />
<uses-permission android:name="android.permission.FOREGROUND_SERVICE_MEDIA_PLAYBACK" />
<uses-permission android:name="android.permission.POST_NOTIFICATIONS" />

<service
    android:name=".playback.PlaybackService"
    android:foregroundServiceType="mediaPlayback"
    android:exported="true">
    <intent-filter>
        <action android:name="androidx.media3.session.MediaSessionService" />
        <action android:name="android.media.browse.MediaBrowserService" />
    </intent-filter>
</service>
Android’s Media3 background playback docs state that playback foreground services require FOREGROUND_SERVICE and FOREGROUND_SERVICE_MEDIA_PLAYBACK, and the service declaration should include foregroundServiceType="mediaPlayback" with a MediaSessionService intent filter
. Google Play’s foreground service policy also treats media playback as a recognized foreground service use case for continuing audio or video playback from the background
.

Android UI
The Compose UI can be intentionally tiny:

Big Play/Pause button.

Volume slider.

Sleep timer chips: 15m, 30m, 60m, Off.

Sound title: “Waterfall”.

Optional waveform/loop indicator.

Maybe “fade in/out” controls if you want more state-machine practice.

Example flow:

text
User taps Play
  -> ViewModel sends AppCommand.Play to Rust
  -> Rust returns PlatformEffect.StartPlayback(...)
  -> ViewModel tells PlaybackController to play asset
  -> Android service starts ExoPlayer
  -> ViewModel sends PlatformPlaybackStarted back to Rust
  -> Rust returns updated AppSnapshot
  -> Compose re-renders
This is the same pattern you want in Clave:

text
User plays card
  -> UI sends GameAction to Rust
  -> Rust returns new PlayerSnapshot + VisualEvents
  -> UI animates/render natively
Web app architecture
Use:

React

TypeScript

Vite

wasm-bindgen

Web Audio API for cleaner loops, or <audio loop> if simplicity wins

PWA service worker to cache the MP3 and WASM bundle

Optional Media Session API for lock-screen/media controls

The web shape:

text
React UI
  │
  ▼
useWaterfallCore hook
  │
  ├── calls Rust WASM reducer
  ├── stores AppSnapshot in React state
  └── dispatches PlatformEffect to AudioEngine
                         │
                         ▼
              WebAudioEngine / HtmlAudioEngine
For loop quality, I would start with Web Audio API if you want the loop to be gapless and stable. AudioBufferSourceNode.loop is specifically a boolean that causes the audio buffer to replay when the end is reached
. A practical Web Audio loop player loads the file, decodes it into an AudioBuffer, assigns it to an AudioBufferSourceNode, sets source.loop = true, and starts playback
.

A minimal web engine:

ts
export class WebAudioEngine {
  private context: AudioContext | null = null;
  private buffer: AudioBuffer | null = null;
  private source: AudioBufferSourceNode | null = null;
  private gain: GainNode | null = null;

  async load(url: string) {
    this.context = new AudioContext();
    const response = await fetch(url);
    const arrayBuffer = await response.arrayBuffer();
    this.buffer = await this.context.decodeAudioData(arrayBuffer);
    this.gain = this.context.createGain();
    this.gain.connect(this.context.destination);
  }

  async play(volumePercent: number) {
    if (!this.context || !this.buffer || !this.gain) return;

    this.source?.stop();

    const source = this.context.createBufferSource();
    source.buffer = this.buffer;
    source.loop = true;
    source.connect(this.gain);

    this.gain.gain.value = volumePercent / 100;
    source.start();

    this.source = source;
  }

  pause() {
    this.source?.stop();
    this.source = null;
  }

  setVolume(volumePercent: number) {
    if (this.gain) {
      this.gain.gain.value = volumePercent / 100;
    }
  }
}
For a 13-minute waterfall MP3, an <audio loop> element may be totally acceptable in practice. But if the point is to test architecture, Web Audio gives you a richer platform adapter and resembles Clave’s planned “platform owns audio, Rust owns timing/state” boundary more closely.

The PWA should cache:

index.html

JS/CSS bundle

WASM package

waterfall MP3

manifest icons

That gives you an offline web version that does not depend on Spotify or network loading once installed.

Rust API shape
Keep the FFI surface deliberately coarse, like your Clave doc recommends
.

Good:

rust
pub fn new_app(manifest_json: String, settings_json: Option<String>) -> Result<AppHandle, CoreError>;

pub fn dispatch(handle: &AppHandle, command_json: String) -> Result<AppUpdateJson, CoreError>;

pub fn snapshot(handle: &AppHandle) -> Result<String, CoreError>;

pub fn export_settings(handle: &AppHandle) -> Result<String, CoreError>;
Or more strongly typed for UniFFI:

rust
#[derive(uniffi::Object)]
pub struct WaterfallCore {
    state: Mutex<AppState>,
}

impl WaterfallCore {
    pub fn new(manifest_json: String, settings_json: Option<String>) -> Result<Self, CoreError>;

    pub fn dispatch(&self, command: AppCommand) -> Result<AppUpdate, CoreError>;

    pub fn current_snapshot(&self) -> AppSnapshot;

    pub fn export_settings(&self) -> Result<String, CoreError>;
}
For WASM, you may prefer JSON crossing the boundary:

rust
#[wasm_bindgen]
pub struct WasmWaterfallCore {
    inner: AppState,
}

#[wasm_bindgen]
impl WasmWaterfallCore {
    #[wasm_bindgen(constructor)]
    pub fn new(manifest_json: String, settings_json: Option<String>) -> Result<WasmWaterfallCore, JsValue>;

    pub fn dispatch_json(&mut self, command_json: String) -> Result<JsValue, JsValue>;

    pub fn snapshot_json(&self) -> Result<JsValue, JsValue>;
}
That gives you two experiments:

UniFFI typed objects on Android.

wasm-bindgen + serde JSON/JS values on web.

That is exactly the kind of early API pressure your Clave doc wants from the web-first path: if the Rust API feels awkward through WASM, fix the API before native bindings harden around it
. The wasm-bindgen project’s purpose is facilitating high-level interactions between Rust-generated WASM modules and JavaScript
.

Domain model for the simple app
Even though the app is simple, I would make the model a little more formal than necessary because the goal is architectural practice.

State
rust
pub enum DesiredPlayback {
    Playing,
    Paused,
}

pub enum PlatformPlaybackState {
    Unknown,
    Starting,
    Playing,
    Paused,
    Failed,
}

pub enum LoopMode {
    Forever,
}

pub struct SleepTimer {
    pub started_at_ms: u64,
    pub duration_ms: u64,
}
Distinguish desired playback from actual platform playback.

That matters because Android and web can fail differently:

Android service may not start.

Browser autoplay may block playback until user gesture.

Audio asset may fail to decode.

App may be backgrounded.

Bluetooth/media button may pause playback.

Sleep timer may fire.

So the Rust core can say:

text
desired_playback = Playing
platform_playback_state = Failed
last_error = "Browser blocked audio until user gesture"
Then both UIs can render the same conceptual error.

This is a very useful rehearsal for Clave, where you will also need to separate:

text
desired game action
actual transport result
current authoritative state
platform-specific failure
Platform effect pattern
The cleanest pattern is:

text
Command in
  -> Rust reducer
  -> New state + snapshot + platform effects out
  -> Platform executes effects
  -> Platform reports result back as command
Example:

text
AppCommand.Play
  -> PlatformEffect.StartPlayback
  -> Android calls ExoPlayer.play()
  -> AppCommand.PlatformPlaybackStarted
  -> snapshot says "Playing"
For the web:

text
AppCommand.Play
  -> PlatformEffect.StartPlayback
  -> WebAudioEngine resumes AudioContext and starts loop
  -> AppCommand.PlatformPlaybackStarted
This prevents Rust from pretending it knows whether Android or the browser actually succeeded.

That pattern will transfer beautifully to Clave:

text
GameAction.PlayCard
  -> Rust validates/reduces
  -> VisualEvent.CardMovedToTable
  -> Platform animates
  -> Platform reports animation complete if needed
Suggested app features that test Clave-relevant concerns
Do not stop at one play button. Add a few small features that exercise architecture without bloating the app.

1. Play/pause
Tests:

Command reducer.

Snapshot rendering.

Platform effects.

Media controls.

2. Volume
Tests:

Shared setting.

Platform effect updates.

Slider UI differences between React and Compose.

3. Sleep timer
Tests:

Time as explicit input.

No system clock inside Rust.

Deterministic tests.

Platform scheduling.

Your Clave doc says the core should not call system time; time-relevant inputs should be passed explicitly, and the UI should run wall-clock timers
. This app can practice exactly that:

rust
AppCommand::Tick { now_ms }
The core should not call SystemTime::now().

4. Fade in / fade out
Tests:

Shared intent in Rust.

Platform-native audio implementation.

Different implementation details on Android and web.

Rust says:

rust
PlatformEffect::PausePlayback { fade_out_ms: 1_500 }
Android uses ExoPlayer/gain handling. Web uses GainNode.gain.linearRampToValueAtTime.

5. Persisted settings
Tests:

Serde schema.

Versioned settings.

Restore behavior.

Cross-platform conceptual parity.

rust
pub struct PersistedSettingsV1 {
    pub selected_sound_id: String,
    pub volume_percent: u8,
    pub fade_in_ms: u32,
    pub fade_out_ms: u32,
    pub default_sleep_timer_minutes: Option<u32>,
}
Android stores JSON in DataStore. Web stores JSON in IndexedDB or localStorage.

6. Asset manifest
Tests:

Content-pack-like design.

Runtime-loaded data.

Future extensibility.

json
{
  "version": 1,
  "sounds": [
    {
      "id": "waterfall",
      "displayName": "Waterfall",
      "durationMs": 780000,
      "assetKey": "waterfall_13min_mp3",
      "defaultLoop": true
    }
  ]
}
This is your mini content-pack system. Clave has its own content packs; this app has sound packs.

Android asset strategy
For your immediate use case, bundle the MP3 with the app.

Options:

Option A: res/raw/waterfall.mp3
Pros:

Simple.

Packaged into APK/AAB.

Stable URI.

Cons:

Resource names must be lowercase/underscore.

Less flexible if you later want user-imported files.

Option B: assets/waterfall.mp3
Pros:

Natural file-like asset.

Good for manifest-driven assets.

Cons:

You may need to copy to app-private storage or create an appropriate URI depending on playback path.

For this app, I would use:

text
apps/waterfall-android/app/src/main/assets/sounds/waterfall.mp3
apps/waterfall-android/app/src/main/assets/sounds/manifest.json
Then Android has a small AssetResolver:

kotlin
interface AssetResolver {
    fun uriFor(soundId: String): Uri
}
The Rust core only knows:

text
sound_id = "waterfall"
asset_key = "waterfall_13min_mp3"
Android maps that to an Android Uri. Web maps it to /sounds/waterfall.mp3.

That mirrors Clave nicely: core knows card/content IDs; platform knows actual asset packaging.

Web asset strategy
Put the file in:

text
apps/waterfall-web/public/sounds/waterfall.mp3
apps/waterfall-web/public/sounds/manifest.json
The web manifest says:

json
{
  "waterfall_13min_mp3": "/sounds/waterfall.mp3"
}
The PWA service worker precaches it so the app works offline.

For development, this is frictionless:

bash
pnpm dev
For deployment, host it on Cloudflare Pages, Vercel, Netlify, or your own static infra.

Why not put audio playback in Rust?
It is tempting to say: “If Rust is the core, maybe Rust should handle audio too.”

I would not do that.

For this project, Rust audio would be the wrong lesson. Clave’s architecture explicitly says platform layers own audio playback, including Web Audio API and any generated soundtrack
. Android already has a mature native media stack with Media3, and Media3 is designed to handle media sessions, system controls, background playback, and device-specific playback behavior
. Web playback also has browser-specific constraints around user gestures, AudioContext lifecycle, PWA behavior, and media controls.

Trying to unify low-level playback in Rust would test a different architecture than Clave. It would also make the app harder while teaching you less about the Clave boundary.

The right shared abstraction is:

text
Rust: "Start looping waterfall at 40% volume with 1.5s fade-in."
Android: "I will implement that with Media3."
Web: "I will implement that with Web Audio."
Concrete event loop
User taps Play on Android
text
Compose Button
  -> ViewModel.dispatch(AppCommand.Play)
  -> core.dispatch(...)
  -> AppUpdate {
       snapshot: { primaryButtonLabel: "Pause", ... },
       effects: [
         StartPlayback {
           sound_id: "waterfall",
           loop_forever: true,
           volume_percent: 45,
           fade_in_ms: 1000
         }
       ]
     }
  -> ViewModel executes effect through AndroidPlaybackController
  -> PlaybackController connects to MediaSessionService
  -> ExoPlayer sets media item, repeatMode = ONE, prepare, play
  -> ViewModel.dispatch(AppCommand.PlatformPlaybackStarted)
User taps Play on web
text
React Button
  -> dispatch({ type: "Play" })
  -> WASM core returns StartPlayback
  -> WebAudioEngine.load("/sounds/waterfall.mp3")
  -> AudioBufferSourceNode.loop = true
  -> source.start()
  -> dispatch({ type: "PlatformPlaybackStarted" })
The conceptual flow is identical, but platform implementation remains native.

Testing strategy
This is where the project can really pay off for Clave.

Rust unit tests
Test the reducer heavily:

rust
#[test]
fn play_from_paused_requests_start_playback() {
    let state = AppState::default_paused();
    let update = reduce(state, AppCommand::Play);

    assert_eq!(update.state.desired_playback, DesiredPlayback::Playing);
    assert!(matches!(
        update.effects[0],
        PlatformEffect::StartPlayback { .. }
    ));
}
Snapshot tests
Create canonical JSON snapshots:

text
paused_default.snapshot.json
playing_with_sleep_timer.snapshot.json
failed_autoplay_blocked.snapshot.json
This is directly analogous to Clave PlayerSnapshot tests.

Property tests
Even this simple app can use property tests:

Volume is always clamped to 0..=100.

Sleep timer duration never underflows.

Pause never emits StartPlayback.

Tick after sleep timer emits PausePlayback.

Settings round-trip through JSON.

WASM smoke test
In the web app:

Load WASM.

Create core.

Dispatch Play.

Verify returned effect.

Mock audio engine.

Android smoke test
In Android:

ViewModel creates core.

Dispatch Play.

Mock playback controller receives StartPlayback.

Snapshot updates Compose state.

The heavy correctness work should stay in Rust, just like your Clave plan
.

Build order
I would build it in this order:

Phase 1: Rust core only
Deliver:

waterfall-core

commands

reducer

snapshots

settings

manifest

tests

No UI yet.

Goal: prove the core shape.

Phase 2: Web app first
Deliver:

React UI

WASM binding

Web Audio playback

PWA cache

offline waterfall loop

This matches your Clave web-first reasoning: WASM/TypeScript is the most demanding binding path and will expose awkward API design early
.

Phase 3: Android playback spike
Deliver:

Minimal Compose UI

UniFFI binding

MediaSessionService

ExoPlayer loop

background playback notification

Goal: prove Rust → Kotlin → native service boundary.

Phase 4: Polish Android
Deliver:

Sleep timer

fade in/out

notification metadata

lock-screen controls

persisted settings

Phase 5: Architecture notes for Clave
After finishing, write a short retro:

Which Rust APIs felt clean through WASM?

Which APIs felt awkward through UniFFI?

Did JSON commands or typed commands work better?

Did platform effects scale cleanly?

Did snapshots contain too much or too little?

Did the ViewModel/hook layer become messy?

What would you change before Clave?

That retro is probably the most valuable output of the project.

Suggested repository layout
text
waterfall-noise/
  Cargo.toml
  README.md

  crates/
    waterfall-core/
      Cargo.toml
      src/
        lib.rs
        command.rs
        effect.rs
        state.rs
        reducer.rs
        snapshot.rs
        settings.rs
        manifest.rs
        error.rs
      tests/
        reducer_tests.rs
        settings_roundtrip_tests.rs
        snapshot_tests.rs

    waterfall-uniffi/
      Cargo.toml
      src/
        lib.rs
        waterfall.udl

    waterfall-wasm/
      Cargo.toml
      src/
        lib.rs

  apps/
    web/
      package.json
      vite.config.ts
      public/
        sounds/
          waterfall.mp3
          manifest.json
      src/
        main.tsx
        App.tsx
        core/
          WaterfallCore.ts
        audio/
          WebAudioEngine.ts
          AudioEngine.ts
        hooks/
          useWaterfallApp.ts
        pwa/
          serviceWorker.ts

    android/
      settings.gradle.kts
      build.gradle.kts
      app/
        build.gradle.kts
        src/main/
          AndroidManifest.xml
          assets/
            sounds/
              waterfall.mp3
              manifest.json
          java/com/jacob/waterfall/
            MainActivity.kt
            ui/WaterfallScreen.kt
            ui/WaterfallViewModel.kt
            core/CoreBridge.kt
            playback/PlaybackService.kt
            playback/AndroidPlaybackController.kt
            playback/AssetResolver.kt
This mirrors the Clave structure without prematurely adding iOS, macOS, Windows, or server.

Core API example
A nice reducer result:

json
{
  "snapshot": {
    "title": "Waterfall",
    "subtitle": "Looping background noise",
    "isPlayingIntended": true,
    "selectedSoundName": "Waterfall",
    "volumePercent": 42,
    "sleepTimerLabel": null,
    "primaryButtonLabel": "Pause",
    "errorMessage": null
  },
  "effects": [
    {
      "type": "StartPlayback",
      "soundId": "waterfall",
      "loopForever": true,
      "volumePercent": 42,
      "fadeInMs": 1000
    },
    {
      "type": "PersistSettings",
      "json": "{\"version\":1,\"volumePercent\":42}"
    }
  ]
}
The web and Android app should both consume that same conceptual structure.

ViewModel / hook design
Android ViewModel
kotlin
class WaterfallViewModel(
    private val core: WaterfallCore,
    private val playback: AndroidPlaybackController,
) : ViewModel() {

    var snapshot by mutableStateOf(core.currentSnapshot())
        private set

    fun onPlayPauseClicked() {
        dispatch(AppCommand.TogglePlayback)
    }

    fun onVolumeChanged(percent: Int) {
        dispatch(AppCommand.SetVolume(percent.toUByte()))
    }

    private fun dispatch(command: AppCommand) {
        val update = core.dispatch(command)
        snapshot = update.snapshot
        executeEffects(update.effects)
    }

    private fun executeEffects(effects: List<PlatformEffect>) {
        effects.forEach { effect ->
            when (effect) {
                is PlatformEffect.StartPlayback -> playback.start(effect)
                is PlatformEffect.PausePlayback -> playback.pause(effect)
                is PlatformEffect.SetPlatformVolume -> playback.setVolume(effect.volumePercent)
                else -> Unit
            }
        }
    }
}
React hook
ts
export function useWaterfallApp() {
  const coreRef = useRef<WasmWaterfallCore | null>(null);
  const audioRef = useRef<AudioEngine | null>(null);
  const [snapshot, setSnapshot] = useState<AppSnapshot | null>(null);

  const dispatch = useCallback((command: AppCommand) => {
    const update = coreRef.current!.dispatchJson(JSON.stringify(command));
    setSnapshot(update.snapshot);
    executeEffects(update.effects);
  }, []);

  return {
    snapshot,
    playPause: () => dispatch({ type: "TogglePlayback" }),
    setVolume: (percent: number) => dispatch({ type: "SetVolume", percent }),
  };
}
Same shape. Different platform idioms.

How this maps back to Clave
Waterfall app	Clave equivalent
waterfall-core	clave-core
SoundManifest	content pack
AppCommand	GameAction
AppState	GameState
AppSnapshot	PlayerSnapshot
PlatformEffect	VisualEvent, audio/timer/network effects
Web Audio engine	Web Clave audio scheduler
Android Media3 service	Android Clave music playback layer
Compose UI	Android Clave game UI
React PWA	Clave web client
UniFFI Kotlin	Clave Android binding
wasm-bindgen	Clave web binding
settings JSON	replay/content/settings serialization
sleep timer explicit ticks	Clave phrase/turn timer explicit ticks
The biggest architectural lesson will be whether your “core returns snapshot + effects” pattern feels natural across both React/WASM and Compose/UniFFI.

If it does, that is a strong signal for Clave.

Important design choice: use “effects,” not direct callbacks
Avoid this:

rust
core.play_audio_somehow()
Avoid this too:

rust
core.register_callback(platform_play_audio)
Prefer this:

rust
let update = core.dispatch(AppCommand::Play);
platform.execute(update.effects);
Why?

Easier to test.

Easier to serialize.

Easier to debug.

Easier to replay.

Easier to use across WASM and UniFFI.

Closer to deterministic game architecture.

For Clave, this can become:

rust
GameUpdate {
    state,
    snapshot,
    visual_events,
    audio_events,
    network_messages,
}
Final recommended architecture
Build the app as a native-shell, headless-core, effect-driven architecture:

text
Rust core:
  deterministic state machine
  commands -> state + snapshot + platform effects
  settings and manifest serialization
  no audio playback

Web:
  React PWA
  Rust via wasm-bindgen
  Web Audio API for looping
  service worker caches MP3/WASM
  optional Media Session API controls

Android:
  Jetpack Compose
  Rust via UniFFI Kotlin
  Media3 ExoPlayer inside MediaSessionService
  foreground media playback notification
  bundled local MP3
This is intentionally a little over-architected for a waterfall-noise app, but appropriately over-architected for its real purpose: validating the Clave design before Clave’s game logic, multiplayer, content packs, and native clients make every architectural mistake more expensive.

### Claude Opus 4.7 Thinking

Now I have a clear picture of the Clave architecture. Let me design a parallel "Waterfall" white-noise app that mirrors the same patterns.

Architecture: "Cascade" — A Headless-Core White-Noise App
A deliberate miniature of the Clave architecture. The point isn't that a 13-minute looping MP3 needs a Rust core — it absolutely doesn't. The point is that you exercise every joint of the Clave stack (Rust core → UniFFI → Compose, Rust core → wasm-bindgen → React) on a problem small enough that you can finish it in a weekend and feel every rough edge in the binding layer before the same shapes ossify into the dance game.

Today, May 27, 2026, that lines up perfectly with where Clave is in its phase plan: pre-Phase 1, with web-first sequencing not yet locked in by real code. Cascade is your dress rehearsal.

1. Mission Statement (mirroring Clave's)
A headless Rust core owns all playback state and timing logic. Each platform builds a native UI in its first-party framework, consuming the core through a thin binding layer. The audio file is data, not code.

If you can say that sentence honestly about a white-noise app, you've validated the pattern.

2. What the Core Owns vs. Doesn't
This is the most instructive constraint to impose on yourself, because it's where Clave's whole architecture lives or dies. Be strict.

cascade-core (Rust) OWNS
Playback state machine: Stopped | Playing | Paused | Fading

Loop logic: track position (in samples or ms), seamless loop point calculation, optional crossfade window length

Session model: which sound is loaded, volume (0.0–1.0 as fixed-point integer to keep determinism per Clave §8), fade-in/fade-out durations, sleep timer countdown

Sound pack manifest (the direct analogue of Clave's content packs): SoundPack { id, name, file_ref, duration_ms, recommended_loop_point_ms, license }

Snapshot type (the analogue of PlayerSnapshot): PlaybackSnapshot { state, position_ms, loop_count, volume, sleep_timer_remaining_ms, visual_hint: Option<VisualEvent> }

Action enum: PlaybackAction::{Play, Pause, Stop, SetVolume(u16), StartFadeOut(ms), SetSleepTimer(ms), Tick(elapsed_ms)}

Reducer: apply_action(state, action) -> Result<State, PlaybackError>

Visual hints: VisualEvent::LoopBoundaryCrossed, VisualEvent::FadeStarted — the core says what happened, the UI animates it

Serde-serializable everything, deterministic, no system time inside the core (time is passed in via Tick)

The core does NOT own
The actual audio decoder or output device

File I/O for the MP3

Notification / media-session / lock-screen integration

Foreground service lifecycle (Android) or Service Worker logic (web)

The waveform visualization rendering

Wake locks, audio focus, ducking

This is the key discipline transfer to Clave. On Clave you'll be tempted to put audio playback in the core because "the core keeps time." Doing Cascade first teaches you in microcosm why the event/snapshot boundary is the right one and why the audio engine itself stays in the platform layer.

3. Repository Layout (Clave-shaped)
text
cascade/
├── core/                          # cascade-core Rust crate
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── state.rs               # PlaybackState, reducer
│       ├── actions.rs             # PlaybackAction enum
│       ├── snapshot.rs            # PlaybackSnapshot
│       ├── sound_pack.rs          # SoundPack manifest loader
│       ├── timer.rs               # Sleep timer / fade scheduling (pure)
│       └── protocol.rs            # (stub — for parity with Clave)
│
├── content/
│   └── waterfalls-13min/
│       ├── manifest.toml          # duration, suggested loop point, license
│       └── audio.mp3              # the actual file (or referenced by path)
│
├── bindings/
│   ├── uniffi/
│   │   ├── Cargo.toml
│   │   ├── src/lib.rs
│   │   └── cascade.udl            # single source of truth for Kotlin (+future Swift/C#)
│   └── wasm/
│       ├── Cargo.toml
│       └── src/lib.rs             # #[wasm_bindgen]
│
└── apps/
    ├── cascade-web/               # React + Vite + WASM   ← BUILD FIRST
    │   ├── package.json
    │   ├── vite.config.ts         # vite-plugin-wasm + top-level-await
    │   └── src/
    │       ├── audio/             # Web Audio API engine (NOT in core)
    │       └── ui/                # React components
    │
    └── cascade-android/           # Kotlin + Jetpack Compose
        ├── app/
        │   ├── audio/             # ExoPlayer / Media3 (NOT in core)
        │   └── ui/                # Compose screens
        └── ...
This is structurally identical to Clave's layout, just smaller. That's the value.

4. The Two Binding Mechanisms You're Validating
Web: Rust → WASM → React (the hardest path, per Clave §6.1)
wasm-pack build bindings/wasm --target web --release

vite-plugin-wasm + vite-plugin-top-level-await

serde-wasm-bindgen for PlaybackSnapshot / PlaybackAction

The actual audio: a single <audio> element or, better, a Web Audio API AudioBufferSourceNode with loop = true and loopStart/loopEnd set to the loop points the core computes

The React loop: requestAnimationFrame → read current playback position from the audio element → call core.apply_action(Tick(elapsed_ms)) → get PlaybackSnapshot → render

For "doesn't load reliably" robustness: wrap the MP3 in a Service Worker cache (Workbox) so once it's fetched once it plays offline forever — directly addresses your Spotify pain point

Android: Rust → UniFFI → Kotlin → Compose
cargo build --release --target aarch64-linux-android (+ armv7, x86_64 for emulator)

cascade.udl defines PlaybackSnapshot, PlaybackAction, apply_action, snapshot

UniFFI generates Kotlin bindings → .aar consumed by the Compose app

The actual audio: Media3 / ExoPlayer with a LoopingMediaSource or setRepeatMode(REPEAT_MODE_ONE). Run it inside a MediaSessionService (foreground service) so it survives screen-off and shows lock-screen controls — strictly better than Spotify local files for your use case

The Compose loop: a ViewModel ticks the core every ~250ms with elapsed wall time, exposes the resulting PlaybackSnapshot as a StateFlow, Compose recomposes from it

5. The FFI API (Coarse-Grained, per Clave §7)
rust
// Single UDL surface — every binding sees this
pub fn create_session(pack: SoundPackData) -> PlaybackState;
pub fn apply_action(state: PlaybackState, action: PlaybackAction) -> Result<PlaybackState, PlaybackError>;
pub fn snapshot(state: &PlaybackState) -> PlaybackSnapshot;
pub fn serialize_state(state: &PlaybackState) -> String;     // for debugging / restore
pub fn deserialize_state(json: String) -> Result<PlaybackState, ParseError>;
That's it. Five functions. No get_volume(), no is_playing() — those are fields on PlaybackSnapshot.

If Cascade's API design holds up cleanly through both wasm-bindgen and UniFFI, you've proven the pattern for Clave. If it gets ugly somewhere — async friction, awkward serde shapes, JS↔Rust ownership confusion — you've found exactly the lessons you wanted to find on a project where the cost of fixing them is hours, not weeks.

6. Determinism Test (the cheap miniature of Clave §8)
Write a property test in core/: feed a random sequence of PlaybackActions including Ticks totalling 60 minutes, assert the loop counter, position, and final state are identical across 1000 runs with the same seed. This is Clave's replay-determinism test scaled down. If it passes here, you have a template for the Clave test.

7. Suggested Build Order (mirroring Clave §12, web-first)
Phase	Deliverable	Time estimate
1	cascade-core with reducer, snapshot, full Rust unit + property tests	half day
2	cascade-web: WASM binding, React UI, Web Audio loop, Service Worker cache for the MP3	1 day
3	Deploy cascade-web to Vercel/Netlify as a PWA — this alone replaces your Spotify workaround on every device, today	1 hour
4	UniFFI binding spike: build the .aar, call apply_action from a blank Compose activity	half day (the Clave §2.5 analogue)
5	cascade-android full Compose UI + Media3 foreground service, MSIX-style polish	1 day
Phase 3 is the secret weapon: a deployed PWA solves your immediate "I want waterfalls while I work and Spotify keeps failing me" problem within ~36 hours of starting, while Phases 4–5 validate the harder Android binding path on your own time.

8. What This Specifically De-Risks for Clave
The web-first hypothesis (Clave §6.1, §12). You'll feel firsthand whether the wasm-bindgen path really is the most disciplining FFI mechanism. If yes, Clave's sequencing is validated. If no, you adjust before committing.

The PlayerSnapshot pattern. PlaybackSnapshot is PlayerSnapshot with the serial numbers filed off. Designing one teaches you how rich vs. lean to make the other.

The "core describes, UI animates" rule. A LoopBoundaryCrossed visual hint is a perfect minimum-viable test of the same idea that drives VisualEvent::CardMovedToTable in Clave.

The Tick pattern for time-independent core code (Clave §8). Audio playback is the domain where the temptation to put SystemTime::now() in the core is strongest. Resisting it here is excellent practice.

The content-pack idea. Your single 13-minute waterfalls MP3 with a manifest.toml is the simplest possible content pack. Adding "rain.mp3" later is the same shape as adding another content pack to Clave.

Tooling muscle memory. wasm-pack, vite-plugin-wasm, UniFFI's .udl workflow, the cross-compilation toolchain for aarch64-linux-android — all the same commands you'll run hundreds of times on Clave.

9. What to Resist
Don't add iOS, macOS, Windows, or a server. The whole point of Cascade is to be the smallest possible exercise of the pattern. Two clients (web + Android) hits both binding mechanisms; more would be busywork.

Don't put the audio decoder in Rust even though crates like rodio and symphonia exist. The discipline of keeping audio output in the platform layer is the lesson. ExoPlayer and Web Audio are vastly better at their jobs than anything you'd wire up through FFI.

Don't skip the .udl file and reach for raw JNI even though you could. UniFFI is what you'll use on Clave; use it here.

Don't let scope creep into "ambient sound mixer with 12 sliders." Ship the waterfall app. Add features only after both clients are deployed.

10. One-Line Summary
Cascade is a 13-minute MP3 looper architected as if it were Clave: cascade-core (Rust) + UniFFI/Kotlin/Compose for Android + wasm-bindgen/React for web, sharing one snapshot-and-action API, with the audio engines living strictly on each platform. Build the web client first, deploy it as a PWA the same week, and let it both solve your immediate Spotify problem and shake out the FFI design choices that will shape Clave for the next year.

### Gemini 3.1 Pro Thinking

This is an excellent way to validate the clave-core architecture before committing to the full complexity of Clave's rules engine. It serves as the perfect "Phase 1 / Phase 2" vertical slice to test your FFI boundaries (UniFFI and wasm-bindgen), state synchronization, and UI binding paradigms.

Based on the architectural pillars in clave-tech-stack.md and your focus on a pure native experience, here is how you should architect this white-noise application.

1. The Architectural Boundary (The "Headless Core" Rule)
In your Clave spec, there is a hard rule: The Rust core manages state, rules, and time, but the native shells handle all side effects (UI, Networking, Audio).

For this app, do not use Rust audio crates (like rodio or cpal) inside the core. If you compile audio libraries into the WASM and Android binaries, you violate the "native ecosystem" pillar and drastically increase your WASM payload size.

Instead, the Rust core should act as a pure state machine, while the React web app and the Compose Android app use their respective platform's native audio APIs to play the loop.

2. The Rust Core (waterfall-core)
The core will be a lightweight library compiled to .so (via UniFFI) for Android and .wasm (via wasm-bindgen) for the Web.

To actually test the Clave architecture's snapshot model, give the core some meaningful state to manage rather than just a boolean toggle. A good idea for a focus app is a Pomodoro/Focus Timer.

rust
// Core State (Source of Truth)
pub struct AppSnapshot {
    pub is_playing: bool,
    pub session_duration_seconds: u32,
    pub focus_state: FocusState, // e.g., Working, Break
}

// Coarse-grained Actions (The FFI Boundary)
pub enum AppAction {
    TogglePlayback,
    Tick(u32), // UI sends a wall-clock tick to update the timer
    ResetSession,
}

// Core exposes a pure function to the UI
pub fn apply_action(state: AppState, action: AppAction) -> AppSnapshot { ... }
The Kotlin and TypeScript shells will hold this AppSnapshot. When the user taps play, the shell sends TogglePlayback to Rust. Rust returns an updated snapshot with is_playing: true. The shell reacts to this state change and triggers its native audio player.

3. The Web App (React + wasm-bindgen)
Per your clave-tech-stack.md sequencing plan, build this first to bypass device whitelisting and validate your serialization cleanly.

UI: React 19 + TypeScript + Vite.

State: Use Zustand to hold the Rust AppSnapshot.

Audio Implementation: Do not use the standard HTML5 <audio> tag for seamless loops. Instead, use the Web Audio API
. You will fetch the file, decode it using AudioContext.decodeAudioData(), and attach it to an AudioBufferSourceNode with loop = true
. This guarantees sample-accurate looping at the browser level without delegating back to Rust
.

4. The Android App (Jetpack Compose + UniFFI)
Once the Web App proves the Rust logic, generate the Kotlin bindings via UniFFI.

UI: Jetpack Compose. Your ViewModel maps the UniFFI AppSnapshot to a Compose StateFlow.

Audio Implementation: Do not use the legacy MediaPlayer class, as it suffers from an unavoidable ~0.5-second gap when looping
. Instead, use Jetpack Media3 (ExoPlayer)
.

You can achieve infinite, gapless looping in ExoPlayer by preparing the media item and calling player.setRepeatMode(Player.REPEAT_MODE_ALL)
.

5. The Audio Format Catch (Crucial for Gapless)
You mentioned using a 13-minute MP3. If you want a truly seamless loop, you must convert your MP3 to OGG Vorbis or WAV
.

MP3 is mathematically incapable of true gapless playback because the encoder adds silent padding frames to the beginning and end of the file
. No matter how well you write your ExoPlayer or Web Audio API code, an MP3 will have a micro-stutter when it loops. OGG Vorbis features a variable frame size that natively supports gapless playback on both Android and Web platforms
.

Repository Structure
To perfectly mirror the Clave repo structure, set it up like this:

text
waterfall-app/
├── core/                       # The headless Rust core
│   ├── src/lib.rs              # State machine and timer logic
│   └── Cargo.toml
├── bindings/
│   ├── uniffi/                 # Kotlin bindings generator (.udl file)
│   └── wasm/                   # wasm-bindgen exports
├── apps/
│   ├── waterfall-web/          # React + Vite (Build this first!)
│   │   └── src/audio.ts        # Web Audio API implementation
│   └── waterfall-android/      # Jetpack Compose
│       └── app/src/main/java/  # ExoPlayer implementation
└── assets/
    └── waterfalls.ogg          # Converted from MP3 for gapless looping
This architecture gives you the exact sandbox you need: you get to configure UniFFI, hook up wasm-bindgen, build a unidirectional data flow from a Rust core to native shells, and solve a real problem for your daily workflow, all without needing to mock up 100+ dance cards first.

## Model Council Sythesis on Naming the App

### 1. Where Models Agree

| Finding | GPT-5.5 Thinking | Claude Opus 4.7 Thinking | Gemini 3.1 Pro Thinking | Evidence |
|---------|-----------|-----------|-----------|----------|
| A short, single-word name is best (good for crates/packages/FFI namespaces) | ✓ | ✓ | ✓ | Claude notes name gets baked into crate/package/UniFFI identifiers; GPT proposes consistent repo/crate/app naming. |
| “Cascade/Cascada” is the strongest on-theme option (waterfall + calm) | ✓ | ✓ | ✓ | GPT recommends **Cascada**; Claude recommends **Cascade**; Gemini lists **Cascade** as top pick. |
| Avoid “Cataract” despite being a waterfall term | ✓ | ✓ |  | GPT explicitly warns against it; Claude also flags eye-condition connotation. |
| If you plan to add more sounds later, pick a name that still fits | ✓ | ✓ | ✓ | Claude frames this as “waterfall-only vs multi-sound”; Gemini also suggests more general names if it grows. |

***

### 2. Where Models Disagree

| Topic | GPT-5.5 Thinking | Claude Opus 4.7 Thinking | Gemini 3.1 Pro Thinking | Why They Differ |
|-------|-----------|-----------|-----------|-----------------|
| Best final name | **Cascada** | **Cascade** | Leans **Cascade** (also offers many alternates) | GPT weights distinctiveness + Spanish resonance with *Clave*; Claude weights “architecture vibe” (cascading state updates) + clean identifiers; Gemini offers broader naming strategies. |
| How “Clave-adjacent” the name should be | Subtle sibling vibe (Cascada) | Sibling vibe (Cascade) without being a Clave sub-brand | Offers direct Clave nods (e.g., Son Clave, First Count) | Gemini is more willing to reference the origin project; GPT/Claude prefer a standalone tool name. |
| Should the name be utilitarian vs product-y | Product-y (Cascada/Cascade) | Product-y (Cascade) | Suggests utilitarian options (waterfall-core) too | Gemini optimizes for internal dev vs eventual shipping as a tool; GPT/Claude assume it’ll be used daily and named like a real app. |

***

### 3. Unique Discoveries

| Model | Unique Finding | Why It Matters |
|-------|----------------|----------------|
| Gemini 3.1 Pro Thinking | **Cortina** (“curtain” / interlude between dance sets) | Nice Clave-adjacent reference that still fits “background audio.” |
| Gemini 3.1 Pro Thinking | **Loop.Page** | Could tie neatly to your stephens.page brand, especially for a PWA. |

***

### 4. Comprehensive Analysis

**High-Confidence Findings.** The models converge on the practical reality that this name will end up everywhere: repo name, Rust crate names, Android package ID, UniFFI namespace, and web bundle identifiers. That makes a short, unambiguous, easy-to-type name disproportionately valuable. Both GPT-5.5 Thinking and Claude Opus 4.7 Thinking explicitly optimize for identifiers like `cascade-core`, `cascade-web`, and `cascade-android`, which is exactly the kind of friction you want to avoid in an architecture rehearsal project.

They also largely converge on “Cascade/Cascada” as the best semantic fit: it’s literally a waterfall, it reads as calm background flow, and it subtly matches your existing naming style (*Clave*, *Clave Atlas*)—short, rhythmic, domain-rooted.

**Areas of Divergence.** The core disagreement is **Cascade vs Cascada**. Claude Opus 4.7 favors *Cascade* because it doubles as an architectural metaphor (cascading state updates / unidirectional flow), and because it produces very clean English identifiers. GPT-5.5 favors *Cascada* because it’s warmer and more distinctive while still meaning “waterfall,” and it has a light Spanish resonance that feels adjacent to *Clave* without being a direct “dance” reference. Gemini doesn’t strongly arbitrate; it supports *Cascade* but also proposes additional directions (Clave-nod, utilitarian, brand-tied).

There’s also a strategic question: is this a forever-waterfall app or a future “ambient sounds” app? Claude notes that if you expect multiple sound packs later, a more general name like “Drift” would age better—though “Cascade” can still work fine even if you add rain or café sounds.

**Unique Insights Worth Noting.** Gemini’s *Cortina* is a genuinely clever bridge: it’s dance-adjacent (so it “belongs” near Clave) but still describes background/interlude audio. If you want the app to feel like part of your broader dance/focus ecosystem, it’s a tasteful nod.

**Recommendations.** If you want one decisive pick that’s most consistent with your existing project naming and easiest to live with in code:

- Choose **Cascade** if you want the cleanest identifiers and an architecture-rehearsal vibe.
- Choose **Cascada** if you want something slightly more distinctive/brandable while still on-theme.

If you want me to pick for you based on your stated goal (“a simple tool I’ll actually use daily” + “architecture rehearsal for Clave”), my pick is **Cascade**.

## Reference Files

The clave-tech-stack.md file is at docs/clave-tech-stack.md.

The audio files are in the assets folder. The_Falls_v3 is available in three formats: MP3, OGG, and WAV there.
