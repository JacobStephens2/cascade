# Cascade Architecture Plan: Listening-Time Tracking

## Executive Summary
This document outlines the architecture for introducing a cross-platform listening-time tracking feature to Cascade. Building upon the Clave architecture where `cascade-core` acts as a pure state machine, we track playback time deterministically through the existing `Tick` mechanism. 

The strategy ensures time tracking works entirely offline and asynchronously. A lightweight Rust backend syncs accumulated listening time per device using a CRDT-style strategy (monotonic additive counters), solving the distributed conflict resolution elegantly. The rollout follows Cascade's established conventions, building the core first and incrementally adapting each native shell.

## 1. Where Measurement Lives (Core Changes)

The Cascade core is a pure state machine driven by `Command`s. We will measure listening time by observing the `Tick` command when the state implies audio is actively playing.

### Core Source of Truth

To ensure we measure *actual listening time* and not just "app open time", the rule is simple: if `intent.is_playing()` is true, the `Tick { elapsed_ms }` contributes to the total listening time.

We need to add a few fields to the core state and wire types.

**`Snapshot` additions:**
We need to communicate the tracked time and tracking opt-in state to the UI.

```rust
// snapshot.rs
pub struct Snapshot {
    // ... existing fields ...
    
    // NEW fields for tracking feature
    pub total_listening_time_ms: u64,
    pub is_tracking_enabled: bool,
}
```

**`Command` additions:**
We need commands to toggle the tracking feature, and to reset the local counter once a sync to the backend succeeds.

```rust
// command.rs
pub enum Command {
    // ... existing ...
    
    /// User toggled the opt-in tracking setting.
    SetTrackingEnabled { enabled: bool },
    
    /// Called by the shell when it has successfully persisted an accumulation
    /// of local time (e.g. flushed to its persistent store or synced).
    /// The core drops this amount from its in-memory "unflushed" accumulator.
    AcknowledgeTrackedTime { ms_flushed: u64 },
}
```

**`Effect` additions:**
When accumulated time crosses a threshold (e.g., every 60 seconds), we fire an effect asking the shell to flush the increment to its persistent local storage.

```rust
// effect.rs
pub enum Effect {
    // ... existing ...
    
    /// Instructs the shell to add `delta_ms` to its persistent offline store.
    FlushTrackedTime { delta_ms: u64 },
}
```

**`State` additions:**
The core needs to hold the user's preference and an accumulator of time that hasn't been flushed to the shell yet.

```rust
// state.rs
pub struct State {
    // ... existing ...
    
    /// NEW: Is tracking enabled? Persisted.
    pub is_tracking_enabled: bool,
    
    /// NEW: Time accumulated since last flush. Not persisted in settings JSON;
    /// this is short-lived until the core fires a `FlushTrackedTime` effect.
    pub unflushed_listening_time_ms: u64,
    
    /// NEW: Lifetime total to display on UI. Derived from Settings or injected.
    pub total_listening_time_ms: u64,
}

// In settings.rs, add `is_tracking_enabled` (default true) and `total_listening_time_ms`
// to `PersistedSettings`. Increment `SETTINGS_VERSION`.
```

**Reducer Logic (`reduce`):**
In `Command::Tick { elapsed_ms }`, check `if state.intent.is_playing() && !state.muted && state.is_tracking_enabled`. 
*(Note: Whether to count time while `muted` is true is a product decision. I recommend NOT counting it since they aren't listening).*

If playing, add `elapsed_ms` to `state.unflushed_listening_time_ms` and `state.total_listening_time_ms`. 

If `state.unflushed_listening_time_ms` exceeds a threshold (e.g., 60,000ms), push `Effect::FlushTrackedTime { delta_ms: state.unflushed_listening_time_ms }`, and zero out the unflushed accumulator.

## 2. Local Persistence Per Shell

The core stays pure and does no I/O. The shell receives `Effect::FlushTrackedTime { delta_ms }`. 

Because network connectivity is flaky and devices go offline, each shell must maintain a local queue or accumulator of unsynced time.

*   **Web (PWA):** Use IndexedDB via a simple wrapper like `idb-keyval` or standard localStorage if the data shape is minimal. We just need to store an integer: `unsynced_ms`.
*   **Android (Kotlin):** Use `DataStore` (Preferences DataStore) for a simple atomic `unsynced_ms` counter.
*   **Apple (macOS/iOS/watchOS - SwiftUI):** Use `UserDefaults` (specifically the App Group container if sharing between extensions/watch, though watchOS often needs manual sync via `WCSession`). Store `unsynced_ms`.
*   **Windows (WinUI 3):** Use `ApplicationData.Current.LocalSettings`.

When the shell successfully syncs `unsynced_ms` to the backend, it subtracts the synced amount from its local persistent store and fires `Command::AcknowledgeTrackedTime` to keep the core's total accurate.

## 3. Backend: Account & Sync Service

To centralize data, we need a lightweight backend running on the existing VPS.

**Stack Recommendation:** Rust + Axum + SQLx (Postgres). It's incredibly low footprint (a few MBs of RAM), fast, and aligns with the Cascade author's Rust expertise.

**Auth Recommendation: Email Magic Links via a minimal passkey/JWT flow.** 
OAuth requires managing 3rd party apps (Google, Apple) which breaks the "lightweight self-hosted" vibe. Passkeys are excellent but fallback is complex across 6 platforms. 
Email magic links are stateless, secure, and require only an SMTP server. When a user clicks the link, the backend issues a long-lived JWT stored securely on the device.

**Data Schema (Postgres):**
We want the simplest schema possible.

```sql
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) UNIQUE NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- We store monotonic counters per device
CREATE TABLE device_stats (
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    device_id UUID NOT NULL, -- Generated locally by the shell on first install
    total_ms BIGINT NOT NULL DEFAULT 0,
    last_synced_at TIMESTAMPTZ DEFAULT NOW(),
    PRIMARY KEY (user_id, device_id)
);
```

**API Surface:**
*   `POST /auth/request-link` - Initiates magic link.
*   `POST /auth/verify` - Exchanges token from link for a JWT.
*   `POST /sync/time` - Pushes a time delta and returns the global total.
*   `GET /sync/total` - Fetches global total (for initial app load).

## 4. Sync Protocol & Conflict Resolution

Concurrent multi-device usage is a classic distributed systems problem. If I listen on my Mac for 30 mins and my phone for 20 mins while offline, they shouldn't overwrite each other (LWW is wrong here). 

**Recommendation: CRDT-style State-based Monotonic Counters per Device.**

Instead of syncing "deltas" (which are not idempotent and fail if requests are retried), each device maintains its own **lifetime monotonic counter** of time played on *that specific device*.

**Algorithm:**
1. Device generates a persistent `device_id` (UUID) on install.
2. Device tracks `local_lifetime_ms`.
3. Sync Request: `POST /sync/time { device_id, local_lifetime_ms }`
4. Backend Upsert: 
   ```sql
   INSERT INTO device_stats (user_id, device_id, total_ms) 
   VALUES ($1, $2, $3)
   ON CONFLICT (user_id, device_id) 
   DO UPDATE SET total_ms = GREATEST(device_stats.total_ms, EXCLUDED.total_ms)
   ```
5. Backend Response: Returns the `SUM(total_ms)` for all devices owned by `user_id`.
6. Device updates the core's total: `dispatch(Command::SetGlobalTotal { ms })`

This is perfectly idempotent. If a request times out and the client retries, `GREATEST` ensures the counter only moves forward. The global sum is always eventually consistent.

## 5. Privacy & Data Minimization

Tracking is ON by default. To maintain user trust and defensibility:

*   **Opt-out:** The `is_tracking_enabled` toggle immediately stops the core from accumulating time.
*   **Data Minimization:** We do *not* store timestamps of *when* a user listened. We do *not* store session lengths. The backend only sees a monolithic `total_ms` counter per device. An attacker compromising the database only sees "Device X played 14 hours total," not "User slept from 11 PM to 7 AM."
*   **Account Deletion:** A clear "Delete Account" button in settings that issues a `DELETE` to the users table, cascading to `device_stats`.

## 6. Phased Build Order

Follow the Cascade convention: one atomic commit for the core, then shells.

**Phase 1: Core & API (Local verification)**
1.  **Backend:** Deploy the Rust/Axum service and Postgres schema to the VPS.
2.  **`cascade-core`:** Add the `Snapshot` fields, commands, and `Tick` reducer logic. 
3.  **Core Tests:** Add unit tests verifying `unflushed_ms` accumulates during `Tick` when playing, triggers `FlushTrackedTime`, and stops when paused or tracking disabled.

**Phase 2: Verifiable Shells (Linux Dev Host)**
4.  **Web (PWA):** Implement IndexedDB storage for device ID and sync logic. Update React UI to show total time and toggle in Settings.
5.  **Android:** Implement DataStore storage and WorkManager for background syncing. Update Compose UI.

**Phase 3: Apple Shells (CI compiled, Mac required)**
6.  **macOS:** Update UniFFI bindings. Implement AppStorage/UserDefaults. Update SwiftUI MenuBar app.
7.  **iOS:** Share logic with macOS. Use BGTaskScheduler for background sync if needed.
8.  **watchOS:** WatchConnectivity syncs the local delta to the iOS companion app, which handles the network request.

**Phase 4: Windows**
9.  **Windows:** Update C ABI. Implement local settings persistence. Update WinUI 3 views.

## Riskiest Unknowns

*   **watchOS offline sync:** Apple Watch can drift significantly from the iPhone. Ensuring the watch accumulates its own monotonic counter and reliably flushes it via `WCSession` when the phone reconnects requires careful lifecycle management.
*   **Background sync restrictions:** iOS and Android aggressively kill apps. We must ensure the `FlushTrackedTime` effect is persisted *synchronously* to local disk before the app is suspended. Network sync can wait, but local persistence cannot.
*   **Muted state tracking:** Users might mute the app to use a timer but not listen to audio. Deciding if this counts as "listening time" is a UX edge case that needs a definitive call (recommended: don't count it).