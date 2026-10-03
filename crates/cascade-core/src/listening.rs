//! Listening-time tracking — a grow-only per-device counter (a G-Counter slot).
//!
//! The core accrues *audible* listening time on the existing [`Command::Tick`]
//! path. It is a pure accumulator: it never reads the clock (the platform hands
//! it deltas), never touches the network, and the device slot never decreases.
//!
//! The device's lifetime total is one replica of a G-Counter (grow-only
//! counter). The server merges replicas with `max` per device and `sum` across
//! a user's devices — so two devices listening concurrently *add* rather than
//! overwrite. Crucially, the only thing ever stored is an aggregate
//! millisecond count: no timestamps, no session log. A listening *timeline*
//! cannot be reconstructed from this data, which is the whole defensibility
//! argument for tracking being on by default — we don't merely choose not to
//! store when you listened, we structurally *cannot*.
//!
//! [`Command::Tick`]: crate::Command::Tick

use serde::{Deserialize, Serialize};

/// Storage-schema version for the persisted listening blob. Separate from
/// `SETTINGS_VERSION` because listening data lives in its own blob
/// (`cascade.listening.v1`), distinct from user settings.
pub const LISTENING_VERSION: u32 = 1;

/// Upper bound on how much a single `Tick` may add to the counter. Shells tick
/// at roughly 250 ms during playback; any delta larger than this is a
/// sleep/wake gap or a clock jump, not real listening, so it is clamped. This
/// is the cheap input-sanitization guard for the one attack surface a G-Counter
/// has — the per-tick delta the merge consumes.
pub const MAX_TICK_ACCRUAL_MS: u64 = 5_000;

/// How much unsynced listening a [`SyncReason::Threshold`] sync waits for
/// before it is worth a PUT. One definition for every shell.
pub const LISTENING_SYNC_THRESHOLD_MS: u64 = 30_000;

/// Why a shell is asking to sync. The shell decides *when it can* talk
/// (reachability, lifecycle); the reason lets the core decide *whether
/// there is anything to say*.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SyncReason {
    /// Routine check (e.g. on every snapshot): send only once at least
    /// [`LISTENING_SYNC_THRESHOLD_MS`] is unsynced.
    Threshold,
    /// The app is backgrounding or closing: send any unsynced time at all.
    Flush,
    /// Just signed in or launched with an account: always send, so the
    /// cross-device aggregate comes back even with nothing new to report.
    Refresh,
}

/// The in-memory listening ledger. Held inside [`crate::State`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListeningLedger {
    /// Grow-only count of confirmed audible milliseconds on THIS device — the
    /// device's G-Counter slot. Never decreases within a device identity.
    pub device_total_ms: u64,
    /// High-water mark of `device_total_ms` already durably accepted by the
    /// server. Sync bookkeeping only; it never lowers `device_total_ms`.
    pub synced_through_ms: u64,
    /// The server's aggregate across all of the user's devices, as of the last
    /// successful sync. `None` until the device first hears from the server
    /// (no account, or never synced). Display only.
    pub server_total_ms: Option<u64>,
    /// Whether listening-time tracking is on. Defaults to `true` (opt-out).
    pub tracking_enabled: bool,
    /// This device's opaque id — the key of its server slot. Owned here, in
    /// the same blob as `device_total_ms`, so rotating it and zeroing the slot
    /// are one persisted write. `None` until [`Self::adopt_device_id`].
    pub device_id: Option<String>,
    /// The PUT currently in flight, if any. Transient — never persisted; a
    /// restart simply forgets the request.
    pub in_flight: Option<InFlight>,
}

/// A listening PUT the shell has been told to send and hasn't settled yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InFlight {
    /// Sent for the current slot, carrying this `device_total_ms`.
    Sent { device_total_ms: u64 },
    /// Sent for a slot a reset has since replaced. Its ack says nothing about
    /// the current slot and is dropped — but it still blocks the next sync,
    /// because acks carry no request identity and it could otherwise settle
    /// a newer PUT.
    Superseded,
}

impl Default for ListeningLedger {
    fn default() -> Self {
        Self {
            device_total_ms: 0,
            synced_through_ms: 0,
            server_total_ms: None,
            tracking_enabled: true,
            device_id: None,
            in_flight: None,
        }
    }
}

impl ListeningLedger {
    /// Milliseconds accrued locally that the server has not yet acknowledged —
    /// what a shell watches to decide when to sync. Saturating so a hand-edited
    /// or partially-written blob can never underflow.
    pub fn unsynced_ms(&self) -> u64 {
        self.device_total_ms.saturating_sub(self.synced_through_ms)
    }

    /// The lifetime total to show the user. With an account this is the server
    /// aggregate plus any locally-accrued time not yet synced, so the number
    /// climbs live even while listening offline. Without an account it is just
    /// this device's slot.
    pub fn displayed_total_ms(&self) -> u64 {
        match self.server_total_ms {
            Some(server) => server.saturating_add(self.unsynced_ms()),
            None => self.device_total_ms,
        }
    }

    /// Accrue one tick of listening, clamped to [`MAX_TICK_ACCRUAL_MS`]. The
    /// caller is responsible for checking the gate (tracking on, audio
    /// confirmed, not muted) before calling this.
    pub fn accrue(&mut self, elapsed_ms: u64) {
        let delta = elapsed_ms.min(MAX_TICK_ACCRUAL_MS);
        self.device_total_ms = self.device_total_ms.saturating_add(delta);
    }

    /// Decide whether to sync, and if so what to send: `(device_id,
    /// device_total_ms)`. Marks that total as in flight, so a second call
    /// returns `None` until [`Self::sync_succeeded`] or [`Self::sync_failed`]
    /// settles it — the one re-entrancy guard every shell shares.
    pub fn begin_sync(&mut self, reason: SyncReason) -> Option<(String, u64)> {
        if self.in_flight.is_some() {
            return None;
        }
        let device_id = self.device_id.clone()?;
        let worth_sending = match reason {
            SyncReason::Threshold => self.unsynced_ms() >= LISTENING_SYNC_THRESHOLD_MS,
            SyncReason::Flush => self.unsynced_ms() > 0,
            SyncReason::Refresh => true,
        };
        if !worth_sending {
            return None;
        }
        self.in_flight = Some(InFlight::Sent {
            device_total_ms: self.device_total_ms,
        });
        Some((device_id, self.device_total_ms))
    }

    /// The in-flight PUT succeeded. The server has accepted exactly what was
    /// sent — not whatever accrued while the request was out — and reports
    /// `server_total_ms` as the cross-device aggregate. This only moves the
    /// display baseline forward; it never touches `device_total_ms`, and is
    /// monotonic so an out-of-order ack can't rewind the high-water mark.
    /// Returns `false` (and changes nothing else) if no sync for the current
    /// slot was in flight.
    pub fn sync_succeeded(&mut self, server_total_ms: u64) -> bool {
        let Some(InFlight::Sent {
            device_total_ms: sent,
        }) = self.in_flight.take()
        else {
            return false;
        };
        self.synced_through_ms = self.synced_through_ms.max(sent);
        self.server_total_ms = Some(server_total_ms);
        true
    }

    /// The in-flight PUT failed. Nothing was acknowledged; the next trigger
    /// may try again.
    pub fn sync_failed(&mut self) {
        self.in_flight = None;
    }

    /// Take `fallback` as this device's id if it doesn't have one yet.
    /// Returns whether it was adopted (and so needs persisting).
    pub fn adopt_device_id(&mut self, fallback: String) -> bool {
        if self.device_id.is_some() {
            return false;
        }
        self.device_id = Some(fallback);
        true
    }

    /// Zero the local slot for a "delete my listening data" request and move to
    /// `new_device_id`, so a stale offline write can't later resurrect the
    /// deleted total against the old slot. Rotation and zeroing happen in the
    /// same state change, so they reach disk in the same persisted write. Any
    /// in-flight sync belonged to the old slot and becomes
    /// [`InFlight::Superseded`].
    /// `tracking_enabled` is left as-is — deleting data is not the same as
    /// turning the feature off.
    pub fn reset(&mut self, new_device_id: String) {
        self.device_total_ms = 0;
        self.synced_through_ms = 0;
        self.server_total_ms = None;
        self.device_id = Some(new_device_id);
        if self.in_flight.is_some() {
            self.in_flight = Some(InFlight::Superseded);
        }
    }

    /// Forget the cross-device total when the account goes away (sign-out, a
    /// 401, account deleted): the display falls back to this device's slot.
    /// Any in-flight sync belonged to that account and becomes
    /// [`InFlight::Superseded`], so its late ack can't bring the total back.
    /// The device slot and its sync bookkeeping are untouched.
    pub fn forget_server(&mut self) {
        self.server_total_ms = None;
        if self.in_flight.is_some() {
            self.in_flight = Some(InFlight::Superseded);
        }
    }

    /// Adopt a restored ledger without ever *lowering* the live counters. A
    /// best-effort writer (Windows/macOS) can leave a partially-written blob,
    /// and a restore must never regress a lifetime total — so the grow-only
    /// fields take the max. `server_total_ms` and `tracking_enabled` are
    /// authoritative from the blob, as is `device_id` when the blob has one.
    pub fn restore_from(&mut self, restored: &ListeningLedger) {
        self.device_total_ms = self.device_total_ms.max(restored.device_total_ms);
        self.synced_through_ms = self.synced_through_ms.max(restored.synced_through_ms);
        self.server_total_ms = restored.server_total_ms;
        self.tracking_enabled = restored.tracking_enabled;
        if restored.device_id.is_some() {
            self.device_id = restored.device_id.clone();
        }
    }

    pub fn to_persisted(&self) -> PersistedListening {
        PersistedListening {
            version: LISTENING_VERSION,
            device_total_ms: self.device_total_ms,
            synced_through_ms: self.synced_through_ms,
            server_total_ms: self.server_total_ms,
            tracking_enabled: self.tracking_enabled,
            device_id: self.device_id.clone(),
        }
    }

    pub fn from_persisted(p: &PersistedListening) -> Self {
        Self {
            device_total_ms: p.device_total_ms,
            synced_through_ms: p.synced_through_ms.min(p.device_total_ms),
            server_total_ms: p.server_total_ms,
            tracking_enabled: p.tracking_enabled,
            device_id: p.device_id.clone(),
            in_flight: None,
        }
    }
}

/// The persisted form of the listening ledger — the JSON the platform stores in
/// its own blob, separate from settings. Versioned so future schema changes are
/// handled explicitly.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PersistedListening {
    pub version: u32,
    pub device_total_ms: u64,
    pub synced_through_ms: u64,
    /// Persisted so the lifetime display survives a restart instead of dropping
    /// to device-only until the next sync. `#[serde(default)]` keeps older
    /// blobs (written before this field existed) loadable.
    #[serde(default)]
    pub server_total_ms: Option<u64>,
    pub tracking_enabled: bool,
    /// The device's server-slot id. `#[serde(default)]` keeps blobs written
    /// before the core owned the id loadable; the shell's fallback fills it.
    #[serde(default)]
    pub device_id: Option<String>,
}

/// Format a millisecond total as a coarse human label: `"12h 34m"`, `"59m"`,
/// `"0m"`. Shells may reformat, but the snapshot ships a ready string so every
/// UI agrees by default.
pub fn format_listening_total(total_ms: u64) -> String {
    let total_minutes = total_ms / 60_000;
    let hours = total_minutes / 60;
    let minutes = total_minutes % 60;
    if hours > 0 {
        format!("{hours}h {minutes:02}m")
    } else {
        format!("{minutes}m")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Send whatever is unsynced and have the server accept it.
    fn sync(l: &mut ListeningLedger, server_total_ms: u64) {
        l.device_id.get_or_insert_with(|| "device".into());
        l.begin_sync(SyncReason::Refresh)
            .expect("nothing in flight");
        assert!(l.sync_succeeded(server_total_ms));
    }

    #[test]
    fn accrue_clamps_per_tick_to_neutralize_clock_jumps() {
        let mut l = ListeningLedger::default();
        l.accrue(250);
        assert_eq!(l.device_total_ms, 250);
        // A 10-minute "tick" is a sleep/wake gap, not listening — clamp it.
        l.accrue(600_000);
        assert_eq!(l.device_total_ms, 250 + MAX_TICK_ACCRUAL_MS);
    }

    #[test]
    fn unsynced_is_device_minus_synced_and_never_underflows() {
        let mut l = ListeningLedger::default();
        l.accrue(3_000);
        assert_eq!(l.unsynced_ms(), 3_000);
        sync(&mut l, 10_000);
        assert_eq!(l.unsynced_ms(), 0);
        // A partially-written blob with synced > device can't underflow.
        l.synced_through_ms = 5_000;
        assert_eq!(l.unsynced_ms(), 0);
    }

    #[test]
    fn displayed_total_uses_server_aggregate_plus_unsynced() {
        let mut l = ListeningLedger::default();
        l.accrue(5_000);
        // No account yet → show the device slot.
        assert_eq!(l.displayed_total_ms(), 5_000);
        // After a sync that reports a 1h aggregate across devices, the live
        // display is aggregate + whatever we've accrued since.
        sync(&mut l, 3_600_000);
        l.accrue(2_000);
        assert_eq!(l.displayed_total_ms(), 3_600_000 + 2_000);
    }

    #[test]
    fn sync_never_lowers_device_slot() {
        let mut l = ListeningLedger::default();
        l.accrue(4_000);
        // Even if the server somehow reports a smaller aggregate, the local
        // grow-only slot is untouched.
        sync(&mut l, 1_000);
        assert_eq!(l.device_total_ms, 4_000);
    }

    #[test]
    fn reset_zeros_local_state_but_leaves_tracking_flag() {
        let mut l = ListeningLedger::default();
        l.accrue(4_000);
        sync(&mut l, 9_000);
        l.tracking_enabled = true;
        l.reset("rotated".into());
        assert_eq!(l.device_total_ms, 0);
        assert_eq!(l.synced_through_ms, 0);
        assert_eq!(l.server_total_ms, None);
        assert_eq!(l.device_id.as_deref(), Some("rotated"));
        assert!(l.tracking_enabled, "deleting data must not flip the toggle");
    }

    #[test]
    fn forget_server_drops_the_total_and_supersedes_but_keeps_the_slot() {
        let mut l = ListeningLedger::default();
        l.accrue(4_000);
        sync(&mut l, 9_000);
        l.accrue(1_000);
        l.begin_sync(SyncReason::Flush).expect("nothing in flight");
        l.forget_server();
        assert_eq!(l.server_total_ms, None);
        assert_eq!(l.in_flight, Some(InFlight::Superseded));
        assert_eq!(l.device_total_ms, 5_000);
        assert_eq!(l.synced_through_ms, 4_000);
        assert_eq!(l.displayed_total_ms(), 5_000);
    }

    #[test]
    fn restore_never_regresses_the_lifetime_total() {
        let mut live = ListeningLedger::default();
        live.accrue(5_000); // already 5s in memory
                            // A stale/partial blob says 3s — restoring must not lower the total.
        let stale = ListeningLedger {
            device_total_ms: 3_000,
            synced_through_ms: 0,
            server_total_ms: None,
            tracking_enabled: false,
            device_id: None,
            in_flight: None,
        };
        live.restore_from(&stale);
        assert_eq!(live.device_total_ms, 5_000);
        assert!(
            !live.tracking_enabled,
            "tracking flag is authoritative from blob"
        );
    }

    #[test]
    fn persisted_round_trips() {
        let mut l = ListeningLedger::default();
        l.accrue(1_000);
        sync(&mut l, 8_000);
        l.accrue(234);
        let json = serde_json::to_string(&l.to_persisted()).unwrap();
        let back: PersistedListening = serde_json::from_str(&json).unwrap();
        assert_eq!(back, l.to_persisted());
        assert_eq!(ListeningLedger::from_persisted(&back), l);
    }

    #[test]
    fn persisted_blob_is_camel_case_for_the_shells() {
        let l = ListeningLedger {
            device_total_ms: 7,
            synced_through_ms: 3,
            server_total_ms: Some(9),
            tracking_enabled: true,
            device_id: Some("d".into()),
            in_flight: Some(InFlight::Sent { device_total_ms: 7 }),
        };
        let json = serde_json::to_string(&l.to_persisted()).unwrap();
        assert!(json.contains(r#""deviceTotalMs":7"#));
        assert!(json.contains(r#""syncedThroughMs":3"#));
        assert!(json.contains(r#""serverTotalMs":9"#));
        assert!(json.contains(r#""trackingEnabled":true"#));
        assert!(json.contains(r#""deviceId":"d""#));
        assert!(!json.contains("inFlight"), "in-flight state is transient");
    }

    #[test]
    fn older_blob_without_server_total_still_loads() {
        let json =
            r#"{"version":1,"deviceTotalMs":100,"syncedThroughMs":50,"trackingEnabled":true}"#;
        let p: PersistedListening = serde_json::from_str(json).unwrap();
        assert_eq!(p.server_total_ms, None);
        assert_eq!(p.device_total_ms, 100);
    }

    #[test]
    fn format_total_uses_hours_and_minutes() {
        assert_eq!(format_listening_total(0), "0m");
        assert_eq!(format_listening_total(59_000), "0m");
        assert_eq!(format_listening_total(60_000), "1m");
        assert_eq!(format_listening_total(3_600_000), "1h 00m");
        assert_eq!(format_listening_total(45_240_000), "12h 34m");
    }
}
