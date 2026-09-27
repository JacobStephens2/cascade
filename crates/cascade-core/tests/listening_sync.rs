//! Listening-sync policy, driven only through `Core::dispatch` — no network.
//!
//! The shell owns *when* it can talk (reachability, lifecycle, auth); the core
//! owns *whether there is anything to say, and what*. These tests pin the core
//! half: the threshold, the payload, the synced high-water mark, the 401 rule,
//! and the device-id lifecycle.

use cascade_core::{Command, Core, Effect, SyncReason, LISTENING_SYNC_THRESHOLD_MS};

const DEVICE_A: &str = "device-a";
const DEVICE_B: &str = "device-b";

/// A core that has restored (nothing) with `DEVICE_A` as its fallback id and
/// has accrued `listened_ms` of confirmed audio.
fn core_with_listening(listened_ms: u64) -> Core {
    let mut core = Core::new();
    core.dispatch(Command::RestoreListening {
        json: String::new(),
        fallback_device_id: DEVICE_A.into(),
    });
    core.dispatch(Command::Play);
    core.dispatch(Command::PlatformPlaybackStarted);
    let mut left = listened_ms;
    while left > 0 {
        let step = left.min(1_000);
        core.dispatch(Command::Tick { elapsed_ms: step });
        left -= step;
    }
    core
}

/// The `PushListening` payload in `effects`, if any.
fn pushed(effects: &[Effect]) -> Option<(String, u64)> {
    effects.iter().find_map(|e| match e {
        Effect::PushListening {
            device_id,
            device_total_ms,
        } => Some((device_id.clone(), *device_total_ms)),
        _ => None,
    })
}

fn begin(core: &mut Core, reason: SyncReason) -> Option<(String, u64)> {
    pushed(
        &core
            .dispatch(Command::BeginListeningSync { reason })
            .effects,
    )
}

#[test]
fn synced_through_is_exactly_what_was_sent() {
    let mut core = core_with_listening(40_000);
    let (device_id, sent) = begin(&mut core, SyncReason::Threshold).expect("40s is past threshold");
    assert_eq!(device_id, DEVICE_A);
    assert_eq!(sent, 40_000);

    // More listening lands while the PUT is in flight.
    core.dispatch(Command::Tick { elapsed_ms: 2_000 });

    let snap = core
        .dispatch(Command::ListeningSyncSucceeded {
            server_total_ms: 1_000_000,
        })
        .snapshot;
    // Only what was sent is acknowledged; the 2s accrued mid-flight stays
    // unsynced rather than being silently marked as delivered.
    assert_eq!(snap.listening.unsynced_ms, 2_000);
    assert_eq!(snap.listening.displayed_total_ms, 1_000_000 + 2_000);
}

// ---- threshold -------------------------------------------------------------

#[test]
fn threshold_sync_waits_for_thirty_seconds_of_unsynced_time() {
    let mut core = core_with_listening(LISTENING_SYNC_THRESHOLD_MS - 1_000);
    assert_eq!(begin(&mut core, SyncReason::Threshold), None);

    core.dispatch(Command::Tick { elapsed_ms: 1_000 });
    assert_eq!(
        begin(&mut core, SyncReason::Threshold),
        Some((DEVICE_A.into(), LISTENING_SYNC_THRESHOLD_MS))
    );
}

#[test]
fn flush_sends_any_unsynced_time_but_not_nothing() {
    let mut core = core_with_listening(0);
    assert_eq!(begin(&mut core, SyncReason::Flush), None);

    core.dispatch(Command::Tick { elapsed_ms: 1 });
    assert_eq!(
        begin(&mut core, SyncReason::Flush),
        Some((DEVICE_A.into(), 1))
    );
}

#[test]
fn refresh_sends_even_with_nothing_unsynced_to_fetch_the_aggregate() {
    let mut core = core_with_listening(0);
    assert_eq!(
        begin(&mut core, SyncReason::Refresh),
        Some((DEVICE_A.into(), 0))
    );
}

#[test]
fn nothing_is_sent_before_the_core_has_a_device_id() {
    let mut core = Core::new();
    assert_eq!(begin(&mut core, SyncReason::Refresh), None);
}

// ---- payload / high-water mark ---------------------------------------------

#[test]
fn a_sync_is_not_reentrant_until_the_first_settles() {
    let mut core = core_with_listening(40_000);
    assert!(begin(&mut core, SyncReason::Threshold).is_some());
    assert_eq!(
        begin(&mut core, SyncReason::Refresh),
        None,
        "one PUT in flight at a time"
    );

    core.dispatch(Command::ListeningSyncFailed {
        unauthorized: false,
    });
    assert!(
        begin(&mut core, SyncReason::Threshold).is_some(),
        "a failure frees the slot for the next trigger"
    );
}

#[test]
fn a_transient_failure_acknowledges_nothing() {
    let mut core = core_with_listening(40_000);
    begin(&mut core, SyncReason::Threshold);
    let update = core.dispatch(Command::ListeningSyncFailed {
        unauthorized: false,
    });
    assert_eq!(update.snapshot.listening.unsynced_ms, 40_000);
    assert!(!update.effects.contains(&Effect::ClearSession));
}

#[test]
fn an_ack_with_nothing_in_flight_is_ignored() {
    let mut core = core_with_listening(40_000);
    let snap = core
        .dispatch(Command::ListeningSyncSucceeded {
            server_total_ms: 9_000_000,
        })
        .snapshot;
    assert_eq!(snap.listening.unsynced_ms, 40_000);
    assert_eq!(snap.listening.displayed_total_ms, 40_000);
}

#[test]
fn a_successful_sync_persists_the_new_high_water_mark() {
    let mut core = core_with_listening(40_000);
    begin(&mut core, SyncReason::Threshold);
    let effects = core
        .dispatch(Command::ListeningSyncSucceeded {
            server_total_ms: 50_000,
        })
        .effects;
    let json = persisted_json(&effects).expect("ack persists the ledger");
    assert!(json.contains(r#""syncedThroughMs":40000"#), "{json}");
}

// ---- 401 rule --------------------------------------------------------------

#[test]
fn unauthorized_clears_the_session_and_keeps_local_listening() {
    let mut core = core_with_listening(40_000);
    begin(&mut core, SyncReason::Threshold);
    let update = core.dispatch(Command::ListeningSyncFailed { unauthorized: true });
    assert!(update.effects.contains(&Effect::ClearSession));
    assert_eq!(update.snapshot.listening.device_total_ms, 40_000);
    assert_eq!(update.snapshot.listening.unsynced_ms, 40_000);
}

// ---- device-id lifecycle ---------------------------------------------------

/// The `PersistListening` JSON in `effects`, if any.
fn persisted_json(effects: &[Effect]) -> Option<String> {
    effects.iter().find_map(|e| match e {
        Effect::PersistListening { json } => Some(json.clone()),
        _ => None,
    })
}

#[test]
fn restore_without_a_stored_id_adopts_and_persists_the_fallback() {
    let mut core = Core::new();
    let effects = core
        .dispatch(Command::RestoreListening {
            json: String::new(),
            fallback_device_id: DEVICE_A.into(),
        })
        .effects;
    let json = persisted_json(&effects).expect("a freshly adopted id is made durable at once");
    assert!(json.contains(r#""deviceId":"device-a""#), "{json}");
}

#[test]
fn a_stored_id_wins_over_the_fallback_across_restarts() {
    let first = core_with_listening(3_000);
    let blob = serde_json::to_string(&first.state().listening.to_persisted()).unwrap();

    let mut second = Core::new();
    let effects = second
        .dispatch(Command::RestoreListening {
            json: blob,
            fallback_device_id: DEVICE_B.into(),
        })
        .effects;
    assert!(
        persisted_json(&effects).is_none(),
        "nothing new to persist when the blob already carries an id"
    );
    assert_eq!(
        begin(&mut second, SyncReason::Flush),
        Some((DEVICE_A.into(), 3_000))
    );
}

#[test]
fn a_legacy_blob_without_an_id_keeps_its_total_and_adopts_the_fallback() {
    // Blobs written before the core owned the id. Shells pass the id they used
    // to store themselves as the fallback, so the server slot is preserved.
    let legacy = r#"{"version":1,"deviceTotalMs":100,"syncedThroughMs":50,"trackingEnabled":true}"#;
    let mut core = Core::new();
    core.dispatch(Command::RestoreListening {
        json: legacy.into(),
        fallback_device_id: DEVICE_B.into(),
    });
    assert_eq!(
        begin(&mut core, SyncReason::Flush),
        Some((DEVICE_B.into(), 100))
    );
}

#[test]
fn reset_rotates_the_id_and_zeroes_the_slot_in_one_persisted_write() {
    let mut core = core_with_listening(40_000);
    let effects = core
        .dispatch(Command::ResetListeningData {
            new_device_id: DEVICE_B.into(),
        })
        .effects;

    let writes: Vec<_> = effects
        .iter()
        .filter(|e| matches!(e, Effect::PersistListening { .. }))
        .collect();
    assert_eq!(writes.len(), 1, "rotation and reset must be one write");
    let json = persisted_json(&effects).unwrap();
    assert!(json.contains(r#""deviceId":"device-b""#), "{json}");
    assert!(json.contains(r#""deviceTotalMs":0"#), "{json}");

    // A crash right after that write restores to exactly the rotated, zeroed
    // slot — never the fresh id holding the old total.
    let mut restarted = Core::new();
    restarted.dispatch(Command::RestoreListening {
        json,
        fallback_device_id: "unused".into(),
    });
    assert_eq!(
        begin(&mut restarted, SyncReason::Refresh),
        Some((DEVICE_B.into(), 0))
    );
}

#[test]
fn an_ack_for_the_old_slot_is_dropped_after_a_reset() {
    let mut core = core_with_listening(40_000);
    begin(&mut core, SyncReason::Threshold);
    core.dispatch(Command::ResetListeningData {
        new_device_id: DEVICE_B.into(),
    });
    core.dispatch(Command::Tick { elapsed_ms: 1_000 });

    let snap = core
        .dispatch(Command::ListeningSyncSucceeded {
            server_total_ms: 1_000_000,
        })
        .snapshot;
    assert_eq!(
        snap.listening.unsynced_ms, 1_000,
        "the old slot's 40s must not be credited to the new one"
    );
    assert_eq!(snap.listening.displayed_total_ms, 1_000);
}

// ---- wire shape ------------------------------------------------------------
//
// Every shell hand-writes these JSON shapes. Lock them.

#[test]
fn sync_commands_accept_the_shells_camel_case_json() {
    let cases = [
        (
            r#"{"type":"restoreListening","json":"","fallbackDeviceId":"d"}"#,
            Command::RestoreListening {
                json: String::new(),
                fallback_device_id: "d".into(),
            },
        ),
        (
            r#"{"type":"beginListeningSync","reason":"threshold"}"#,
            Command::BeginListeningSync {
                reason: SyncReason::Threshold,
            },
        ),
        (
            r#"{"type":"beginListeningSync","reason":"flush"}"#,
            Command::BeginListeningSync {
                reason: SyncReason::Flush,
            },
        ),
        (
            r#"{"type":"beginListeningSync","reason":"refresh"}"#,
            Command::BeginListeningSync {
                reason: SyncReason::Refresh,
            },
        ),
        (
            r#"{"type":"listeningSyncSucceeded","serverTotalMs":5}"#,
            Command::ListeningSyncSucceeded { server_total_ms: 5 },
        ),
        (
            r#"{"type":"listeningSyncFailed","unauthorized":true}"#,
            Command::ListeningSyncFailed { unauthorized: true },
        ),
        (
            r#"{"type":"resetListeningData","newDeviceId":"d"}"#,
            Command::ResetListeningData {
                new_device_id: "d".into(),
            },
        ),
    ];
    for (json, expected) in cases {
        let parsed: Command = serde_json::from_str(json).expect(json);
        assert_eq!(parsed, expected, "{json}");
    }
}

#[test]
fn sync_effects_serialize_camel_case() {
    let push = Effect::PushListening {
        device_id: "d".into(),
        device_total_ms: 7,
    };
    assert_eq!(
        serde_json::to_string(&push).unwrap(),
        r#"{"type":"pushListening","deviceId":"d","deviceTotalMs":7}"#
    );
    assert_eq!(
        serde_json::to_string(&Effect::ClearSession).unwrap(),
        r#"{"type":"clearSession"}"#
    );
}

#[test]
fn a_new_sync_waits_for_the_old_slots_put_to_settle_after_a_reset() {
    let mut core = core_with_listening(40_000);
    begin(&mut core, SyncReason::Threshold);
    core.dispatch(Command::ResetListeningData {
        new_device_id: DEVICE_B.into(),
    });
    core.dispatch(Command::Tick { elapsed_ms: 1_000 });

    // Acks carry no request identity, so the old PUT's ack must not be able
    // to settle a new one: nothing starts until it comes back.
    assert_eq!(begin(&mut core, SyncReason::Refresh), None);

    let snap = core
        .dispatch(Command::ListeningSyncSucceeded {
            server_total_ms: 1_000_000,
        })
        .snapshot;
    assert_eq!(snap.listening.unsynced_ms, 1_000);
    assert_eq!(
        begin(&mut core, SyncReason::Refresh),
        Some((DEVICE_B.into(), 1_000))
    );
}
