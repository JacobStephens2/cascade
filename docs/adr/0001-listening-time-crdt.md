# ADR 0001 — Cross-platform listening-time tracking: a pure-core G-Counter with data-minimizing sync

- **Status:** Accepted; decisions 5 and 6 amended 2026-09-27 (issue #18); decision 5 amended again 2026-10-03 (issue #31)
- **Date:** 2026-06-05
- **Context:** Cascade is one headless Rust core (`cascade-core`) driving six native shells (web, Android, macOS, Windows, iOS, watchOS). We want to show a user the total time they've spent listening, aggregated across every device they use, with an optional account to centralize it — without turning a white-noise app into a surveillance liability.

> An ADR for the [Cascade](https://github.com/JacobStephens2/cascade) "one core → six shells" architecture. Mirrors into `infrastructure-patterns`.

## Decision

Track listening time as a **grow-only counter (a G-Counter CRDT)** that accrues inside the pure core, syncs through a minimal account service, and is **structurally incapable of recording a timeline**. Concretely, six linked decisions:

### 1. Accrue in the pure core, on the existing tick path

The core already receives wall-clock deltas via a `Tick` command (it owns no clock). Listening accrues there, as a pure accumulator — no new clock, filesystem, or network dependency. This keeps measurement identical across all six shells by construction: there is exactly one definition of "how much was played," and it lives in the one place every shell already shares.

### 2. Gate on *confirmed* playback, not intent

A naïve gate (`is_playing`) over-counts: a browser that blocks autoplay reports "playing" the instant the user clicks, but no audio is out. We added an `audio_confirmed_playing` flag, flipped by the platform's real playback signal (`PlatformPlaybackStarted` / paused / error), and accrue only when it's true and not muted. "Listening time" then means time audio was actually audible, even on shells with weak autoplay guarantees.

### 3. Merge with a G-Counter, never last-write-wins

Two devices listening concurrently must **add**, not overwrite. LWW silently destroys one device's hours. So each device owns a monotonic slot; the server merges with `GREATEST(existing, incoming)` per device on write and `SUM` across a user's devices on read. The device is a CRDT replica; the server does nothing cleverer than max-and-sum. The only attack surface a grow-only counter has is the per-tick *input* delta, which the core clamps (≤ 5 s/tick) so a sleep/wake gap or clock jump can't inflate it.

### 4. Data minimization *by construction*

The account stores an email and **one integer per device** — no timestamps, no session log, no event stream. The schema literally cannot express *when* someone listened, only *how much*. This is the difference between "we choose not to store your timeline" and "we cannot": the stronger, checkable claim, and the reason default-on tracking is defensible.

### 5. The shell decides when it *can* talk; the core decides whether there is anything to say, and what

*Amended 2026-09-27 (issue #18).* Originally "sync cadence lives in the shells, not the core", which conflated two things. Knowledge only a shell has — lifecycle, reachability, auth state (web `pagehide`, Android `onStop`, etc.) — stays in the shell, as does the HTTP transport. Protocol rules the core already has every input for now live in the core, defined once:

- **Threshold and payload.** The shell dispatches `BeginListeningSync { reason }` (`threshold` / `flush` / `refresh`). If there is something worth sending (≥ 30 s unsynced for `threshold`, anything for `flush`, always for `refresh`) the core answers with one `PushListening { deviceId, deviceTotalMs }`, and the shell PUTs exactly that.
- **High-water mark.** The shell reports `ListeningSyncSucceeded { serverTotalMs }` or `ListeningSyncFailed { unauthorized }`. The core marks as synced exactly the total it sent — never a shell-reconstructed value — so accrual during the request stays unsynced.
- **Re-entrancy.** A sync in flight blocks the next `BeginListeningSync` until it is settled — including one sent for a slot a reset has since replaced, whose ack is then dropped.
- **401 rule.** `unauthorized: true` makes the core emit `ClearSession`; the shell drops its token.

Six shells running decision 5 as first written produced ~20 copies of these four rules, with no test on any shell; they are now one tested module (`crates/cascade-core/tests/listening_sync.rs`) and four thin adapters.

*Amended 2026-10-03 (issue #31).* "Auth state" is no longer on the list of things only a shell knows. Whether the user is signed in, and as whom, is data the core can hold, and four hand-written account flows had drifted with no test on any of them. The core now holds the **Account** — the session and its rules — while reachability, lifecycle, the HTTP transport and storage stay in the shell:

- **Request and settle, as above.** User intents (`RequestSignInLink`, `SubmitSignInLink`, `SignOut`, `DeleteListeningData`, `DeleteAccount`) are answered with one effect describing an HTTP request (`SendSignInLink`, `VerifySignInToken`, `DeleteServerListening`, `DeleteServerAccount`, plus the fire-and-forget `RevokeSession`). The shell settles each with a success command or `AccountRequestFailed { unauthorized }`. One account request runs at a time; `SignOut` always works and drops it.
- **The session rides on effects.** `PushListening` carries `sessionToken`, and `BeginListeningSync` sends nothing while signed out. The snapshot's `account` section never carries the token.
- **Signing out forgets the server.** Sign-out, a 401 on any request, and a deleted account forget the cross-device total and supersede any in-flight sync, so neither a stale total nor a late ack outlives the account.
- **401 rule, revised.** `unauthorized: true` signs out and emits an empty `PersistAccount`. For a listening sync this holds whether or not the core held a session; only a sync that sign-out or a reset has already superseded is ignored. *(Issue #36: `ClearSession` is gone, since every shell now takes the account from the core.)*
- **Storage.** The core persists the account through `PersistAccount { json }` and restores it from `Restore`'s `accountJson`; each shell keeps its own storage location.

The rules are tested once in `crates/cascade-core/tests/account.rs`.

### 6. Opaque tokens + magic-link, and `device_id` rotation on delete

Auth is email magic-link (no passwords) with **opaque server-side session tokens** (not JWT), so logout / delete-account revoke instantly with one `DELETE`. Tokens are stored only as SHA-256 hashes. "Delete my data" rotates the client's `device_id`, closing the one loophole inherent to grow-only counters: a forgotten offline device can't later resurrect a deleted total by pushing a stale higher counter — it lands in a fresh slot.

*Amended 2026-09-27 (issue #18).* The core owns `device_id`, inside the persisted listening blob. `ResetListeningData { newDeviceId }` zeroes the slot and rotates the id in the same state change, so both reach disk in one `PersistListening` write: a crash can no longer leave a fresh id holding the old total (which the next sync would write into a new server slot — exactly the resurrection this decision exists to prevent). The server cannot enforce this — deletes are by `user_id` only — so the client-side atomicity is the whole guarantee. The core has no randomness, so shells supply ids: a fresh UUID on reset, and `restore`'s `fallbackDeviceId` that the core adopts only when the listening blob has none (shells pass the id they used to store themselves; the core takes it only on the first launch after upgrading, so existing server slots carry over).

## Consequences

**Positive**
- One measurement definition across six platforms; adding a shell adds no listening logic to the core.
- Concurrent multi-device use is correct (adds, never clobbers) with trivial server logic.
- The privacy posture is a *structural* property, not a promise — auditable, and a strong legal stance for default-on tracking.
- The backend is tiny: Rust/Axum + SQLx + Postgres, four tables, ~seven endpoints, runtime queries (no compile-time DB), migrations on boot.

**Negative / trade-offs**
- A user can inflate *their own* counter (it's a vanity number; per-device slots mean it never affects anyone else — accepted).
- No audit log and no timeline means no per-session analytics — deliberate, and revisited only if scope changes.
- Grow-only counters require the deletion/rotation dance (decision 6) to be correct.

## Alternatives considered

- **Last-write-wins / "latest total" sync** — rejected: destroys concurrent device time (the whole point is that concurrent listening must add).
- **Server-side session events with timestamps** — rejected: richer analytics, but reintroduces the timeline we specifically refuse to store.
- **JWT sessions** — rejected: revocation requires either short expiries or a denylist; opaque tokens make logout/delete a single `DELETE` on a single VPS.
- **OAuth / passwords** — rejected: disproportionate for an opt-in counter; magic-link is the smallest cross-platform path with the least PII.
- **Sync orchestration in the core** — rejected: the core can't know reachability/lifecycle/auth; that knowledge belongs in each shell. (Sync *policy* — threshold, payload, 401 rule, device-id lifecycle — was later moved into the core; see the decision 5 amendment.)
- **The account in the shell** — first chosen as "auth state" in decision 5, and reversed in its 2026-10-03 amendment: the session is data the core can hold, and four shell copies of the account flow drifted. Reachability, lifecycle, transport and storage stay in the shell.
- **Transport in the core** (a Rust HTTP client) — rejected in the amendment: it would drag an async HTTP stack into the wasm build and fight each platform's lifecycle.

## Validation

Core: 49 Rust unit/property tests (since grown by the listening-sync suite in `crates/cascade-core/tests/listening_sync.rs`) + 12 assertions against the wasm build (gating, clamp, restore monotonicity, sync baseline, wire shape). Backend: end-to-end against Postgres (single-use magic links, `GREATEST` merge keeping the higher slot, `SUM`, delete-cascade session revocation, 401 without a token). Web↔backend: a real headless-Chrome magic-link sign-in. A focused security review of the branch found no newly-introduced vulnerabilities (see `server/docs/threat-model.md`).
