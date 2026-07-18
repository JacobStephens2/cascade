# GPT-5.5 Cascade listening-time architecture plan

## Executive summary

**Recommendation:** implement listening-time as a new deterministic ledger inside `cascade-core`, persisted by shells through a new opaque `persistListeningLedger` effect, and synced by a tiny Rust/Axum service using per-device monotonic counters rather than event logs or last-write-wins state. Cascade already uses the right architectural seam: a pure Rust core owns state and time via `Command::Tick`, shells execute side effects, and all bindings exchange one JSON wire shape across wasm-bindgen, UniFFI, and C ABI/PInvoke ([README](https://github.com/JacobStephens2/cascade/blob/8eef7bdfbcde6ff9923a14097bed3fa38c0f97d5/README.md#L44-L59), [wasm wrapper](https://github.com/JacobStephens2/cascade/blob/8eef7bdfbcde6ff9923a14097bed3fa38c0f97d5/crates/cascade-wasm/src/lib.rs#L47-L54), [UniFFI/C ABI wrapper](https://github.com/JacobStephens2/cascade/blob/8eef7bdfbcde6ff9923a14097bed3fa38c0f97d5/crates/cascade-uniffi/src/lib.rs#L72-L85)).

The highest-leverage design decision is to distinguish **intended playback** from **confirmed audio playback** before adding time accounting. Today `Play` sets `PlaybackIntent::Playing`, while `PlatformPlaybackStarted` only clears stale error state, so measuring from `snapshot.isPlaying` would count time after an autoplay-blocked browser `play()` request until the later error command arrives ([state reducer](https://github.com/JacobStephens2/cascade/blob/8eef7bdfbcde6ff9923a14097bed3fa38c0f97d5/crates/cascade-core/src/state.rs#L189-L201), [web effect handler](https://github.com/JacobStephens2/cascade/blob/8eef7bdfbcde6ff9923a14097bed3fa38c0f97d5/apps/web/src/core/useCascade.ts#L92-L105)). Add `audio_confirmed_playing: bool` to core state, flip it on `PlatformPlaybackStarted`, flip it off on `Pause`, `PlatformPlaybackPaused`, and `PlatformPlaybackError`, and accrue only on `Tick` when both `tracking_enabled` and `audio_confirmed_playing` are true.

The second key design decision is to drive ticks while audio is playing, not only while a timer is active. The current web, Android, Apple, and Windows shells intentionally start their 250 ms tick loops only for sleep, pomodoro, or stopwatch timers, so plain untimed listening would not advance any core clock unless those loops are broadened ([web tick loop](https://github.com/JacobStephens2/cascade/blob/8eef7bdfbcde6ff9923a14097bed3fa38c0f97d5/apps/web/src/core/useCascade.ts#L148-L171), [Android tick loop](https://github.com/JacobStephens2/cascade/blob/8eef7bdfbcde6ff9923a14097bed3fa38c0f97d5/apps/android/app/src/main/java/page/stephens/cascade/ui/CascadeViewModel.kt#L23-L33), [Apple tick loop](https://github.com/JacobStephens2/cascade/blob/8eef7bdfbcde6ff9923a14097bed3fa38c0f97d5/apps/apple/CascadeShared/App/AppStore.swift#L101-L110), [Windows tick loop](https://github.com/JacobStephens2/cascade/blob/8eef7bdfbcde6ff9923a14097bed3fa38c0f97d5/apps/windows/Cascade/ViewModels/AppViewModel.cs#L99-L106)). Shells should start ticking when `snapshot.listening.isAccruing || timerActive`, and they should flush one final `Tick` immediately before executing a user pause or platform pause if the scheduler has a pending elapsed delta.

For cross-device aggregation, use a **grow-only per-device counter**: every install has a stable random `device_id`, the core maintains `device_total_ms`, the API accepts absolute counter values, and Postgres stores `max(existing_total_ms, submitted_total_ms)` per `(account_id, device_id)`. This makes retry, offline replay, and concurrent devices idempotent; last-write-wins would lose listening minutes when two devices sync around the same time.

For auth, ship **email magic-link first** and defer passkeys. OIDC is the correct layer when using third-party identity providers rather than raw OAuth for authentication ([OWASP Authentication Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Authentication_Cheat_Sheet.html)), and WebAuthn/passkeys are stronger because the relying party stores public keys while the authenticator keeps private keys ([W3C WebAuthn](https://www.w3.org/TR/webauthn-3/)); however, magic-link is the smallest cross-web/native path for an optional low-risk account whose only purpose is centralizing a counter.

## 1. Where measurement lives: core commands, effects, snapshot fields, and wire shape

### Current convention to preserve

Cascade’s public Rust API is deliberately coarse: `Core::dispatch(Command) -> Update { snapshot, effects }`, and every dispatch serializes a fresh snapshot plus effects for platform execution ([core API](https://github.com/JacobStephens2/cascade/blob/8eef7bdfbcde6ff9923a14097bed3fa38c0f97d5/crates/cascade-core/src/lib.rs#L28-L35), [dispatch implementation](https://github.com/JacobStephens2/cascade/blob/8eef7bdfbcde6ff9923a14097bed3fa38c0f97d5/crates/cascade-core/src/lib.rs#L86-L94)). Commands and effects use internally tagged serde enums with `type` discriminators and camelCase fields, which the web TypeScript DTOs, Android Kotlin sealed classes, Swift manual Codable types, and C# polymorphic JSON records all mirror ([Command serde shape](https://github.com/JacobStephens2/cascade/blob/8eef7bdfbcde6ff9923a14097bed3fa38c0f97d5/crates/cascade-core/src/command.rs#L7-L9), [Effect serde shape](https://github.com/JacobStephens2/cascade/blob/8eef7bdfbcde6ff9923a14097bed3fa38c0f97d5/crates/cascade-core/src/effect.rs#L8-L10), [TypeScript DTOs](https://github.com/JacobStephens2/cascade/blob/8eef7bdfbcde6ff9923a14097bed3fa38c0f97d5/apps/web/src/core/types.ts#L7-L27), [Kotlin DTOs](https://github.com/JacobStephens2/cascade/blob/8eef7bdfbcde6ff9923a14097bed3fa38c0f97d5/apps/android/app/src/main/java/page/stephens/cascade/core/Dto.kt#L21-L46), [Swift DTOs](https://github.com/JacobStephens2/cascade/blob/8eef7bdfbcde6ff9923a14097bed3fa38c0f97d5/apps/apple/CascadeShared/App/Dto.swift#L7-L81), [C# DTOs](https://github.com/JacobStephens2/cascade/blob/8eef7bdfbcde6ff9923a14097bed3fa38c0f97d5/apps/windows/Cascade/Services/Dto.cs#L23-L65)).

The existing `Tick { elapsed_ms }` command is exactly the primitive this feature should reuse, because the core never reads system time and timer math already accumulates elapsed milliseconds from platform-supplied deltas ([Command::Tick](https://github.com/JacobStephens2/cascade/blob/8eef7bdfbcde6ff9923a14097bed3fa38c0f97d5/crates/cascade-core/src/command.rs#L38-L40), [timer module](https://github.com/JacobStephens2/cascade/blob/8eef7bdfbcde6ff9923a14097bed3fa38c0f97d5/crates/cascade-core/src/timer.rs#L1-L7)). The new logic should sit in `state::reduce`, adjacent to the current timer tick branch, so a replay of commands deterministically reproduces the same listening ledger ([reducer](https://github.com/JacobStephens2/cascade/blob/8eef7bdfbcde6ff9923a14097bed3fa38c0f97d5/crates/cascade-core/src/state.rs#L117-L203)).

### Recommended core model

Add a separate `listening.rs` module with a versioned opaque persisted ledger and a small UI snapshot. Keep the ledger as counters, not sessions, because the stated product requirement is total listening time and NIST-style minimization says organizations should minimize PII collection and retention to what is strictly necessary for the business purpose ([NIST SP 800-122](https://nvlpubs.nist.gov/nistpubs/legacy/sp/nistspecialpublication800-122.pdf)).

```rust
// crates/cascade-core/src/listening.rs
pub const LISTENING_LEDGER_VERSION: u32 = 1;
pub const LISTENING_PERSIST_THRESHOLD_MS: u64 = 5_000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PersistedListeningLedger {
    pub version: u32,
    pub device_id: String,              // random UUID generated by shell
    pub tracking_enabled: bool,         // default true on fresh install
    pub device_total_ms: u64,           // grow-only local counter
    pub last_synced_device_total_ms: u64,
    pub server_aggregate_total_ms: u64, // last pulled account-wide total
    pub pending_persist_ms: u64,        // local batching, not shown in UI
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListeningSnapshot {
    pub tracking_enabled: bool,
    pub is_accruing: bool,
    pub device_id: String,
    pub device_total_ms: u64,
    pub unsynced_ms: u64,
    pub aggregate_total_ms: u64,
    pub aggregate_label: String,
}
```

Add the ledger to `State`, but do not put network account identifiers or access tokens into core state. Core should know `device_id`, counters, opt-in status, and server aggregate baseline only, while shells own auth state and network credentials.

```rust
pub struct State {
    pub intent: PlaybackIntent,
    pub audio_confirmed_playing: bool, // new: true only after PlatformPlaybackStarted
    // existing fields...
    pub listening: PersistedListeningLedger,
}
```

Add commands that are pure inputs from shell I/O and backend sync. `RestoreListeningLedger` is intentionally a command rather than a constructor-only path so the wire shape stays consistent and shells can bootstrap with `new/restore_or_new(settings)` then immediately replay the locally persisted ledger before rendering user-facing stats.

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Command {
    // existing variants...
    RestoreListeningLedger { json: String, fallback_device_id: String },
    SetListeningTrackingEnabled { enabled: bool },
    ListeningSyncSucceeded {
        synced_device_total_ms: u64,
        server_aggregate_total_ms: u64,
    },
    ListeningSyncDisabledByServer,
}
```

Add one new effect to persist the listening ledger locally and optionally one effect to delete local listening data after an explicit privacy action. The sync network call should not be an effect from core, because cadence depends on platform reachability, lifecycle, and auth; shells can observe `snapshot.listening.unsyncedMs` and push when appropriate.

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Effect {
    // existing variants...
    PersistListeningLedger { json: String },
    ClearListeningLedger,
}
```

Add a nested `listening` field to `Snapshot` rather than top-level scalar fields, because the current snapshot already groups timer state into a nested `TimerSnapshot` and UIs render from one complete snapshot ([Snapshot structure](https://github.com/JacobStephens2/cascade/blob/8eef7bdfbcde6ff9923a14097bed3fa38c0f97d5/crates/cascade-core/src/snapshot.rs#L35-L47)). The nested field crosses all three bindings as ordinary camelCase JSON, so no FFI surface expansion is needed beyond updating each platform DTO.

```rust
pub struct Snapshot {
    pub title: String,
    pub subtitle: String,
    pub is_playing: bool,
    pub is_muted: bool,
    pub timer: TimerSnapshot,
    pub listening: ListeningSnapshot, // new
    pub error_message: Option<String>,
}
```

Recommended reducer sketch:

```rust
pub fn reduce(state: &mut State, command: Command, effects: &mut Vec<Effect>) {
    state.timer_just_completed = None;

    match command {
        Command::Tick { elapsed_ms } => {
            // Existing timer behavior remains first or second; both are pure.
            tick_active_timer(state, elapsed_ms, effects);

            if state.listening.tracking_enabled && state.audio_confirmed_playing {
                state.listening.device_total_ms =
                    state.listening.device_total_ms.saturating_add(elapsed_ms);
                state.listening.pending_persist_ms =
                    state.listening.pending_persist_ms.saturating_add(elapsed_ms);

                if state.listening.pending_persist_ms >= LISTENING_PERSIST_THRESHOLD_MS {
                    state.listening.pending_persist_ms = 0;
                    push_persist_listening(state, effects);
                }
            }
        }

        Command::PlatformPlaybackStarted => {
            state.audio_confirmed_playing = true;
            state.last_error = None;
            push_persist_listening(state, effects); // captures accruing state after crash
        }

        Command::Pause | Command::PlatformPlaybackPaused => {
            state.audio_confirmed_playing = false;
            stop_playback_or_sync_intent_as_today(state, effects);
            push_persist_listening(state, effects); // flush counter at boundary
        }

        Command::PlatformPlaybackError { message } => {
            state.audio_confirmed_playing = false;
            state.last_error = Some(message);
            state.intent = PlaybackIntent::Paused;
            push_persist_listening(state, effects);
        }

        Command::SetListeningTrackingEnabled { enabled } => {
            state.listening.tracking_enabled = enabled;
            if !enabled {
                // Recommendation: keep total visible locally unless user chooses
                // "delete listening data"; just stop future accrual here.
                state.listening.pending_persist_ms = 0;
            }
            push_persist_listening(state, effects);
        }

        Command::ListeningSyncSucceeded { synced_device_total_ms, server_aggregate_total_ms } => {
            state.listening.last_synced_device_total_ms =
                state.listening.last_synced_device_total_ms.max(synced_device_total_ms);
            state.listening.server_aggregate_total_ms = server_aggregate_total_ms;
            push_persist_listening(state, effects);
        }

        Command::RestoreListeningLedger { json, fallback_device_id } => {
            state.listening = PersistedListeningLedger::restore_or_default(&json, fallback_device_id);
        }

        // Existing commands unchanged, except Play should not set audio_confirmed_playing.
        other => reduce_existing(state, other, effects),
    }
}

impl ListeningSnapshot {
    fn from_ledger(l: &PersistedListeningLedger, audio_confirmed_playing: bool) -> Self {
        let unsynced = l.device_total_ms.saturating_sub(l.last_synced_device_total_ms);
        let aggregate = l.server_aggregate_total_ms.saturating_add(unsynced);
        Self {
            tracking_enabled: l.tracking_enabled,
            is_accruing: l.tracking_enabled && audio_confirmed_playing,
            device_id: l.device_id.clone(),
            device_total_ms: l.device_total_ms,
            unsynced_ms: unsynced,
            aggregate_total_ms: aggregate,
            aggregate_label: format_duration(aggregate),
        }
    }
}
```

### Binding updates

For web, add `listening` to `Snapshot`, add `PersistListeningLedger` and `ClearListeningLedger` to `Effect`, add the three listening commands to `Command`, persist the effect to `localStorage` under `cascade.listening.v1`, and broaden the tick-loop predicate from `timerActive` to `timerActive || snapshot.listening.isAccruing` ([web DTO location](https://github.com/JacobStephens2/cascade/blob/8eef7bdfbcde6ff9923a14097bed3fa38c0f97d5/apps/web/src/core/types.ts#L7-L57), [web persistence effect switch](https://github.com/JacobStephens2/cascade/blob/8eef7bdfbcde6ff9923a14097bed3fa38c0f97d5/apps/web/src/core/useCascade.ts#L87-L121)).

For Android, update `Dto.kt`, have `SettingsStore` either become a more general `CoreStateStore` with separate `settings_v1` and `listening_v1` keys or add `ListeningStore`, and broaden `CascadeViewModel`’s active tick predicate to include `snap.listening.isAccruing` ([Kotlin DTOs](https://github.com/JacobStephens2/cascade/blob/8eef7bdfbcde6ff9923a14097bed3fa38c0f97d5/apps/android/app/src/main/java/page/stephens/cascade/core/Dto.kt#L21-L82), [Android settings store](https://github.com/JacobStephens2/cascade/blob/8eef7bdfbcde6ff9923a14097bed3fa38c0f97d5/apps/android/app/src/main/java/page/stephens/cascade/settings/SettingsStore.kt#L10-L25), [Android tick predicate](https://github.com/JacobStephens2/cascade/blob/8eef7bdfbcde6ff9923a14097bed3fa38c0f97d5/apps/android/app/src/main/java/page/stephens/cascade/ui/CascadeViewModel.kt#L23-L33)).

For Apple, update `Dto.swift`, add a `ListeningStore` adjacent to `SettingsStore`, persist the new effect in `AppStore.apply`, and broaden `timerActive` to `timerActive || update.snapshot.listening.isAccruing` ([Swift DTOs](https://github.com/JacobStephens2/cascade/blob/8eef7bdfbcde6ff9923a14097bed3fa38c0f97d5/apps/apple/CascadeShared/App/Dto.swift#L77-L129), [SettingsStore path](https://github.com/JacobStephens2/cascade/blob/8eef7bdfbcde6ff9923a14097bed3fa38c0f97d5/apps/apple/CascadeShared/Persistence/SettingsStore.swift#L3-L37), [AppStore effect switch and tick loop](https://github.com/JacobStephens2/cascade/blob/8eef7bdfbcde6ff9923a14097bed3fa38c0f97d5/apps/apple/CascadeShared/App/AppStore.swift#L85-L110)).

For Windows, update `Dto.cs`, add a `ListeningStore` next to `SettingsStore`, persist the new effect in `AppViewModel.Apply`, and broaden `nowActive` to include `Snapshot.Listening.IsAccruing` ([C# DTOs](https://github.com/JacobStephens2/cascade/blob/8eef7bdfbcde6ff9923a14097bed3fa38c0f97d5/apps/windows/Cascade/Services/Dto.cs#L53-L121), [Windows SettingsStore](https://github.com/JacobStephens2/cascade/blob/8eef7bdfbcde6ff9923a14097bed3fa38c0f97d5/apps/windows/Cascade/Services/SettingsStore.cs#L6-L49), [Windows effect switch and tick loop](https://github.com/JacobStephens2/cascade/blob/8eef7bdfbcde6ff9923a14097bed3fa38c0f97d5/apps/windows/Cascade/ViewModels/AppViewModel.cs#L69-L108)).

## 2. Local persistence per shell

**Web/PWA:** persist `PersistListeningLedger.json` to `localStorage` key `cascade.listening.v1`, because the current web shell already stores opaque core settings JSON in `localStorage` and also stores a live-session JSON blob there for reload recovery ([web storage keys](https://github.com/JacobStephens2/cascade/blob/8eef7bdfbcde6ff9923a14097bed3fa38c0f97d5/apps/web/src/core/useCascade.ts#L7-L10), [web settings write](https://github.com/JacobStephens2/cascade/blob/8eef7bdfbcde6ff9923a14097bed3fa38c0f97d5/apps/web/src/core/useCascade.ts#L113-L119), [web session write](https://github.com/JacobStephens2/cascade/blob/8eef7bdfbcde6ff9923a14097bed3fa38c0f97d5/apps/web/src/core/useCascade.ts#L231-L253)). If token storage is needed for account sync, use an HTTP-only secure cookie for the web session if the API is same-site, but keep the listening ledger itself as the core-owned local JSON.

**Android:** persist the listening ledger in Jetpack DataStore under a new `stringPreferencesKey("listening_v1")`, because the existing Android shell already uses Preferences DataStore to round-trip the opaque settings JSON and the bridge holder already centralizes effect handling ([Android DataStore declaration](https://github.com/JacobStephens2/cascade/blob/8eef7bdfbcde6ff9923a14097bed3fa38c0f97d5/apps/android/app/src/main/java/page/stephens/cascade/settings/SettingsStore.kt#L10-L25), [BridgeHolder effect persistence](https://github.com/JacobStephens2/cascade/blob/8eef7bdfbcde6ff9923a14097bed3fa38c0f97d5/apps/android/app/src/main/java/page/stephens/cascade/core/CascadeBridgeHolder.kt#L47-L55)). Store refresh tokens separately through Android Keystore-backed storage, because account credentials are shell/network state rather than core state.

**macOS:** persist `listening.json` in `~/Library/Application Support/Cascade/` next to `settings.json`, because the existing Swift `SettingsStore` writes opaque core JSON atomically to that directory ([Apple SettingsStore](https://github.com/JacobStephens2/cascade/blob/8eef7bdfbcde6ff9923a14097bed3fa38c0f97d5/apps/apple/CascadeShared/Persistence/SettingsStore.swift#L3-L37)). Store account refresh tokens in Keychain, because Keychain is the platform-appropriate shell-owned store for credentials while core stays I/O-free.

**iOS:** use the same `CascadeShared/Persistence/ListeningStore.swift` implementation under the app container’s Application Support directory, because the iOS target shares non-UI code through `CascadeShared` with macOS ([iOS runbook](https://github.com/JacobStephens2/cascade/blob/8eef7bdfbcde6ff9923a14097bed3fa38c0f97d5/docs/running-ios.md#L7-L10)). Store account refresh tokens in iOS Keychain, and trigger sync from the app foreground/background lifecycle with best-effort background execution only while audio is already keeping the app alive.

**Windows:** persist `%LOCALAPPDATA%\Cascade\listening.json` next to `%LOCALAPPDATA%\Cascade\settings.json`, because the Windows shell already uses this directory and treats settings JSON as an opaque Rust-owned blob ([Windows SettingsStore](https://github.com/JacobStephens2/cascade/blob/8eef7bdfbcde6ff9923a14097bed3fa38c0f97d5/apps/windows/Cascade/Services/SettingsStore.cs#L6-L49)). Store account refresh tokens with Windows Credential Manager or DPAPI-protected local settings, because credentials should not be embedded in the core-owned JSON ledger.

**watchOS:** do not create an independent listening ledger in v1, because the current watch app is a thin remote that has no Rust core, no audio engine, and sends commands to the iPhone over `WCSession` while the iPhone owns state and playback ([watchOS runbook](https://github.com/JacobStephens2/cascade/blob/8eef7bdfbcde6ff9923a14097bed3fa38c0f97d5/docs/running-watchos.md#L7-L11), [watch client cache](https://github.com/JacobStephens2/cascade/blob/8eef7bdfbcde6ff9923a14097bed3fa38c0f97d5/apps/apple/CascadeWatch/Services/WatchConnectivityClient.swift#L17-L28)). If standalone watch playback is later implemented, add a watch-local ledger in the watch app container and treat the watch as a distinct `device_id`, but that is explicitly outside the current Mode A watch architecture ([watchOS follow-up list](https://github.com/JacobStephens2/cascade/blob/8eef7bdfbcde6ff9923a14097bed3fa38c0f97d5/docs/running-watchos.md#L103-L118)).

## 3. Backend: account, auth, API, and Postgres schema

### Recommended service shape

Use a small Rust service on the existing VPS: Axum for HTTP routing and SQLx for async Postgres access. Axum is documented as an HTTP routing and request-handling library focused on ergonomics and modularity ([Axum docs](https://docs.rs/axum/latest/axum/)), and SQLx is documented as an async SQL toolkit with a PostgreSQL driver and `PgPool`/`PgConnection` support ([SQLx docs](https://docs.rs/sqlx/latest/sqlx/)). Run it as `cascade-listening.service` under systemd on `127.0.0.1:3090`, and put Apache in front at `https://api.cascade.stephens.page` to match the existing deployment style where Apache serves `cascade.stephens.page` with HTTPS, cache headers, and certbot-managed certificates ([Apache vhost](https://github.com/JacobStephens2/cascade/blob/8eef7bdfbcde6ff9923a14097bed3fa38c0f97d5/deploy/cascade.stephens.page.apache.conf#L18-L88)).

Do not add managed cloud auth or analytics infrastructure for this feature. The deployment context already includes Postgres and a modest VPS, and the feature only needs low-volume account lookup, device counter upsert, and aggregate reads.

### Account model

Use a local anonymous install identity first and an optional account identity later. Every install generates a UUID `device_id` before account creation, the core uses that ID in the local ledger, and signing in attaches the device counter to a `user_id` without changing offline behavior.

Use email as the only real identity attribute in v1. OAuth/OIDC adds provider complexity, app-deep-link complexity, and third-party identity dependency; OIDC is the correct identity layer on top of OAuth when using external providers, but this app does not need external user profile claims ([OWASP Authentication Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Authentication_Cheat_Sheet.html)). Passkeys/WebAuthn are the best long-term sign-in method because the site stores a public key and the authenticator keeps the private key, but cross-platform native passkey implementation is too much surface area for a first sync feature ([W3C WebAuthn](https://www.w3.org/TR/webauthn-3/)).

### Auth recommendation

Ship email magic-link for v1: `POST /auth/magic-link` sends a short-lived link, `POST /auth/exchange` turns a one-time code into an access token and refresh token, and native apps use universal/app links or a copyable code fallback. This is the smallest passwordless path, it avoids password storage entirely, and it can later coexist with passkeys as a second login method.

Use opaque random tokens stored server-side rather than self-contained JWTs. Opaque sessions make account deletion, logout-all-devices, and token revocation simple on a single Postgres-backed VPS.

### Minimal API surface

```http
POST /v1/auth/magic-link
{ "email": "user@example.com", "deviceId": "uuid", "returnUrl": "cascade://auth" }
-> 204  // always, to avoid account enumeration

POST /v1/auth/exchange
{ "code": "single-use-code", "deviceId": "uuid", "deviceName": "Android Pixel 8" }
-> { "accessToken": "opaque", "refreshToken": "opaque", "user": { "email": "user@example.com", "trackingEnabled": true } }

POST /v1/auth/refresh
{ "refreshToken": "opaque" }
-> { "accessToken": "opaque", "refreshToken": "opaque" }

GET /v1/listening
Authorization: Bearer <access>
-> { "trackingEnabled": true, "aggregateTotalMs": 1234567, "devices": [{ "deviceId": "uuid", "totalMs": 456789 }] }

PUT /v1/devices/{deviceId}/listening-counter
Authorization: Bearer <access>
{ "deviceTotalMs": 456789, "ledgerVersion": 1 }
-> { "trackingEnabled": true, "acceptedDeviceTotalMs": 456789, "aggregateTotalMs": 1234567 }

PATCH /v1/me/settings
Authorization: Bearer <access>
{ "trackingEnabled": false }
-> { "trackingEnabled": false }

DELETE /v1/me/listening
Authorization: Bearer <access>
-> 204  // deletes counters but not account

DELETE /v1/me
Authorization: Bearer <access>
-> 204  // deletes account, devices, sessions, counters
```

### Postgres schema sketch

```sql
create table users (
  id uuid primary key default gen_random_uuid(),
  email citext unique not null,
  tracking_enabled boolean not null default true,
  listening_deleted_at timestamptz,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create table devices (
  id uuid primary key,                       -- client-generated device_id
  user_id uuid not null references users(id) on delete cascade,
  display_name text,
  platform text not null check (platform in ('web','android','macos','ios','windows','watchos')),
  created_at timestamptz not null default now(),
  last_seen_at timestamptz
);

create table device_listening_counters (
  user_id uuid not null references users(id) on delete cascade,
  device_id uuid not null references devices(id) on delete cascade,
  total_ms bigint not null check (total_ms >= 0),
  updated_at timestamptz not null default now(),
  primary key (user_id, device_id)
);

create table magic_login_codes (
  code_hash bytea primary key,
  email citext not null,
  device_id uuid,
  expires_at timestamptz not null,
  consumed_at timestamptz,
  created_at timestamptz not null default now()
);

create table sessions (
  id uuid primary key default gen_random_uuid(),
  user_id uuid not null references users(id) on delete cascade,
  device_id uuid references devices(id) on delete set null,
  refresh_token_hash bytea unique not null,
  expires_at timestamptz not null,
  revoked_at timestamptz,
  created_at timestamptz not null default now()
);

-- Counter upsert: grow-only and idempotent.
insert into device_listening_counters(user_id, device_id, total_ms)
values ($1, $2, $3)
on conflict (user_id, device_id) do update
set total_ms = greatest(device_listening_counters.total_ms, excluded.total_ms),
    updated_at = now();
```

## 4. Sync protocol: offline accumulation, push/pull cadence, idempotency, and conflicts

### Recommendation: G-counter style per-device sums

Use CRDT-style grow-only counters, not last-write-wins. A device counter is monotonic, retrying the same PUT is harmless, offline replay is just a later higher counter, and concurrent devices merge by summing per-device maxima.

If two signed-in devices play audio for the same 10 wall-clock minutes, count 20 listening-minutes. Deduplicating simultaneous human listening would require timestamped intervals, clock trust, and overlap logic, which is more data than the product needs and conflicts with minimizing collection to what is strictly necessary ([NIST SP 800-122](https://nvlpubs.nist.gov/nistpubs/legacy/sp/nistspecialpublication800-122.pdf)).

### Client algorithm

```pseudo
on app boot:
  settings_json = shell.read_settings()
  listening_json = shell.read_listening_ledger()
  core = Core.restore_or_new(settings_json)
  core.dispatch(RestoreListeningLedger { json: listening_json, fallbackDeviceId: shell.device_id() })
  if account_session_exists:
    enqueue_pull_then_push()

on each dispatch update:
  for effect in update.effects:
    if effect.type == "persistListeningLedger":
      shell.write_listening_ledger(effect.json)
  if authenticated and update.snapshot.listening.trackingEnabled:
    if update.snapshot.listening.unsyncedMs >= 30_000:
      debounce(sync_now, 10 seconds)

sync_now:
  snap = core.snapshot()
  if !authenticated or !snap.listening.trackingEnabled:
    return
  response = PUT /v1/devices/{snap.listening.deviceId}/listening-counter {
    deviceTotalMs: snap.listening.deviceTotalMs,
    ledgerVersion: 1
  }
  if response.ok:
    core.dispatch(ListeningSyncSucceeded {
      syncedDeviceTotalMs: response.acceptedDeviceTotalMs,
      serverAggregateTotalMs: response.aggregateTotalMs
    })
  else:
    backoff_and_retry_later()

on app foreground / network regained / playback paused:
  sync_now()
```

Use a 30-second unsynced threshold plus sync-on-pause and sync-on-foreground. The local ledger persists every 5 seconds or at play/pause boundaries, while network sync can be much less frequent because the absolute counter makes retries safe.

### Server merge algorithm

```pseudo
put_counter(user_id, device_id, submitted_total_ms):
  assert user.tracking_enabled
  ensure device row exists and belongs to user
  old = select total_ms from counters where user_id = $user and device_id = $device
  accepted = max(old or 0, submitted_total_ms)
  upsert counter total_ms = accepted
  aggregate = select sum(total_ms) from counters where user_id = $user
  return { acceptedDeviceTotalMs: accepted, aggregateTotalMs: aggregate, trackingEnabled: true }
```

Lost acknowledgements are safe because the client resends the same or higher `deviceTotalMs`. Out-of-order requests are safe because the server keeps the maximum. Device clock skew does not affect merge correctness because the server never needs client timestamps for totals.

### Opt-out and reset behavior

When a user toggles tracking off, core stops local accrual immediately, persists the local setting, and an authenticated shell sends `PATCH /v1/me/settings { trackingEnabled: false }`. When a user chooses “delete listening data,” the shell calls `DELETE /v1/me/listening`, then dispatches a local reset command that clears counters and generates a new `device_id` so an old server-side max cannot resurrect deleted totals.

## 5. Privacy and data minimization

This feature changes Cascade’s privacy posture materially because the current web privacy page says Cascade has no accounts, no analytics, no trackers, no servers receiving data about users, and no native data-collection network calls ([privacy page](https://github.com/JacobStephens2/cascade/blob/8eef7bdfbcde6ff9923a14097bed3fa38c0f97d5/apps/web/public/privacy/index.html#L130-L159)). Update the privacy page before shipping account sync, and make the first-run/settings UI state plainly say “Listening time tracking is on; sign in only if you want cross-device sync.”

Store only these fields server-side: email, user id, device id, platform, device display name, tracking-enabled flag, per-device total milliseconds, aggregate computed by query, session token hashes, and audit timestamps. Do not store raw listening events, play/pause timestamps, IP-derived location, audio titles beyond the one built-in waterfall app, or per-day buckets in v1.

Tracking-on-by-default is defensible only if there is no network transmission without account sign-in, the setting is visible, and opt-out stops both local accrual and server sync. NIST’s privacy guidance emphasizes collection limitation, purpose specification, use limitation, consent/knowledge, and retention only as long as necessary for the specified purpose ([NIST SP 800-122](https://nvlpubs.nist.gov/nistpubs/legacy/sp/nistspecialpublication800-122.pdf)).

Recommended opt-out semantics are precise: toggling tracking off stops future local accrual, persists the disabled flag, hides or freezes totals depending on UX choice, and syncs the disabled account setting when signed in. Recommended deletion semantics are stronger: “Delete listening data” deletes server counters, clears local ledger on every device as it next syncs, and rotates the local `device_id` so stale offline PUTs cannot reattach old totals.

Retention should be account-lifetime for counters and 90 days for expired magic-login codes and revoked sessions. Account deletion should hard-delete users, devices, counters, sessions, and magic codes through `ON DELETE CASCADE`, while aggregate counters should be recomputed from device rows rather than stored as a separate denormalized user field.

## 6. Phased build order aligned to Cascade’s core-first and shell-by-shell convention

### Phase 0 — validation spike

First validate the “actual audio” edge cases on web and Android, because those are the two targets the README says are built and verified on the Linux dev host ([README platform verification](https://github.com/JacobStephens2/cascade/blob/8eef7bdfbcde6ff9923a14097bed3fa38c0f97d5/README.md#L29-L32)). Confirm that `PlatformPlaybackStarted` fires promptly after successful audio start, that failures dispatch `PlatformPlaybackError`, and that the tick loop can run during untimed playback without visible battery or render churn.

### Phase 1 — core only

Add `listening.rs`, core state fields, command/effect/snapshot fields, serde wire tests, and property tests. Extend the existing proptest command strategy with `SetListeningTrackingEnabled`, `RestoreListeningLedger`, and `ListeningSyncSucceeded`, then assert that listening total increases exactly by ticked milliseconds only when tracking is enabled and platform playback is confirmed, similar to the existing stopwatch property that checks exact tick accumulation ([property test style](https://github.com/JacobStephens2/cascade/blob/8eef7bdfbcde6ff9923a14097bed3fa38c0f97d5/crates/cascade-core/tests/property_tests.rs#L145-L162)).

Suggested core tests:

```rust
#[test]
fn does_not_accrue_before_platform_started() { ... }

#[test]
fn accrues_on_tick_while_confirmed_playing() { ... }

#[test]
fn platform_pause_stops_accrual() { ... }

#[test]
fn sync_ack_advances_server_baseline_without_changing_device_total() { ... }

proptest! {
  #[test]
  fn listening_total_is_monotonic(cmds in command_strategy_with_listening()) { ... }
}
```

### Phase 2 — web shell

Update TypeScript DTOs, add localStorage ledger restore/persist, broaden the tick loop to `timerActive || snapshot.listening.isAccruing`, add a small settings UI toggle, and show total listening time somewhere low-risk such as Settings before making it prominent. Web is the fastest runtime verification path because the repository already documents wasm-pack plus Vite for local development ([web quick start](https://github.com/JacobStephens2/cascade/blob/8eef7bdfbcde6ff9923a14097bed3fa38c0f97d5/README.md#L92-L99)).

### Phase 3 — Android shell

Update Kotlin DTOs, add `ListeningStore` via DataStore, broaden `CascadeViewModel` ticking, add the settings toggle, and manually verify plain untimed playback increments the counter while app background audio is active. Android is the second locally verifiable platform because the README states the Android shell builds as a debug APK on the Linux dev host ([README Android build](https://github.com/JacobStephens2/cascade/blob/8eef7bdfbcde6ff9923a14097bed3fa38c0f97d5/README.md#L101-L104)).

### Phase 4 — backend and web sync

Build the Axum/SQLx service, Postgres migrations, systemd unit, Apache vhost, and web client sync worker first. Deploy behind `api.cascade.stephens.page`, use rate-limited magic links, and verify offline web increments followed by online push/pull.

### Phase 5 — native sync shell-by-shell

Add sync/auth to Android after web proves the backend, then Windows, then macOS/iOS, and leave watchOS as a phone-remote display/control surface. Windows should be treated as compile-only in CI unless a Windows machine is available, because the workflow comments state WinUI has no cross-compile path and the GitHub Actions job is the compile gate ([Windows workflow](https://github.com/JacobStephens2/cascade/blob/8eef7bdfbcde6ff9923a14097bed3fa38c0f97d5/.github/workflows/windows.yml#L3-L6), [Windows build step](https://github.com/JacobStephens2/cascade/blob/8eef7bdfbcde6ff9923a14097bed3fa38c0f97d5/.github/workflows/windows.yml#L84-L88)). Apple targets should be treated as CI-compiled unless a Mac is available, because the Apple workflow says it is the first real compile for macOS, iOS, and watchOS from a Linux-authored codebase ([Apple workflow](https://github.com/JacobStephens2/cascade/blob/8eef7bdfbcde6ff9923a14097bed3fa38c0f97d5/.github/workflows/apple.yml#L3-L10)).

### Phase 6 — privacy launch gate

Update the privacy page, add account deletion and listening-data deletion UI, and add a migration note for users who installed when the privacy page promised no accounts or network data collection. Do not deploy tracking-on-by-default sync behavior until the privacy copy and opt-out are in the same release artifact.

## Riskiest unknowns and what to validate first

**Riskiest unknown 1: confirmed playback events are not equally reliable on every shell.** The code already has `PlatformPlaybackStarted`, `PlatformPlaybackPaused`, and `PlatformPlaybackError`, but the reducer does not currently maintain an actual-playback flag, and each shell’s audio engine may differ in how promptly it reports start/pause ([Command variants](https://github.com/JacobStephens2/cascade/blob/8eef7bdfbcde6ff9923a14097bed3fa38c0f97d5/crates/cascade-core/src/command.rs#L42-L49), [current platform-start reducer](https://github.com/JacobStephens2/cascade/blob/8eef7bdfbcde6ff9923a14097bed3fa38c0f97d5/crates/cascade-core/src/state.rs#L189-L201)). Validate web autoplay failure, Android audio focus loss, iOS lock-screen pause, and Windows SMTC pause before trusting the counter.

**Riskiest unknown 2: broadening the tick loop may affect battery or UI churn.** Today the tick loop intentionally runs only while timers are active on web, Android, Apple, and Windows, so untimed all-day listening would introduce a new 250 ms loop on every platform ([web tick loop](https://github.com/JacobStephens2/cascade/blob/8eef7bdfbcde6ff9923a14097bed3fa38c0f97d5/apps/web/src/core/useCascade.ts#L148-L171), [Windows tick scheduler](https://github.com/JacobStephens2/cascade/blob/8eef7bdfbcde6ff9923a14097bed3fa38c0f97d5/apps/windows/Cascade/Services/TickScheduler.cs#L24-L40)). Validate whether listening-time accrual can tick at 1 second while timers keep 250 ms for smooth countdowns, because counters do not need sub-second visual updates.

**Riskiest unknown 3: tracking-on-by-default conflicts with the current no-collection promise.** The existing privacy page promises no accounts, analytics, trackers, servers receiving user data, or data-collection network calls, so even a minimal account-sync counter is a product/legal posture change ([privacy page](https://github.com/JacobStephens2/cascade/blob/8eef7bdfbcde6ff9923a14097bed3fa38c0f97d5/apps/web/public/privacy/index.html#L130-L159)). Validate UX copy and opt-out behavior before building OAuth/passkey polish.

**Riskiest unknown 4: account deep links across six shells can consume disproportionate time.** Email magic-link is recommended for v1 because passkeys are stronger but require WebAuthn/passkey platform work, and OIDC is correct for third-party identity but unnecessary if Cascade only needs an email-bound account ([W3C WebAuthn](https://www.w3.org/TR/webauthn-3/), [OWASP Authentication Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Authentication_Cheat_Sheet.html)). Validate one robust fallback flow, such as “enter six-digit code shown in email,” before implementing native universal links everywhere.

**Riskiest unknown 5: reset/delete semantics with monotonic counters need a clean epoch story.** A grow-only counter is ideal for idempotent sync, but privacy deletion means an old offline client must not later resurrect deleted totals by PUTing a higher stale counter. Validate an epoch or device-id rotation design in the backend before shipping account deletion.
