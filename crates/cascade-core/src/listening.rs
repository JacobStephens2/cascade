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
}

impl Default for ListeningLedger {
    fn default() -> Self {
        Self {
            device_total_ms: 0,
            synced_through_ms: 0,
            server_total_ms: None,
            tracking_enabled: true,
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

    /// Record a successful sync: the server has accepted everything up to
    /// `synced_through_ms` and reports `server_total_ms` as the cross-device
    /// aggregate. This only moves the display baseline forward — it never
    /// touches `device_total_ms`. Monotonic in `synced_through_ms` so an
    /// out-of-order ack can't rewind the high-water mark.
    pub fn apply_synced(&mut self, synced_through_ms: u64, server_total_ms: u64) {
        self.synced_through_ms = self.synced_through_ms.max(synced_through_ms);
        self.server_total_ms = Some(server_total_ms);
    }

    /// Zero the local slot for a "delete my listening data" request. The shell
    /// is responsible for rotating its `device_id` alongside this so a stale
    /// offline write can't later resurrect the deleted total against the old
    /// slot. `tracking_enabled` is left as-is — deleting data is not the same
    /// as turning the feature off.
    pub fn reset(&mut self) {
        self.device_total_ms = 0;
        self.synced_through_ms = 0;
        self.server_total_ms = None;
    }

    /// Adopt a restored ledger without ever *lowering* the live counters. A
    /// best-effort writer (Windows/macOS) can leave a partially-written blob,
    /// and a restore must never regress a lifetime total — so the grow-only
    /// fields take the max. `server_total_ms` and `tracking_enabled` are
    /// authoritative from the blob.
    pub fn restore_from(&mut self, restored: &ListeningLedger) {
        self.device_total_ms = self.device_total_ms.max(restored.device_total_ms);
        self.synced_through_ms = self.synced_through_ms.max(restored.synced_through_ms);
        self.server_total_ms = restored.server_total_ms;
        self.tracking_enabled = restored.tracking_enabled;
    }

    pub fn to_persisted(&self) -> PersistedListening {
        PersistedListening {
            version: LISTENING_VERSION,
            device_total_ms: self.device_total_ms,
            synced_through_ms: self.synced_through_ms,
            server_total_ms: self.server_total_ms,
            tracking_enabled: self.tracking_enabled,
        }
    }

    pub fn from_persisted(p: &PersistedListening) -> Self {
        Self {
            device_total_ms: p.device_total_ms,
            synced_through_ms: p.synced_through_ms.min(p.device_total_ms),
            server_total_ms: p.server_total_ms,
            tracking_enabled: p.tracking_enabled,
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
        l.apply_synced(3_000, 10_000);
        assert_eq!(l.unsynced_ms(), 0);
        // A stale, smaller ack can't rewind the high-water mark or underflow.
        l.apply_synced(1_000, 10_000);
        assert_eq!(l.synced_through_ms, 3_000);
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
        l.apply_synced(5_000, 3_600_000);
        l.accrue(2_000);
        assert_eq!(l.displayed_total_ms(), 3_600_000 + 2_000);
    }

    #[test]
    fn apply_synced_never_lowers_device_slot() {
        let mut l = ListeningLedger::default();
        l.accrue(4_000);
        // Even if the server somehow reports a smaller aggregate, the local
        // grow-only slot is untouched.
        l.apply_synced(4_000, 1_000);
        assert_eq!(l.device_total_ms, 4_000);
    }

    #[test]
    fn reset_zeros_local_state_but_leaves_tracking_flag() {
        let mut l = ListeningLedger::default();
        l.accrue(4_000);
        l.apply_synced(4_000, 9_000);
        l.tracking_enabled = true;
        l.reset();
        assert_eq!(l.device_total_ms, 0);
        assert_eq!(l.synced_through_ms, 0);
        assert_eq!(l.server_total_ms, None);
        assert!(l.tracking_enabled, "deleting data must not flip the toggle");
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
        l.accrue(1_234);
        l.apply_synced(1_000, 8_000);
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
        };
        let json = serde_json::to_string(&l.to_persisted()).unwrap();
        assert!(json.contains(r#""deviceTotalMs":7"#));
        assert!(json.contains(r#""syncedThroughMs":3"#));
        assert!(json.contains(r#""serverTotalMs":9"#));
        assert!(json.contains(r#""trackingEnabled":true"#));
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
