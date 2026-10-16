//! Boot-time restore, driven only through `Core::dispatch`.
//!
//! Every shell boots the same way: a fresh core, one `restore` command carrying
//! both persisted blobs and a fallback device id, then the resulting update.
//! These tests pin what that one command does with each blob.

use cascade_core::{Command, Core, Effect, SyncReason, SETTINGS_VERSION};

const DEVICE_A: &str = "device-a";
const DEVICE_B: &str = "device-b";
/// A stored account, so the core holds a session and its syncs go out.
const SIGNED_IN: &str = r#"{"version":1,"sessionToken":"t","email":"a@example.com"}"#;

fn restore(settings_json: &str, listening_json: &str, fallback_device_id: &str) -> Command {
    Command::Restore {
        settings_json: settings_json.into(),
        listening_json: listening_json.into(),
        fallback_device_id: fallback_device_id.into(),
        account_json: SIGNED_IN.into(),
    }
}

fn settings_json(volume_percent: u8, sleep: Option<u32>, pomodoro: Option<u32>) -> String {
    serde_json::to_string(&cascade_core::PersistedSettings {
        version: SETTINGS_VERSION,
        volume_percent,
        default_sleep_minutes: sleep,
        default_pomodoro_minutes: pomodoro,
    })
    .unwrap()
}

/// The `PersistListening` JSON in `effects`, if any.
fn persisted_listening(effects: &[Effect]) -> Option<String> {
    effects.iter().find_map(|e| match e {
        Effect::PersistListening { json } => Some(json.clone()),
        _ => None,
    })
}

fn pushed_device_id(core: &mut Core) -> Option<String> {
    core.dispatch(Command::BeginListeningSync {
        reason: SyncReason::Refresh,
    })
    .effects
    .into_iter()
    .find_map(|e| match e {
        Effect::ServerRequest {
            body: Some(body), ..
        } => serde_json::from_str::<serde_json::Value>(&body).unwrap()["deviceId"]
            .as_str()
            .map(String::from),
        _ => None,
    })
}

// ---- settings --------------------------------------------------------------

#[test]
fn restore_applies_the_persisted_settings() {
    let mut core = Core::new();
    let update = core.dispatch(restore(
        &settings_json(42, Some(45), Some(90)),
        "",
        DEVICE_A,
    ));
    assert_eq!(update.snapshot.volume_percent, 42);
    assert_eq!(core.state().default_sleep_minutes, Some(45));
    assert_eq!(core.state().default_pomodoro_minutes, Some(90));
}

#[test]
fn restore_emits_no_settings_or_playback_effects() {
    // The core is paused at boot: nothing to persist back, nothing to play.
    let mut core = Core::new();
    let effects = core
        .dispatch(restore(&settings_json(42, None, None), "", DEVICE_A))
        .effects;
    assert!(
        effects
            .iter()
            .all(|e| matches!(e, Effect::PersistListening { .. })),
        "{effects:?}"
    );
    assert!(!core.snapshot().is_playing);
}

#[test]
fn restore_clamps_an_out_of_range_volume() {
    let mut core = Core::new();
    let snap = core
        .dispatch(restore(&settings_json(250, None, None), "", DEVICE_A))
        .snapshot;
    assert_eq!(snap.volume_percent, 100);
}

#[test]
fn a_missing_settings_blob_keeps_the_defaults() {
    let fresh = Core::new();
    let mut core = Core::new();
    core.dispatch(restore("", "", DEVICE_A));
    assert_eq!(
        core.snapshot().volume_percent,
        fresh.snapshot().volume_percent
    );
    assert_eq!(
        core.state().default_sleep_minutes,
        fresh.state().default_sleep_minutes
    );
    assert_eq!(
        core.state().default_pomodoro_minutes,
        fresh.state().default_pomodoro_minutes
    );
}

#[test]
fn an_unparseable_or_old_version_settings_blob_is_ignored() {
    let fresh = Core::new();
    let wrong_version = format!(
        r#"{{"version":{},"volumePercent":10,"defaultSleepMinutes":5,"defaultPomodoroMinutes":5}}"#,
        SETTINGS_VERSION + 1
    );
    for blob in ["not json", "{}", wrong_version.as_str()] {
        let mut core = Core::new();
        let update = core.dispatch(restore(blob, "", DEVICE_A));
        assert_eq!(
            update.snapshot.volume_percent,
            fresh.snapshot().volume_percent,
            "{blob}"
        );
        assert_eq!(
            core.state().default_sleep_minutes,
            fresh.state().default_sleep_minutes,
            "{blob}"
        );
        // A bad settings blob doesn't stop the listening half from running.
        assert!(persisted_listening(&update.effects).is_some(), "{blob}");
    }
}

#[test]
fn a_bad_settings_blob_does_not_block_the_listening_blob() {
    let listening = r#"{"version":1,"deviceTotalMs":100,"syncedThroughMs":0,"trackingEnabled":true,"deviceId":"stored"}"#;
    let mut core = Core::new();
    let snap = core
        .dispatch(restore("garbage", listening, DEVICE_A))
        .snapshot;
    assert_eq!(snap.listening.device_total_ms, 100);
    assert_eq!(pushed_device_id(&mut core).as_deref(), Some("stored"));
}

// ---- once only -------------------------------------------------------------

#[test]
fn only_the_first_restore_takes_effect() {
    let mut core = Core::new();
    core.dispatch(restore(&settings_json(42, None, None), "", DEVICE_A));

    let update = core.dispatch(restore(&settings_json(7, None, None), "", DEVICE_B));
    assert!(update.effects.is_empty(), "{:?}", update.effects);
    assert_eq!(update.snapshot.volume_percent, 42);
    assert_eq!(pushed_device_id(&mut core).as_deref(), Some(DEVICE_A));
}

#[test]
fn a_stale_blob_replayed_after_a_reset_cannot_resurrect_the_old_slot() {
    // Restore → listen → reset rotates to DEVICE_B. Replaying the pre-reset
    // blob must not point the core back at the deleted DEVICE_A slot.
    let mut core = Core::new();
    core.dispatch(restore("", "", DEVICE_A));
    core.dispatch(Command::Play);
    core.dispatch(Command::PlatformPlaybackStarted);
    core.dispatch(Command::Tick { elapsed_ms: 1_000 });
    let stale = serde_json::to_string(&core.state().listening.to_persisted()).unwrap();

    core.dispatch(Command::ResetListeningData {
        new_device_id: DEVICE_B.into(),
    });
    let update = core.dispatch(restore("", &stale, "unused"));

    assert!(update.effects.is_empty(), "{:?}", update.effects);
    assert_eq!(update.snapshot.listening.device_total_ms, 0);
    assert_eq!(pushed_device_id(&mut core).as_deref(), Some(DEVICE_B));
}

// ---- account ---------------------------------------------------------------

fn restore_account(account_json: &str) -> Command {
    Command::Restore {
        settings_json: String::new(),
        listening_json: String::new(),
        fallback_device_id: DEVICE_A.into(),
        account_json: account_json.into(),
    }
}

fn pushed_session_token(core: &mut Core) -> Option<String> {
    core.dispatch(Command::BeginListeningSync {
        reason: SyncReason::Refresh,
    })
    .effects
    .into_iter()
    .find_map(|e| match e {
        Effect::ServerRequest { bearer_token, .. } => bearer_token,
        _ => None,
    })
}

#[test]
fn restore_signs_in_from_the_versioned_account_blob() {
    let mut core = Core::new();
    let update = core.dispatch(restore_account(
        r#"{"version":1,"sessionToken":"t","email":"a@example.com"}"#,
    ));
    assert_eq!(
        update.snapshot.account.email.as_deref(),
        Some("a@example.com")
    );
    assert!(
        update
            .effects
            .iter()
            .all(|e| matches!(e, Effect::PersistListening { .. })),
        "restore re-persists no account: {:?}",
        update.effects
    );
    assert_eq!(pushed_session_token(&mut core), Some("t".into()));
}

#[test]
fn restore_accepts_the_version_less_legacy_account_blob() {
    // What web and Apple stored before the core held the account.
    let mut core = Core::new();
    let snap = core
        .dispatch(restore_account(
            r#"{"sessionToken":"t","email":"a@example.com"}"#,
        ))
        .snapshot;
    assert_eq!(snap.account.email.as_deref(), Some("a@example.com"));
    assert_eq!(pushed_session_token(&mut core), Some("t".into()));
}

#[test]
fn an_empty_or_garbage_account_blob_means_no_account() {
    for blob in [
        "",
        "not json",
        "{}",
        r#"{"version":2,"sessionToken":"t","email":"a@example.com"}"#,
        r#"{"version":1,"sessionToken":"","email":"a@example.com"}"#,
    ] {
        let mut core = Core::new();
        let snap = core.dispatch(restore_account(blob)).snapshot;
        assert_eq!(snap.account.email, None, "{blob}");
        assert_eq!(pushed_session_token(&mut core), None, "{blob}");
    }
}

#[test]
fn an_older_restore_without_account_json_still_restores() {
    let json = r#"{"type":"restore","settingsJson":"","listeningJson":"","fallbackDeviceId":"d"}"#;
    let command: Command = serde_json::from_str(json).unwrap();
    let mut core = Core::new();
    let snap = core.dispatch(command).snapshot;
    assert_eq!(snap.account.email, None);
    // A missing `accountJson` is an empty one: signed out, so nothing syncs.
    assert_eq!(pushed_session_token(&mut core), None);
}

#[test]
fn restoring_signed_out_forgets_a_leftover_cross_device_total() {
    // A device signed out before the core held the account kept the old
    // account's total in its listening blob.
    let listening = r#"{"version":1,"deviceTotalMs":100,"syncedThroughMs":100,"serverTotalMs":9000000,"trackingEnabled":true,"deviceId":"stored"}"#;
    let mut core = Core::new();
    let update = core.dispatch(Command::Restore {
        settings_json: String::new(),
        listening_json: listening.into(),
        fallback_device_id: DEVICE_A.into(),
        account_json: String::new(),
    });
    assert_eq!(update.snapshot.listening.displayed_total_ms, 100);
    let json = persisted_listening(&update.effects).expect("the forgotten total is persisted");
    assert!(!json.contains("9000000"), "{json}");
}

// ---- wire shape ------------------------------------------------------------

#[test]
fn restore_accepts_the_shells_camel_case_json() {
    let json =
        r#"{"type":"restore","settingsJson":"s","listeningJson":"l","fallbackDeviceId":"d"}"#;
    let parsed: Command = serde_json::from_str(json).unwrap();
    assert_eq!(
        parsed,
        Command::Restore {
            settings_json: "s".into(),
            listening_json: "l".into(),
            fallback_device_id: "d".into(),
            account_json: String::new(),
        }
    );
}

#[test]
fn restore_accepts_account_json() {
    let json = r#"{"type":"restore","settingsJson":"s","listeningJson":"l","fallbackDeviceId":"d","accountJson":"a"}"#;
    let parsed: Command = serde_json::from_str(json).unwrap();
    assert_eq!(
        parsed,
        Command::Restore {
            settings_json: "s".into(),
            listening_json: "l".into(),
            fallback_device_id: "d".into(),
            account_json: "a".into(),
        }
    );
}
