# ADR 0001 — Cross-platform listening-time tracking: a pure-core G-Counter with data-minimizing sync

- **Status:** Accepted
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

### 5. Sync cadence lives in the shells, not the core

The core never emits a "sync now" effect. It exposes `unsyncedMs`; each shell decides when to talk to the server based on what only it knows — lifecycle, reachability, auth state (web Service Worker / `pagehide`, Android `WorkManager`/`onStop`, etc.). The core stays free of network policy.

### 6. Opaque tokens + magic-link, and `device_id` rotation on delete

Auth is email magic-link (no passwords) with **opaque server-side session tokens** (not JWT), so logout / delete-account revoke instantly with one `DELETE`. Tokens are stored only as SHA-256 hashes. "Delete my data" rotates the client's `device_id`, closing the one loophole inherent to grow-only counters: a forgotten offline device can't later resurrect a deleted total by pushing a stale higher counter — it lands in a fresh slot.

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
- **Sync orchestration in the core** — rejected: the core can't know reachability/lifecycle/auth; that knowledge (and the policy) belongs in each shell.

## Validation

Core: 49 Rust unit/property tests + 12 assertions against the wasm build (gating, clamp, restore monotonicity, sync baseline, wire shape). Backend: end-to-end against Postgres (single-use magic links, `GREATEST` merge keeping the higher slot, `SUM`, delete-cascade session revocation, 401 without a token). Web↔backend: a real headless-Chrome magic-link sign-in. A focused security review of the branch found no newly-introduced vulnerabilities (see `server/docs/threat-model.md`).
