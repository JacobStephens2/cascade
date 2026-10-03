//! Listening-sync policy, driven only through `Core::dispatch` — no network.
//!
//! The shell owns *when* it can talk (reachability, lifecycle); the core
//! owns *whether there is anything to say, and what*, and the routine
//! threshold push that a counted tick makes. These tests pin the core half:
//! the threshold and its failure pacing, the payload, the synced high-water
//! mark, the 401 rule, and the device-id lifecycle.

use cascade_core::{Command, Core, Effect, SyncReason, LISTENING_SYNC_THRESHOLD_MS};

const DEVICE_A: &str = "device-a";
const DEVICE_B: &str = "device-b";
/// A stored account, so the core holds a session and may sync.
const SIGNED_IN: &str = r#"{"version":1,"sessionToken":"session","email":"a@example.com"}"#;

/// A signed-in core that has restored no listening, with `DEVICE_A` as its
/// fallback id, and audio confirmed playing.
fn signed_in_and_playing() -> Core {
    let mut core = Core::new();
    core.dispatch(Command::Restore {
        settings_json: String::new(),
        listening_json: String::new(),
        fallback_device_id: DEVICE_A.into(),
        account_json: SIGNED_IN.into(),
    });
    core.dispatch(Command::Play);
    core.dispatch(Command::PlatformPlaybackStarted);
    core
}

/// Tick `listened_ms` of confirmed audio in one-second steps, returning every
/// effect the ticks emitted.
fn listen(core: &mut Core, listened_ms: u64) -> Vec<Effect> {
    let mut effects = Vec::new();
    let mut left = listened_ms;
    while left > 0 {
        let step = left.min(1_000);
        effects.extend(core.dispatch(Command::Tick { elapsed_ms: step }).effects);
        left -= step;
    }
    effects
}

/// A signed-in core that has accrued `listened_ms` of confirmed audio. From
/// 30 seconds on, the tick that crossed the threshold left a push in flight.
fn core_with_listening(listened_ms: u64) -> Core {
    let mut core = signed_in_and_playing();
    listen(&mut core, listened_ms);
    core
}

/// A signed-in core restored from a blob holding `unsynced_ms` of listening
/// the server has not seen, with `tracking` as the tracking flag. Audio is
/// confirmed playing, and no tick has run yet.
fn restored_with_unsynced(unsynced_ms: u64, tracking: bool) -> Core {
    let mut core = Core::new();
    core.dispatch(Command::Restore {
        settings_json: String::new(),
        listening_json: format!(
            r#"{{"version":1,"deviceTotalMs":{unsynced_ms},"syncedThroughMs":0,"trackingEnabled":{tracking},"deviceId":"{DEVICE_A}"}}"#
        ),
        fallback_device_id: DEVICE_B.into(),
        account_json: SIGNED_IN.into(),
    });
    core.dispatch(Command::Play);
    core.dispatch(Command::PlatformPlaybackStarted);
    core
}

/// The `PushListening` payload in `effects`, if any.
fn pushed(effects: &[Effect]) -> Option<(String, u64)> {
    effects.iter().find_map(|e| match e {
        Effect::PushListening {
            device_id,
            device_total_ms,
            ..
        } => Some((device_id.clone(), *device_total_ms)),
        _ => None,
    })
}

fn pushes(effects: &[Effect]) -> Vec<&Effect> {
    effects
        .iter()
        .filter(|e| matches!(e, Effect::PushListening { .. }))
        .collect()
}

fn begin(core: &mut Core, reason: SyncReason) -> Option<(String, u64)> {
    pushed(
        &core
            .dispatch(Command::BeginListeningSync { reason })
            .effects,
    )
}

fn tick(core: &mut Core, elapsed_ms: u64) -> Option<(String, u64)> {
    pushed(&core.dispatch(Command::Tick { elapsed_ms }).effects)
}

fn fail(core: &mut Core) {
    core.dispatch(Command::ListeningSyncFailed {
        unauthorized: false,
    });
}

/// The `PersistListening` JSON in `effects`, if any.
fn persisted_json(effects: &[Effect]) -> Option<String> {
    effects.iter().find_map(|e| match e {
        Effect::PersistListening { json } => Some(json.clone()),
        _ => None,
    })
}

// ---- threshold, from the tick ----------------------------------------------

#[test]
fn the_tick_that_reaches_thirty_seconds_pushes_once_with_the_session_token() {
    let mut core = signed_in_and_playing();
    let below = listen(&mut core, LISTENING_SYNC_THRESHOLD_MS - 1_000);
    assert!(pushes(&below).is_empty(), "{below:?}");

    let effects = core.dispatch(Command::Tick { elapsed_ms: 1_000 }).effects;
    assert_eq!(
        pushes(&effects),
        vec![&Effect::PushListening {
            device_id: DEVICE_A.into(),
            device_total_ms: LISTENING_SYNC_THRESHOLD_MS,
            session_token: "session".into(),
        }]
    );
}

#[test]
fn a_signed_out_core_never_pushes_from_a_tick() {
    let mut core = Core::new();
    core.dispatch(Command::Restore {
        settings_json: String::new(),
        listening_json: String::new(),
        fallback_device_id: DEVICE_A.into(),
        account_json: String::new(),
    });
    core.dispatch(Command::Play);
    core.dispatch(Command::PlatformPlaybackStarted);
    let effects = listen(&mut core, 3 * LISTENING_SYNC_THRESHOLD_MS);
    assert!(pushes(&effects).is_empty(), "{effects:?}");
}

#[test]
fn a_muted_tick_never_pushes() {
    let mut core = restored_with_unsynced(40_000, true);
    core.dispatch(Command::ToggleMute);
    assert_eq!(tick(&mut core, 1_000), None);
}

#[test]
fn an_untracked_tick_never_pushes() {
    let mut core = restored_with_unsynced(40_000, false);
    assert_eq!(tick(&mut core, 1_000), None);
}

#[test]
fn a_push_in_flight_blocks_every_tick_until_it_settles() {
    let mut core = core_with_listening(LISTENING_SYNC_THRESHOLD_MS);
    let during = listen(&mut core, LISTENING_SYNC_THRESHOLD_MS);
    assert!(pushes(&during).is_empty(), "{during:?}");

    core.dispatch(Command::ListeningSyncSucceeded {
        server_total_ms: 1_000_000,
    });
    // 30s unsynced was waiting behind the push; the next counted tick sends it.
    assert_eq!(
        tick(&mut core, 1_000),
        Some((DEVICE_A.into(), 2 * LISTENING_SYNC_THRESHOLD_MS + 1_000))
    );
}

#[test]
fn after_a_failure_no_tick_pushes_until_another_thirty_seconds() {
    let mut core = core_with_listening(LISTENING_SYNC_THRESHOLD_MS);
    fail(&mut core);

    let paced = listen(&mut core, LISTENING_SYNC_THRESHOLD_MS - 1_000);
    assert!(pushes(&paced).is_empty(), "{paced:?}");
    assert_eq!(
        tick(&mut core, 1_000),
        Some((DEVICE_A.into(), 2 * LISTENING_SYNC_THRESHOLD_MS))
    );
}

#[test]
fn flush_and_refresh_still_send_right_after_a_failure() {
    let mut core = core_with_listening(LISTENING_SYNC_THRESHOLD_MS);
    fail(&mut core);
    assert_eq!(
        begin(&mut core, SyncReason::Flush),
        Some((DEVICE_A.into(), LISTENING_SYNC_THRESHOLD_MS))
    );
    fail(&mut core);
    assert_eq!(
        begin(&mut core, SyncReason::Refresh),
        Some((DEVICE_A.into(), LISTENING_SYNC_THRESHOLD_MS))
    );
}

#[test]
fn a_successful_settle_rearms_the_normal_threshold() {
    let mut core = core_with_listening(LISTENING_SYNC_THRESHOLD_MS);
    fail(&mut core);
    listen(&mut core, 5_000);
    assert!(begin(&mut core, SyncReason::Flush).is_some());
    core.dispatch(Command::ListeningSyncSucceeded {
        server_total_ms: 1_000_000,
    });

    // Synced through 35s: the next push comes 30s after that.
    let below = listen(&mut core, LISTENING_SYNC_THRESHOLD_MS - 1_000);
    assert!(pushes(&below).is_empty(), "{below:?}");
    assert_eq!(
        tick(&mut core, 1_000),
        Some((DEVICE_A.into(), 5_000 + 2 * LISTENING_SYNC_THRESHOLD_MS))
    );
}

#[test]
fn a_reset_clears_the_failure_pacing() {
    let mut core = core_with_listening(LISTENING_SYNC_THRESHOLD_MS);
    fail(&mut core);
    core.dispatch(Command::ResetListeningData {
        new_device_id: DEVICE_B.into(),
    });
    core.dispatch(Command::Play);
    core.dispatch(Command::PlatformPlaybackStarted);

    // The new slot's first 30s pushes, not 30s past the old slot's failure.
    let effects = listen(&mut core, LISTENING_SYNC_THRESHOLD_MS);
    assert_eq!(
        pushed(&effects),
        Some((DEVICE_B.into(), LISTENING_SYNC_THRESHOLD_MS))
    );
}

#[test]
fn signing_out_clears_the_failure_pacing() {
    let mut core = core_with_listening(LISTENING_SYNC_THRESHOLD_MS);
    fail(&mut core);
    // A flush goes out, then the user signs out and back in. The sign-in's
    // refresh waits behind the superseded flush, which then fails.
    listen(&mut core, 1_000);
    assert!(begin(&mut core, SyncReason::Flush).is_some());
    core.dispatch(Command::SignOut);
    core.dispatch(Command::SubmitSignInLink {
        input: "raw-token".into(),
    });
    let signed_in = core.dispatch(Command::SignInVerified {
        session_token: "session-2".into(),
        email: "a@example.com".into(),
    });
    assert_eq!(pushed(&signed_in.effects), None);
    fail(&mut core);

    // Nothing has synced, so the threshold counts from zero again.
    assert_eq!(
        tick(&mut core, 1_000),
        Some((DEVICE_A.into(), LISTENING_SYNC_THRESHOLD_MS + 2_000))
    );
}

#[test]
fn the_failure_pacing_is_never_persisted() {
    let mut core = core_with_listening(LISTENING_SYNC_THRESHOLD_MS);
    fail(&mut core);
    let json = persisted_json(&core.dispatch(Command::Tick { elapsed_ms: 1_000 }).effects)
        .expect("a counted tick persists the ledger");

    let mut restarted = Core::new();
    restarted.dispatch(Command::Restore {
        settings_json: String::new(),
        listening_json: json,
        fallback_device_id: "unused".into(),
        account_json: SIGNED_IN.into(),
    });
    restarted.dispatch(Command::Play);
    restarted.dispatch(Command::PlatformPlaybackStarted);
    assert_eq!(
        tick(&mut restarted, 1_000),
        Some((DEVICE_A.into(), LISTENING_SYNC_THRESHOLD_MS + 2_000))
    );
}

// ---- reasons a shell sends ---------------------------------------------------

#[test]
fn a_threshold_request_from_a_shell_still_waits_for_thirty_seconds() {
    let mut below = restored_with_unsynced(LISTENING_SYNC_THRESHOLD_MS - 1, true);
    assert_eq!(begin(&mut below, SyncReason::Threshold), None);

    let mut at = restored_with_unsynced(LISTENING_SYNC_THRESHOLD_MS, true);
    assert_eq!(
        begin(&mut at, SyncReason::Threshold),
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
fn synced_through_is_exactly_what_was_sent() {
    let mut core = signed_in_and_playing();
    let (device_id, sent) =
        pushed(&listen(&mut core, LISTENING_SYNC_THRESHOLD_MS)).expect("the threshold tick pushes");
    assert_eq!(device_id, DEVICE_A);
    assert_eq!(sent, LISTENING_SYNC_THRESHOLD_MS);

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

#[test]
fn a_sync_is_not_reentrant_until_the_first_settles() {
    let mut core = core_with_listening(LISTENING_SYNC_THRESHOLD_MS);
    assert_eq!(
        begin(&mut core, SyncReason::Refresh),
        None,
        "one PUT in flight at a time"
    );

    fail(&mut core);
    assert!(
        begin(&mut core, SyncReason::Refresh).is_some(),
        "a failure frees the slot for the next trigger"
    );
}

#[test]
fn a_transient_failure_acknowledges_nothing() {
    let mut core = core_with_listening(LISTENING_SYNC_THRESHOLD_MS);
    let update = core.dispatch(Command::ListeningSyncFailed {
        unauthorized: false,
    });
    assert_eq!(
        update.snapshot.listening.unsynced_ms,
        LISTENING_SYNC_THRESHOLD_MS
    );
    assert!(update.effects.is_empty(), "{:?}", update.effects);
}

#[test]
fn an_ack_with_nothing_in_flight_is_ignored() {
    let mut core = core_with_listening(20_000);
    let snap = core
        .dispatch(Command::ListeningSyncSucceeded {
            server_total_ms: 9_000_000,
        })
        .snapshot;
    assert_eq!(snap.listening.unsynced_ms, 20_000);
    assert_eq!(snap.listening.displayed_total_ms, 20_000);
}

#[test]
fn a_successful_sync_persists_the_new_high_water_mark() {
    let mut core = core_with_listening(LISTENING_SYNC_THRESHOLD_MS);
    let effects = core
        .dispatch(Command::ListeningSyncSucceeded {
            server_total_ms: 50_000,
        })
        .effects;
    let json = persisted_json(&effects).expect("ack persists the ledger");
    assert!(json.contains(r#""syncedThroughMs":30000"#), "{json}");
}

// ---- 401 rule --------------------------------------------------------------

#[test]
fn unauthorized_signs_out_and_keeps_local_listening() {
    let mut core = core_with_listening(LISTENING_SYNC_THRESHOLD_MS);
    let update = core.dispatch(Command::ListeningSyncFailed { unauthorized: true });
    assert_eq!(update.snapshot.account.email, None);
    assert_eq!(
        update.snapshot.listening.device_total_ms,
        LISTENING_SYNC_THRESHOLD_MS
    );
    assert_eq!(
        update.snapshot.listening.unsynced_ms,
        LISTENING_SYNC_THRESHOLD_MS
    );
}

#[test]
fn a_401_for_a_sync_superseded_by_sign_out_is_dropped() {
    let mut core = core_with_listening(LISTENING_SYNC_THRESHOLD_MS);
    core.dispatch(Command::SignOut);
    let status = core.snapshot().account.status_label;
    let update = core.dispatch(Command::ListeningSyncFailed { unauthorized: true });
    assert!(update.effects.is_empty(), "{:?}", update.effects);
    assert_eq!(update.snapshot.account.status_label, status);
    assert!(
        begin(&mut core, SyncReason::Refresh).is_none(),
        "signed out, so nothing to send"
    );
}

// ---- device-id lifecycle ---------------------------------------------------

#[test]
fn restore_without_a_stored_id_adopts_and_persists_the_fallback() {
    let mut core = Core::new();
    let effects = core
        .dispatch(Command::Restore {
            settings_json: String::new(),
            listening_json: String::new(),
            fallback_device_id: DEVICE_A.into(),
            account_json: SIGNED_IN.into(),
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
        .dispatch(Command::Restore {
            settings_json: String::new(),
            listening_json: blob,
            fallback_device_id: DEVICE_B.into(),
            account_json: SIGNED_IN.into(),
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
    core.dispatch(Command::Restore {
        settings_json: String::new(),
        listening_json: legacy.into(),
        fallback_device_id: DEVICE_B.into(),
        account_json: SIGNED_IN.into(),
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
    restarted.dispatch(Command::Restore {
        settings_json: String::new(),
        listening_json: json,
        fallback_device_id: "unused".into(),
        account_json: SIGNED_IN.into(),
    });
    assert_eq!(
        begin(&mut restarted, SyncReason::Refresh),
        Some((DEVICE_B.into(), 0))
    );
}

#[test]
fn an_ack_for_the_old_slot_is_dropped_after_a_reset() {
    let mut core = core_with_listening(LISTENING_SYNC_THRESHOLD_MS);
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
        "the old slot's 30s must not be credited to the new one"
    );
    assert_eq!(snap.listening.displayed_total_ms, 1_000);
}

#[test]
fn a_new_sync_waits_for_the_old_slots_put_to_settle_after_a_reset() {
    let mut core = core_with_listening(LISTENING_SYNC_THRESHOLD_MS);
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

// ---- wire shape ------------------------------------------------------------
//
// Every shell hand-writes these JSON shapes. Lock them.

#[test]
fn sync_commands_accept_the_shells_camel_case_json() {
    let cases = [
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
        session_token: "s".into(),
    };
    assert_eq!(
        serde_json::to_string(&push).unwrap(),
        r#"{"type":"pushListening","deviceId":"d","deviceTotalMs":7,"sessionToken":"s"}"#
    );
}
