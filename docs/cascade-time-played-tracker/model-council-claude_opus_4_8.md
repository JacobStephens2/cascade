# Cascade — Cross-Platform Listening-Time Architecture

**Author:** Claude Opus 4.8 (model council member)
**Repo grounded:** [`JacobStephens2/cascade`](https://github.com/JacobStephens2/cascade) — read `crates/cascade-core/`, all three bindings (`cascade-wasm`, `cascade-uniffi` UniFFI + C ABI), the six shells, CI workflows, and `docs/architecture-brief-web-and-android.md`.
**Depth:** Deep dive. Every recommendation is tied to a code convention actually observed in the repo.

---

## Executive Summary

Cascade is a textbook "Clave kata": one pure Rust reducer (`dispatch(Command) -> Update { snapshot, effects }`) owns *all* intent and state, six shells own *all* side effects, and one JSON wire shape crosses three bindings (wasm-bindgen, UniFFI, hand-rolled C ABI). The reducer never touches a clock — time enters via `Command::Tick { elapsed_ms }` — and persistence is delegated entirely to shells via `Effect::PersistSettings { json }`. The feature must respect every one of these invariants.

My core recommendation, distilled:

1. **Measurement lives in the core, as a pure accumulator on the existing `Tick` path.** The reducer already knows `state.intent.is_playing()` and `state.muted` on every tick. I add a single accumulator (`lifetime`) that increments only when audio is *actually audible* (playing AND not muted), plus a small set of new Commands/Effects/Snapshot fields. The core stays 100% pure: it computes *how much* to persist and *when*, but the shell does the I/O. This rides the exact same `Tick` → reducer → effect → shell-persist loop that already drives the timers.

2. **The device is the CRDT replica.** Total lifetime listening is a **G-Counter** (grow-only counter): each device owns one slot holding *its own* monotonic accrued milliseconds; the global total is the **sum of all device slots**; merge is **per-slot `max`**. This is the canonical CRDT for "count upward-only events across replicas that can't always communicate" ([CRDT field guide, Ian Duncan](https://iankduncan.com/engineering/2025-11-27-crdt-dictionary/); [Wikipedia: CRDT](https://en.wikipedia.org/wiki/Conflict-free_replicated_data_type)). It makes the backend a *dumb sum-of-maxes store* with zero conflict logic, and it's correct under arbitrary offline windows, duplicate pushes, and concurrent multi-device playback — exactly the hard cases the task names.

3. **Local persistence reuses each shell's existing settings sink**, but in a *separate* blob. The web shell already demonstrates the pattern: settings (`cascade.settings.v1`, core-owned) live separately from live session (`cascade.session.v1`, shell-owned). I add a third blob, `cascade.listening.v1`, written from a new `Effect::PersistListening { json }`. Android uses DataStore, Windows/macOS/iOS use a file in app-support, watchOS forwards to iPhone over WatchConnectivity (it never persists or syncs on its own).

4. **Backend: a single small Axum + SQLx + Postgres service** on a subdomain (`api.cascade.stephens.page`), Apache-reverse-proxied, systemd-managed, using the **Postgres instance already on the data volume**. Auth: **email magic-link** (recommended over OAuth and passkeys for this app — rationale in §3). Account model: **anonymous device-id first, optional email upgrade**. Three endpoints. ~30 MB binary, a few MB of data for years of users.

5. **Privacy by construction.** The only thing that ever leaves a device is a single integer (accrued audible ms) keyed to an opaque device UUID, plus an optional email if the user links an account. No timestamps of *when* you listened, no session logs, no IP retention beyond rate-limiting. Opt-out (default ON, per the spec) freezes the accumulator and stops all network egress; account deletion is a single cascading `DELETE`. The defensibility story is short precisely because the data is minimal.

6. **Build order follows the repo's "core first, then one atomic commit per shell" rule.** Core + both bindings land first (runtime-verifiable via `cargo test` and the web shell on Linux). Then web and Android (the two shells *runtime-verified locally* on the Linux dev host). Then the backend. Then Windows (CI artifact only) and the three Apple targets (CI compile only). watchOS is last and trivial because it's a thin remote.

**Riskiest unknown:** offline clock integrity. Because the core trusts `elapsed_ms` deltas the shell computes, a device with a jumpy or attacker-controlled clock can inflate its own G-Counter slot. This is a *measurement* risk, not a *merge* risk (the merge is provably correct). Mitigations in the final section.

---

## Question 1 — Where measurement lives (core API additions)

### Principle: accrue on the Tick path, stay pure

The reducer is the only place that mutates state, and it already receives wall-clock deltas via `Command::Tick { elapsed_ms }`. It already inspects play state (`state.intent.is_playing()`) and mute (`state.muted`, with `state.output_volume()` returning 0 while muted). So the *measurement* — "how many audible milliseconds elapsed this tick" — is already computable inside `reduce()` with no new inputs and no clock access. This is the key insight: **we add zero new dependencies to the pure core; we only add state that aggregates a value the reducer can already see.**

"Actual listening time (audio playing), not app-open time" is therefore defined precisely as:

> accrue `elapsed_ms` into the lifetime counter **iff** `state.intent.is_playing() && !state.muted`.

Whether muted time counts is a genuine product decision. I **recommend counting only audible time** (exclude muted), because the spec says "measure actual listening time (audio playing)" and a muted stream is not listening. This is a one-line predicate and can be flipped later; I expose it as a named helper (`is_audible()`) so the choice is explicit and testable.

A subtlety the existing code forces us to handle: the `Tick` handler currently does nothing unless `state.active_timer` is `Some`. Listening accrual must happen **on every tick while audible, regardless of whether a timer is running.** That means shells must tick whenever audio is playing, not only when a timer is active. The web shell today gates its tick loop on `isTimerActive` (see `useCascade.ts`); that gate must widen to `isTimerActive || isPlaying`. This is a shell change, documented per-platform in §6.

### New core types (Rust pseudocode diff)

```rust
// ── crates/cascade-core/src/listening.rs (NEW module) ─────────────────────
//! Lifetime listening accumulator. Pure: counts audible milliseconds from the
//! Tick path. Owns the "how much to flush / when" logic but never does I/O —
//! it emits a value the shell persists, mirroring PersistSettings.

use serde::{Deserialize, Serialize};

/// Flush to the shell once we've accrued at least this much un-persisted time.
/// Keeps PersistListening effects rare (≈ one write/minute while playing)
/// instead of one per 250ms tick.
pub const FLUSH_THRESHOLD_MS: u64 = 60_000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Listening {
    /// Monotonic total audible ms ever accrued ON THIS DEVICE. This is the
    /// device's G-Counter slot value. Only ever increases. Source of truth
    /// for the lifetime number the UI shows and the backend stores.
    pub device_total_ms: u64,
    /// Ms accrued since the last successful flush to the shell's store. Folded
    /// into device_total_ms immediately; this field only decides WHEN to emit
    /// a PersistListening effect.
    pub unflushed_ms: u64,
    /// Whether tracking is enabled. Mirrors the account setting; ON by default.
    /// When false, ticks do not accrue and nothing is emitted.
    pub enabled: bool,
}

impl Default for Listening {
    fn default() -> Self {
        Self { device_total_ms: 0, unflushed_ms: 0, enabled: true } // ON by default
    }
}

impl Listening {
    /// Accrue audible time. Returns true if a flush threshold was crossed.
    pub fn accrue(&mut self, elapsed_ms: u64) -> bool {
        if !self.enabled { return false; }
        self.device_total_ms = self.device_total_ms.saturating_add(elapsed_ms);
        self.unflushed_ms = self.unflushed_ms.saturating_add(elapsed_ms);
        self.unflushed_ms >= FLUSH_THRESHOLD_MS
    }
    pub fn mark_flushed(&mut self) { self.unflushed_ms = 0; }
}
```

```rust
// ── command.rs: add three variants (camelCase wire shape preserved) ───────
pub enum Command {
    // ... all existing variants unchanged ...

    /// User toggled the "track listening time" account setting. ON by default.
    /// Turning OFF freezes accrual and stops network egress; does NOT erase the
    /// already-accrued local total (deletion is a separate explicit action).
    SetListeningTrackingEnabled { enabled: bool },

    /// Shell reports the locally persisted lifetime total at startup, so the
    /// core can restore device_total_ms without doing I/O itself. Mirrors how
    /// `Core::restore` rehydrates settings — but listening is a separate blob.
    RestoreListening { device_total_ms: u64, enabled: bool },

    /// Shell reports the authoritative merged total after a sync round-trip
    /// (max(local_slot, server_slot) summed across devices). The core adopts it
    /// for display only; it never lowers device_total_ms below its own slot.
    ApplySyncedTotal { merged_total_ms: u64 },
}
```

```rust
// ── effect.rs: add two variants ───────────────────────────────────────────
pub enum Effect {
    // ... existing variants unchanged ...

    /// Persist the lifetime listening blob. Separate from PersistSettings so
    /// the high-frequency listening writes don't churn the settings JSON and
    /// so opt-out / deletion can target one file. Shell decides where.
    PersistListening { json: String },

    /// Ask the shell to push the device's current slot to the backend. The
    /// core does NO network I/O; it just signals "there's enough to sync".
    /// Emitted when unflushed crosses threshold AND tracking is enabled.
    RequestSync { device_total_ms: u64 },
}
```

```rust
// ── snapshot.rs: extend Snapshot so UIs render lifetime with no extra call ─
pub struct Snapshot {
    // ... existing fields ...
    pub listening: ListeningSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListeningSnapshot {
    pub tracking_enabled: bool,
    /// Best-known lifetime total across all devices: max(local slot, last
    /// merged total from server). Pre-formatted label provided so every UI
    /// renders the identical string, exactly like timer.remaining_label.
    pub total_ms: u64,
    pub total_label: String,   // e.g. "1,284 hours" via a format_listening() helper
}
```

```rust
// ── state.rs: reducer integration. Pure; no new dependencies. ─────────────
pub struct State {
    // ... existing fields ...
    pub listening: Listening,
    /// Highest merged total the server has confirmed. Display only; never
    /// used to lower the local G-Counter slot. None until first sync.
    pub synced_total_ms: Option<u64>,
}

/// True when audio is genuinely audible (playing AND not muted).
impl State {
    pub fn is_audible(&self) -> bool { self.intent.is_playing() && !self.muted }
    /// The lifetime number to SHOW: never less than this device's own slot.
    pub fn display_total_ms(&self) -> u64 {
        self.synced_total_ms.unwrap_or(0).max(self.listening.device_total_ms)
    }
}

pub fn reduce(state: &mut State, command: Command, effects: &mut Vec<Effect>) {
    state.timer_just_completed = None;
    match command {
        // ... existing arms unchanged ...

        Command::Tick { elapsed_ms } => {
            // EXISTING timer logic stays exactly as-is.
            if let Some(t) = state.active_timer.as_mut() { /* ...unchanged... */ }

            // NEW: accrue audible listening time on the same tick.
            if state.is_audible() {
                let crossed = state.listening.accrue(elapsed_ms);
                if crossed {
                    // Persist locally, then ask the shell to sync. Two effects,
                    // both shell-executed; the core does neither itself.
                    push_persist_listening(state, effects);
                    if state.listening.enabled {
                        effects.push(Effect::RequestSync {
                            device_total_ms: state.listening.device_total_ms,
                        });
                    }
                }
            }
        }

        Command::SetListeningTrackingEnabled { enabled } => {
            state.listening.enabled = enabled;
            push_persist_listening(state, effects); // record the preference
        }
        Command::RestoreListening { device_total_ms, enabled } => {
            // Adopt the larger value so we never regress the monotonic slot.
            state.listening.device_total_ms =
                state.listening.device_total_ms.max(device_total_ms);
            state.listening.enabled = enabled;
        }
        Command::ApplySyncedTotal { merged_total_ms } => {
            state.synced_total_ms = Some(merged_total_ms); // display only
        }
    }
}

fn push_persist_listening(state: &mut State, effects: &mut Vec<Effect>) {
    if let Ok(json) = serde_json::to_string(&state.listening) {
        effects.push(Effect::PersistListening { json });
    }
    state.listening.mark_flushed();
}
```

Note the deliberate stop-on-pause behavior: pausing (or muting) simply means `is_audible()` returns false, so subsequent ticks don't accrue — no special pause handling needed. The existing property tests (`stopwatch_accumulates_and_never_ends`, `mute_keeps_session_and_round_trips`) give us a template; I'd add `listening_accrues_only_while_audible` and `listening_monotonic_under_arbitrary_commands` to the proptest suite, asserting `device_total_ms` never decreases across any command sequence.

### Crossing the three bindings

The beauty of this design is that **no binding changes shape.** All three bridges already do the same thing: `dispatch(command_json) -> update_json`, JSON in, JSON out, with serde's `rename_all = "camelCase"` as the single source of truth (the wasm and uniffi crates both document this explicitly). Because the new Commands/Effects/Snapshot fields are plain serde enums/structs with camelCase fields, they cross **for free**:

- **wasm-bindgen (web):** `CascadeCore::dispatch` already takes a JSON string and returns a JSON `Update`. The TS `Command`/`Effect`/`Snapshot` types in `apps/web/src/core/types.ts` gain the three new command variants, two effect variants, and the `listening` snapshot field. No Rust binding code changes.
- **UniFFI (Kotlin/Swift):** `CascadeBridge::dispatch` is identical — JSON string in/out. The Kotlin `Dto.kt` and Swift DTOs get the same new fields. No `.udl`/proc-macro changes because the FFI surface (one `dispatch` method) is unchanged; only the JSON payloads grow.
- **C ABI + P/Invoke (Windows):** `cascade_dispatch` is a `*const c_char` → `*mut c_char` JSON pump. The C# `Dto.cs` gets the new fields. The hand-rolled ABI doesn't change at all.

This is the entire reason the repo chose "JSON across the boundary" over typed FFI (the `cascade-wasm` and `cascade-uniffi` module docs both call this out): schema evolution costs one serde field, replicated as DTO fields in three languages, with zero binding-layer churn. I'd add a binding round-trip test in each (the repo already has `dispatch_play_emits_start_playback` and `c_abi_round_trip`) asserting `{"type":"setListeningTrackingEnabled","enabled":false}` parses and that an `Update` carries `listening.totalMs`.

---

## Question 2 — Local persistence per shell

The core never does I/O; it emits `Effect::PersistListening { json }` and the shell stores the opaque blob, exactly as it already does for `Effect::PersistSettings`. **The blob is stored separately from settings**, following the precedent the web shell already set (settings vs. session are two distinct localStorage keys in `useCascade.ts`). Separation matters for three reasons: (a) listening writes are frequent (~1/min while playing) and shouldn't churn the settings file; (b) opt-out/deletion can target exactly one artifact; (c) the device UUID for sync lives here too, isolated from user-facing settings.

At startup each shell reads the listening blob and replays `Command::RestoreListening { device_total_ms, enabled }` (analogous to how the web shell replays session state after `Core::restore`). The device UUID is generated once on first run and stored in the same blob.

| Platform | Store for the listening blob | Mechanism | Notes |
|---|---|---|---|
| **Web (PWA)** | `localStorage["cascade.listening.v1"]` | Same path as `persistSettings` in `runEffects` | Add a `case "persistListening"`. Device UUID via `crypto.randomUUID()`. IndexedDB optional later; localStorage is fine for one integer + UUID. |
| **Android (Kotlin)** | Jetpack DataStore, new key `listening_v1` | Extend `SettingsStore` (or a sibling `ListeningStore`) — same DataStore pattern | DataStore writes are async/transactional; the JSON shape stays opaque to Kotlin, as the existing store comment notes. |
| **Windows (C#/WinUI)** | `%LOCALAPPDATA%\Cascade\listening.json` | Mirror `SettingsStore.cs` (`ReadSafely`/`WriteSafely`) | Best-effort, never block the dispatch loop — same contract the settings store already documents. |
| **macOS (SwiftUI)** | `~/Library/Application Support/Cascade/listening.json` | File write, same as settings | App-support dir per the existing macOS shell convention. |
| **iOS (SwiftUI)** | App-container `listening.json` (app support) | File write | iOS owns truth for the paired watch. |
| **watchOS** | **None** | Forwards over WatchConnectivity | watchOS is a thin remote (per `docs/watchos-architecture.md` recommendation); it sends commands to iPhone and renders the snapshot. It does **not** persist or sync independently — otherwise the same human's wrist + phone would double-count. Watch-originated audible time is attributed to the **iPhone's** device slot. |

**Why this keeps the core as single source of truth without doing I/O:** the core holds `device_total_ms` in memory and decides exactly when to emit a persist/sync effect. The shell is a dumb sink that hands the same number back at next launch via `RestoreListening`. The `max()` in the `RestoreListening` handler guarantees that even a stale or partially-written blob can never *lower* the in-memory slot — a cheap monotonicity guard that matters because some shells (Windows, macOS) do best-effort writes that can be interrupted.

---

## Question 3 — Backend: account + sync service

### Recommendation summary

- **Service:** one **Rust + Axum + SQLx** binary on `api.cascade.stephens.page`, behind Apache `ProxyPass`, managed by systemd, talking to the **existing Postgres** on the data volume. ~30 MB static binary, single-digit MB RAM at idle, trivial disk. This matches the "smallest lightweight low-footprint service on a subdomain" directive and reuses the Rust toolchain already in the repo (so the core's serde types can even be shared as a crate dependency if desired). Axum + SQLx + Postgres is the well-trodden minimal path ([DEV: Rust CRUD with Axum/SQLx/Postgres](https://dev.to/francescoxx/rust-crud-rest-api-using-axum-sqlx-postgres-docker-and-docker-compose-152a)).
- **Account model:** **anonymous device-id first; optional email upgrade.** A device is usable for sync the instant it generates a UUID — no signup wall — which honors "Cascade works fully without account; account ONLY centralizes data." Linking an email *merges devices under one account*; that's the only thing an account buys.
- **Auth:** **email magic-link.** Recommended over OAuth and passkeys (rationale below).

### Why magic-link over OAuth and passkeys

| Method | Fit for Cascade | Verdict |
|---|---|---|
| **Email magic-link** | No passwords to store/leak; one DB table of short-lived hashed tokens; works on every one of six platforms via a tapped link / deep link; no third-party dependency; the *only* PII is an email the user volunteers. Best practices are simple and well-documented: short expiry (~15 min), one-time use, CSPRNG token, store only the hash ([Postmark magic-link guide](https://postmarkapp.com/blog/magic-links); [Deepak Gupta on magic-link security](https://guptadeepak.com/mastering-magic-link-security-a-deep-dive-for-developers/)). | **RECOMMENDED** |
| **OAuth (Google/Apple)** | Adds a third-party identity dependency, per-platform SDK integration across six shells, and pulls in *more* identity data than a white-noise app should hold. Apple would also mandate "Sign in with Apple" if any other social login ships, increasing scope. Overkill for "sum one integer across my devices." | Rejected — disproportionate |
| **Passkeys (WebAuthn)** | Phishing-resistant and elegant, but cross-device/cross-ecosystem passkey sync and the six-shell platform-authenticator integration (especially Windows/WinUI and watchOS) is materially more engineering than the payoff for an opt-in listening counter ([Authgear: magic links vs passkeys vs OTP](https://www.authgear.com/post/passwordless-authentication-magic-links-passkeys-otp/)). Revisit if Cascade ever holds higher-value data. | Rejected for v1 — premature |

Magic-link is the smallest thing that meets the requirement, and "smallest thing" is the stated design value.

### API surface (4 routes, all JSON)

```
POST /v1/devices/sync          # the hot path — anonymous, device-id authed
  Auth: header  X-Device-Id: <uuid>   + optional  Authorization: Bearer <session-jwt>
  Body: { "device_total_ms": 4823000, "client_seq": 17 }   # idempotency key
  Resp: { "merged_total_ms": 9981000 }                      # sum of maxes across the account/device

POST /v1/auth/request          # send magic link
  Body: { "email": "a@b.com", "device_id": "<uuid>" }
  Resp: 202 (always, to avoid email enumeration)

GET  /v1/auth/verify?token=... # consume one-time link, mint session, claim device
  Resp: { "session_jwt": "...", "account_id": "..." }  # links X-Device-Id → account

DELETE /v1/account             # full deletion (account + all its device slots)
  Auth: Bearer <session-jwt>
  Resp: 204
```

A device that never links an email still syncs against `/v1/devices/sync` keyed purely by its UUID; in that case "the account" is implicitly the single device, and `merged_total_ms` just echoes its own slot. Linking email later associates the device row with an `account_id`, and the sum-of-maxes now spans every device under that account. This is what makes the account *purely additive*: it changes nothing about local behavior, it only widens the set summed in the merge.

### Minimal Postgres schema

```sql
-- One row per account (only exists once an email is linked).
CREATE TABLE account (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email       TEXT UNIQUE NOT NULL,          -- only PII in the system
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- One row per device = one G-Counter slot. The whole CRDT lives here.
CREATE TABLE device (
    id              UUID PRIMARY KEY,           -- the client-generated device UUID
    account_id      UUID REFERENCES account(id) ON DELETE CASCADE,  -- NULL until linked
    -- The monotonic slot value. Server NEVER lowers it (see sync algo).
    total_ms        BIGINT NOT NULL DEFAULT 0 CHECK (total_ms >= 0),
    last_client_seq BIGINT NOT NULL DEFAULT 0,  -- idempotency / replay guard
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX device_account_idx ON device(account_id);

-- Short-lived, single-use, hashed magic-link tokens.
CREATE TABLE magic_token (
    token_hash  BYTEA PRIMARY KEY,              -- SHA-256 of the random token; raw token never stored
    email       TEXT NOT NULL,
    device_id   UUID NOT NULL,                  -- device to claim on verify
    expires_at  TIMESTAMPTZ NOT NULL,           -- now() + 15 min
    consumed_at TIMESTAMPTZ                      -- set on first use; one-time
);
```

That is the *entire* schema: three tables, one integer of real data per device, one email per account. For thousands of users with years of history this is a few MB — the "disk not binding" note holds with enormous margin. No event log, no per-session rows, no timestamps of listening activity (see §5).

---

## Question 4 — Sync protocol & conflict resolution

### Recommendation: G-Counter (per-device monotonic slots, sum of maxes)

The task asks us to choose between "monotonic per-device counters / CRDT-style sums" and "last-writer-wins." **LWW is wrong here and would silently lose data.** Consider two devices listening concurrently offline: device A accrues 2 h, device B accrues 1 h. With LWW, whichever syncs *last* overwrites the total, and an hour vanishes. The whole point of "track total across every platform" is that **concurrent listening must add, not overwrite.**

The correct primitive is the **G-Counter** (grow-only counter), the canonical CRDT for upward-only distributed counts: each replica owns one slot, the value is the **sum of all slots**, and merge is **per-slot `max`** — commutative, associative, idempotent, and guaranteed to converge without losing increments ([Ian Duncan CRDT field guide](https://iankduncan.com/engineering/2025-11-27-crdt-dictionary/); [Wikipedia: CRDT](https://en.wikipedia.org/wiki/Conflict-free_replicated_data_type); [The Third Bit: CRDTs](https://third-bit.com/dsdx/crdt/)). Mapping to Cascade: **device = replica, slot = `device.total_ms`, total = SUM over the account's devices.** A PN-counter is unnecessary because listening time never decreases (the spec has no "subtract time" operation; deletion is a separate destructive action, not a decrement).

Why this is the right call for *this* system specifically:
- **Concurrency is correct by construction.** Two devices playing at once each grow their own slot; the sum is exactly their combined audible time. No coordination, no locks, no conflict resolution code on the server.
- **Idempotency is free.** Re-pushing the same slot value is a no-op under `max`. Duplicate/retried sync requests can't double-count. (We still carry a `client_seq` as a belt-and-suspenders replay guard and to short-circuit no-op writes.)
- **Offline windows don't matter.** A device offline for a month just pushes a bigger slot when it reconnects; `max` absorbs it. Sync order across devices is irrelevant to the final total.
- **The server is dumb.** It stores per-device maxes and returns a sum. No merge logic can be "wrong."

### Cadence

- **Push:** triggered by `Effect::RequestSync` (emitted by the core when un-flushed audible time crosses `FLUSH_THRESHOLD_MS` = 60 s) **and** on a coalescing timer (at most one push every ~5 min while playing) **and** opportunistically on app foreground/background transitions and clean shutdown. The shell debounces; the core never times this (no clock in core).
- **Pull:** piggybacked — every `/v1/devices/sync` response *returns* the freshly merged total, so a push doubles as a pull. The shell feeds it back via `Command::ApplySyncedTotal { merged_total_ms }`. Extra standalone pulls only on app launch.
- **Offline:** the shell holds the latest slot in the local blob and retries on reconnect. Nothing is lost because the slot is monotonic and local-authoritative.

### Server-side sync algorithm (pseudocode)

```rust
// POST /v1/devices/sync  — the only hot endpoint. Pure G-Counter merge.
async fn sync(device_id: Uuid, account: Option<AccountId>,
              body: SyncReq, db: &Pool) -> SyncResp {
    let mut tx = db.begin().await?;

    // 1. Upsert this device's slot with MONOTONIC MAX. The server never lowers
    //    a slot — that is the G-Counter invariant and the entire conflict rule.
    //    client_seq guards against out-of-order/replayed requests.
    sqlx::query!(
        "INSERT INTO device (id, account_id, total_ms, last_client_seq)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (id) DO UPDATE
            SET total_ms        = GREATEST(device.total_ms, EXCLUDED.total_ms),
                last_client_seq = GREATEST(device.last_client_seq, EXCLUDED.last_client_seq),
                updated_at      = now()
            WHERE EXCLUDED.last_client_seq > device.last_client_seq
               OR EXCLUDED.total_ms       > device.total_ms",
        device_id, account, body.device_total_ms as i64, body.client_seq as i64
    ).execute(&mut *tx).await?;

    // 2. The merged total = SUM of maxes across all devices in this account.
    //    For an unlinked device, "the account" is just itself.
    let merged: i64 = match account {
        Some(acc) => sqlx::query_scalar!(
            "SELECT COALESCE(SUM(total_ms),0) FROM device WHERE account_id = $1", acc
        ).fetch_one(&mut *tx).await?,
        None => sqlx::query_scalar!(
            "SELECT total_ms FROM device WHERE id = $1", device_id
        ).fetch_one(&mut *tx).await?,
    };

    tx.commit().await?;
    SyncResp { merged_total_ms: merged as u64 }
}
```

```text
// Client-side (shell) view of the loop — no clock logic in core:
on Effect::RequestSync { device_total_ms }:
    enqueue push { device_id, device_total_ms, client_seq: ++local_seq }
    (debounced; retried on failure with backoff; survives offline)
on push success { merged_total_ms }:
    dispatch Command::ApplySyncedTotal { merged_total_ms }
        -> Snapshot.listening.total_ms = max(merged, local_slot)
```

`GREATEST` in the upsert is the literal per-slot `max` of the G-Counter; `SUM` is the literal value query. The merge cannot lose an increment regardless of request order, duplicates, or concurrent multi-device pushes — which is precisely the property LWW lacks.

---

## Question 5 — Privacy & data minimization

Tracking is **ON by default**, so the design must be *defensible by minimization* rather than by consent friction. The strategy: make the data so small and so dull that there is almost nothing to defend.

**What is stored, end to end:**
- *On device:* one integer (`device_total_ms`), one boolean (`enabled`), one opaque device UUID. No history, no timestamps.
- *On the server:* per device, one `BIGINT` and a sequence number; per account (only if the user links email), one email address. **No record of *when* anyone listened, no session boundaries, no IP logs beyond ephemeral rate-limiting, no analytics.** The G-Counter design actively *prevents* a timeline from existing — the server only ever sees a running total, never deltas tagged with time.

This is the strongest privacy property of the architecture and a direct consequence of choosing a sum-of-counters over an event log: a counter is not a diary. There is no table that could answer "what time did this person fall asleep" because no such data is ever transmitted.

**Opt-out semantics (the default-on defensibility):**
- `Command::SetListeningTrackingEnabled { enabled: false }` immediately stops accrual (ticks no longer fold into the counter) and suppresses `Effect::RequestSync`, so **network egress halts**. The local total is frozen, not erased — turning tracking back on resumes from where it stopped, which is the least surprising behavior.
- Because it's on by default, the setting must be **discoverable and one-tap**, and first-run UX should surface "Cascade counts your listening time across devices — you can turn this off in Settings." That disclosure plus a frictionless toggle is the consent posture that makes default-on defensible for a metric this benign.

**Account deletion:** `DELETE /v1/account` cascades (`ON DELETE CASCADE` on `device.account_id`) removing the account row, its email, and all device slots in one transaction — no soft-delete, no tombstones. A device that wants purely local erasure deletes its local listening blob (a shell action) and may call sync with a fresh UUID; the orphaned old slot ages out via a periodic sweep (e.g. delete unlinked devices untouched for N months). Email is the only deletable PII and it's gone immediately.

**Defensibility summary:** opt-in-by-default is acceptable here because (a) the data is a single aggregate integer with no temporal resolution, (b) it never leaves the device unless tracking is on, (c) the user can disable it in one tap and disclosure happens at first run, and (d) deletion is total and immediate. There is no profiling, no third party, and no behavioral timeline. Magic-link auth means the only stored identifier a user didn't auto-generate is an email they explicitly typed.

---

## Question 6 — Phased build order

The repo's convention (README + the per-platform docs) is **core first, then one atomic commit per shell**, with two shells *runtime-verified on the Linux dev host* (web, Android) and the rest *only compiled in CI* (Windows via `windows.yml`, the three Apple targets via `apple.yml` on `macos-latest`). The build order mirrors that exactly, front-loading everything that's locally verifiable.

| Phase | Scope | Verification available | Commit shape |
|---|---|---|---|
| **0 — Core** | New `listening.rs`, 3 Commands, 2 Effects, `ListeningSnapshot`, reducer accrual on the Tick path, `format_listening` helper. Add proptests: monotonicity, audible-only accrual, idempotent restore. | **`cargo test` on Linux** — fully runtime-verified. This is where correctness is proven. | One core commit. Nothing ships until proptests are green. |
| **1 — Bindings** | wasm-bindgen + UniFFI + C ABI: **no signature changes** (JSON pump unchanged). Add binding round-trip tests for the new JSON shapes. Regenerate Kotlin/Swift DTOs; update TS `types.ts`, `Dto.kt`, `Dto.cs`. | `cargo test` for the uniffi C-ABI round-trip; `wasm-pack build` on Linux. Runtime-verified. | One commit. |
| **2 — Web** | `case "persistListening"` + device-UUID gen in `runEffects`; widen the tick gate to `isTimerActive \|\| isPlaying`; add `cascade.listening.v1` blob + `RestoreListening` replay; sync client (fetch with retry/debounce); lifetime readout UI + tracking toggle. | **Runtime-verified locally** (`npm run dev`) and on the live PWA at cascade.stephens.page. First end-to-end proof of the whole feature. | One atomic web commit. |
| **3 — Android** | `ListeningStore` (DataStore); widen tick scheduling to play state; sync client (OkHttp/Ktor) with WorkManager retry; UI toggle + readout. | **Runtime-verified locally** (`./gradlew :app:assembleDebug` + emulator on the Linux host). | One atomic Android commit. |
| **4 — Backend** | Axum + SQLx service: 4 routes, 3-table schema, magic-link email (SMTP), systemd unit, Apache vhost for `api.cascade.stephens.page`, migrations. | Runtime-verified locally + on the VPS against the existing Postgres. Integration test the sum-of-maxes merge. | Separate service (own deploy dir, mirroring `deploy/`). |
| **5 — Windows** | `ListeningStore.cs` (mirror `SettingsStore.cs`); P/Invoke unchanged; sync via `HttpClient`; tick widen; UI. | **CI only** (`windows.yml` artifact) — no local Windows host. Verify the C# compiles and the DTOs match. | One atomic Windows commit; rely on CI badge. |
| **6 — macOS / iOS** | App-support listening file; sync via `URLSession`; tick widen; SwiftUI toggle + readout. | **CI only** (`apple.yml` on `macos-latest`, unsigned compile). | One commit per shell (or one apple commit), CI-gated. |
| **7 — watchOS** | Thinnest: forward the tracking toggle and render `listening.totalLabel` from the snapshot over WatchConnectivity. **No local persistence or sync** — iPhone owns the slot. | **CI compile only**. | Smallest commit; trivial because it's a remote. |

The ordering is deliberate: every bit of *logic* (the accrual math, the merge) is proven in Phase 0–1 by `cargo test` and again end-to-end on web (Phase 2) before a single CI-only platform is touched. By the time Windows/Apple commits land, the only thing those shells contribute is plumbing the same JSON to a platform store and an HTTP call — the part CI can validate by compilation. This is exactly why the Clave kata keeps logic in the core: the CI-only platforms inherit correctness instead of re-implementing it.

---

## Riskiest unknowns (validate first)

1. **Client clock integrity / inflation (highest risk).** The core trusts `elapsed_ms` deltas computed by the shell from wall-clock reads (`DateTimeOffset.UtcNow`, `performance.now()`, etc.). A device with a fast/jumpy clock — or a malicious user — can inflate its own G-Counter slot, and the server's monotonic `max` will faithfully preserve the inflated value. **The merge is provably correct; the *input* is the soft spot.** *Validate first:* (a) clamp per-tick `elapsed_ms` in the core to a sane ceiling (e.g. ≤ 5 s, since shells tick at 250 ms — a delta larger than that is a sleep/wake gap, not listening, and arguably shouldn't count anyway); (b) server-side, reject slot *jumps* that exceed wall-clock-since-last-sync by a margin. Decide explicitly whether a backgrounded-but-playing app should keep accruing during OS suspension (it should, for true listening time) versus a laptop that slept (it shouldn't) — these look identical to the core and must be disambiguated in the shell.

2. **Tick coverage while playing without a timer.** Today every shell only ticks while a *timer* runs (web `useCascade.ts` gates on `isTimerActive`; Windows `TickScheduler` is started/stopped around timers). Listening accrual requires ticking whenever *audio is audible*. *Validate first:* on each platform, confirm the tick loop runs during background playback (Android `MediaSessionService`, iOS `AVAudioSession`, macOS, watch) — if the OS throttles timers in background, accrued time will undercount. This is the single most likely source of "my numbers are too low" bugs and must be tested on real background-audio sessions, not just foreground.

3. **Device-slot proliferation & re-install identity.** A fresh install = a new UUID = a new slot. Reinstalling repeatedly (or clearing storage) creates orphan slots whose sum still counts (good for not losing time) but inflates device counts and can double-count if the same physical device's old slot is also summed. *Validate first:* decide the orphan-sweep policy and whether linked accounts should *merge* a replaced device's slot into its successor, or just let both sum.

4. **Magic-link deliverability across six platforms / deep-linking.** A tapped link must return to the *right* app (PWA, Android intent filter, iOS/macOS universal link, Windows protocol handler). *Validate first:* the verify → session → device-claim round trip on each platform's link-handling story, plus SMTP deliverability from the VPS (SPF/DKIM) so links don't land in spam. This is more integration surface than the auth logic itself.

5. **Watch/phone double-counting.** Attributing watch-originated audible time to the iPhone slot (the recommended thin-remote model) is correct only if the watch never *also* runs its own accrual. *Validate first:* confirm watchOS sends commands but never persists/syncs a slot, and that standalone watch playback (if ever added in a future phase) is explicitly assigned the phone's slot or its own — not silently both.

6. **`u64` ms vs Postgres `BIGINT` (i64) range.** Listening ms fits in `i64` for ~292 million years, so no practical overflow — but the core uses `u64` and the DB uses signed `BIGINT`; keep the `CHECK (total_ms >= 0)` and saturating arithmetic (already the core's habit, e.g. `saturating_add` in `timer.rs`) to avoid any wrap or sign surprise.
