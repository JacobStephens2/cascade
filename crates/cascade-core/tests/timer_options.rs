//! Timer choices and timer status, driven only through `Core::dispatch`.
//!
//! The core decides which timer lengths a user is offered, the custom-duration
//! limits, what the custom field opens on, whether a timer is active and how
//! the timer reads as one status line, including how long a finished timer's
//! message stays. Every shell renders these verbatim, so these tests pin the
//! rules on the snapshot.

use cascade_core::{
    Command, Core, Effect, HttpMethod, PersistedSettings, Snapshot, SyncReason, TimerSnapshotKind,
    Update, LISTENING_SYNC_THRESHOLD_MS,
};

fn core_after(commands: impl IntoIterator<Item = Command>) -> Core {
    let mut core = Core::new();
    for command in commands {
        core.dispatch(command);
    }
    core
}

fn snapshot_after(commands: impl IntoIterator<Item = Command>) -> Snapshot {
    core_after(commands).snapshot()
}

/// The settings blob on the `PersistSettings` effect in `update`, if any.
fn persisted(update: &Update) -> Option<PersistedSettings> {
    update.effects.iter().find_map(|e| match e {
        Effect::PersistSettings { json } => Some(serde_json::from_str(json).unwrap()),
        _ => None,
    })
}

fn restore_with(settings: PersistedSettings) -> Command {
    Command::Restore {
        settings_json: serde_json::to_string(&settings).unwrap(),
        listening_json: String::new(),
        fallback_device_id: "device".into(),
        account_json: String::new(),
    }
}

fn settings(sleep: Option<u32>, pomodoro: Option<u32>) -> PersistedSettings {
    PersistedSettings {
        version: cascade_core::SETTINGS_VERSION,
        volume_percent: 60,
        default_sleep_minutes: sleep,
        default_pomodoro_minutes: pomodoro,
    }
}

// ---- clamp ----------------------------------------------------------------

#[test]
fn start_commands_clamp_minutes_into_the_limits() {
    for (asked, ran) in [
        (0u32, 1u32),
        (1, 1),
        (1440, 1440),
        (1441, 1440),
        (u32::MAX, 1440),
    ] {
        let sleep = snapshot_after([Command::StartSleepTimer { minutes: asked }]);
        assert_eq!(
            sleep.timer.total_ms,
            u64::from(ran) * 60_000,
            "sleep {asked}"
        );
        let focus = snapshot_after([Command::StartPomodoro { minutes: asked }]);
        assert_eq!(
            focus.timer.total_ms,
            u64::from(ran) * 60_000,
            "focus {asked}"
        );
    }
}

#[test]
fn zero_minutes_starts_a_one_minute_timer_instead_of_cancelling() {
    let mut core = core_after([Command::StartSleepTimer { minutes: 10 }]);
    let update = core.dispatch(Command::StartSleepTimer { minutes: 0 });
    assert!(update.snapshot.timer.is_active);
    assert_eq!(update.snapshot.timer.total_ms, 60_000);
}

#[test]
fn the_saved_default_is_the_clamped_value() {
    let mut core = Core::new();
    let sleep = core.dispatch(Command::StartSleepTimer { minutes: 5_000 });
    assert_eq!(persisted(&sleep).unwrap().default_sleep_minutes, Some(1440));
    let focus = core.dispatch(Command::StartPomodoro { minutes: 0 });
    assert_eq!(persisted(&focus).unwrap().default_pomodoro_minutes, Some(1));
}

// ---- timer options --------------------------------------------------------

#[test]
fn options_carry_the_presets_and_limits() {
    let options = Core::new().snapshot().timer_options;
    let presets = |list: &[cascade_core::TimerPreset]| -> Vec<(u32, String, String)> {
        list.iter()
            .map(|p| (p.minutes, p.label.clone(), p.short_label.clone()))
            .collect()
    };
    let p = |m: u32, l: &str, s: &str| (m, l.to_string(), s.to_string());
    assert_eq!(
        presets(&options.focus_presets),
        vec![
            p(30, "30 min", "30m"),
            p(60, "1 hr", "1h"),
            p(480, "8 hr", "8h")
        ]
    );
    assert_eq!(
        presets(&options.sleep_presets),
        vec![
            p(15, "15 min", "15m"),
            p(30, "30 min", "30m"),
            p(60, "1 hr", "1h")
        ]
    );
    assert_eq!(options.min_minutes, 1);
    assert_eq!(options.max_minutes, 1440);
}

#[test]
fn custom_values_start_at_thirty_on_a_fresh_install() {
    let options = Core::new().snapshot().timer_options;
    assert_eq!(options.custom_focus_minutes, 30);
    assert_eq!(options.custom_sleep_minutes, 30);
}

#[test]
fn custom_values_follow_the_last_started_duration_of_each_kind() {
    let options = snapshot_after([
        Command::StartPomodoro { minutes: 45 },
        Command::StartSleepTimer { minutes: 15 },
        Command::CancelTimer,
    ])
    .timer_options;
    assert_eq!(options.custom_focus_minutes, 45);
    assert_eq!(options.custom_sleep_minutes, 15);

    let clamped = snapshot_after([Command::StartPomodoro { minutes: 9_999 }]).timer_options;
    assert_eq!(clamped.custom_focus_minutes, 1440);
}

#[test]
fn custom_values_come_from_restored_settings_clamped_with_fallback() {
    let restored = snapshot_after([restore_with(settings(Some(20), Some(90)))]).timer_options;
    assert_eq!(restored.custom_sleep_minutes, 20);
    assert_eq!(restored.custom_focus_minutes, 90);

    let missing = snapshot_after([restore_with(settings(None, None))]).timer_options;
    assert_eq!(missing.custom_sleep_minutes, 30);
    assert_eq!(missing.custom_focus_minutes, 30);

    let out_of_range =
        snapshot_after([restore_with(settings(Some(0), Some(10_000)))]).timer_options;
    assert_eq!(out_of_range.custom_sleep_minutes, 1);
    assert_eq!(out_of_range.custom_focus_minutes, 1440);
}

// ---- isActive -------------------------------------------------------------

#[test]
fn is_active_for_every_running_timer_kind_only() {
    assert!(!Core::new().snapshot().timer.is_active, "off");
    for start in [
        Command::StartSleepTimer { minutes: 10 },
        Command::StartPomodoro { minutes: 10 },
        Command::StartStopwatch,
    ] {
        assert!(snapshot_after([start.clone()]).timer.is_active, "{start:?}");
    }
    let mut core = core_after([Command::StartSleepTimer { minutes: 1 }]);
    let completed = core.dispatch(Command::Tick { elapsed_ms: 60_000 }).snapshot;
    assert_eq!(
        completed.timer.kind,
        cascade_core::TimerSnapshotKind::JustCompleted
    );
    assert!(!completed.timer.is_active, "just completed");
}

// ---- statusLabel ----------------------------------------------------------

#[test]
fn status_label_with_no_timer() {
    assert_eq!(Core::new().snapshot().timer.status_label, "Paused");
    assert_eq!(
        snapshot_after([Command::Play]).timer.status_label,
        "Playing · no timer"
    );
    assert_eq!(
        snapshot_after([Command::Play, Command::ToggleMute])
            .timer
            .status_label,
        "Muted · no timer"
    );
}

#[test]
fn status_label_for_a_countdown() {
    let tick = Command::Tick { elapsed_ms: 26_000 };
    assert_eq!(
        snapshot_after([Command::StartSleepTimer { minutes: 13 }, tick.clone()])
            .timer
            .status_label,
        "Paused · 12:34 left"
    );
    assert_eq!(
        snapshot_after([Command::StartPomodoro { minutes: 13 }, tick.clone()])
            .timer
            .status_label,
        "Playing · 12:34 left"
    );
    assert_eq!(
        snapshot_after([
            Command::StartPomodoro { minutes: 13 },
            Command::ToggleMute,
            tick
        ])
        .timer
        .status_label,
        "Muted · 12:34 left"
    );
}

#[test]
fn status_label_for_a_stopwatch() {
    assert_eq!(
        snapshot_after([
            Command::StartStopwatch,
            Command::Tick {
                elapsed_ms: 754_000
            }
        ])
        .timer
        .status_label,
        "Stopwatch · 12:34"
    );
}

#[test]
fn status_label_when_just_completed_is_the_completion_text() {
    let mut sleep = core_after([Command::StartSleepTimer { minutes: 1 }]);
    let snap = sleep
        .dispatch(Command::Tick { elapsed_ms: 60_000 })
        .snapshot;
    assert_eq!(snap.timer.status_label, "Sleep timer ended");

    let mut focus = core_after([Command::StartPomodoro { minutes: 1 }]);
    let snap = focus
        .dispatch(Command::Tick { elapsed_ms: 60_000 })
        .snapshot;
    assert_eq!(snap.timer.status_label, "Session complete");
}

// ---- completion message ---------------------------------------------------

const SLEEP_TIMER_ENDED: &str = "Sleep timer ended";
/// A stored account, so the core holds a session and may sync.
const SIGNED_IN: &str = r#"{"version":1,"sessionToken":"t","email":"a@example.com"}"#;

/// A core whose one-minute sleep timer has just run out while playing.
fn sleep_timer_ended() -> Core {
    let mut core = core_after([
        Command::Play,
        Command::StartSleepTimer { minutes: 1 },
        Command::Tick { elapsed_ms: 60_000 },
    ]);
    assert_eq!(core.snapshot().timer.status_label, SLEEP_TIMER_ENDED);
    // A shell still ticking after expiry must not clear it either.
    core.dispatch(Command::Tick { elapsed_ms: 250 });
    core
}

#[test]
fn the_completion_message_survives_everything_the_user_did_not_ask_for() {
    let mut core = sleep_timer_ended();
    let background = [
        Command::Tick { elapsed_ms: 1_000 },
        Command::BeginListeningSync {
            reason: SyncReason::Flush,
        },
        Command::BeginListeningSync {
            reason: SyncReason::Refresh,
        },
        // Responses the core is not waiting for.
        Command::ServerResponse {
            id: 1,
            status: 200,
            body: r#"{"serverTotalMs":0}"#.into(),
        },
        Command::ServerResponse {
            id: 2,
            status: 0,
            body: String::new(),
        },
        Command::ServerResponse {
            id: 3,
            status: 401,
            body: String::new(),
        },
        Command::PlatformPlaybackStarted,
        Command::PlatformPlaybackPaused,
        Command::SetVolume { percent: 30 },
        Command::ToggleMute,
    ];
    for command in background {
        let snap = core.dispatch(command.clone()).snapshot;
        assert_eq!(
            snap.timer.kind,
            TimerSnapshotKind::JustCompleted,
            "{command:?}"
        );
        assert_eq!(snap.timer.status_label, SLEEP_TIMER_ENDED, "{command:?}");
    }
}

/// One step of a shell's traffic: a command, or the response to the last
/// request the core sent.
enum Step {
    Send(Command),
    Answer(u16, &'static str),
}

#[test]
fn the_completion_message_survives_sync_and_account_traffic_that_lands() {
    // Signed in, so each request really goes out and each response really
    // lands on a request the core is waiting for.
    let mut core = Core::new();
    core.dispatch(Command::Restore {
        settings_json: String::new(),
        listening_json: String::new(),
        fallback_device_id: "device".into(),
        account_json: SIGNED_IN.into(),
    });
    core.dispatch(Command::Play);
    core.dispatch(Command::StartSleepTimer { minutes: 1 });
    core.dispatch(Command::Tick { elapsed_ms: 60_000 });
    let refresh = || {
        Step::Send(Command::BeginListeningSync {
            reason: SyncReason::Refresh,
        })
    };
    let pushed = r#"{"serverTotalMs":0}"#;
    let verified = r#"{"sessionToken":"t2","email":"a@example.com"}"#;
    let sign_in = || {
        [
            Step::Send(Command::SubmitSignInLink {
                input: "raw-token".into(),
            }),
            Step::Answer(200, verified),
        ]
    };
    let delete_listening = |id: &str| {
        Step::Send(Command::DeleteListeningData {
            new_device_id: id.into(),
        })
    };
    let traffic = [
        refresh(),
        Step::Answer(200, pushed),
        refresh(),
        Step::Answer(500, ""),
        delete_listening("d2"),
        Step::Answer(204, ""),
        delete_listening("d3"),
        Step::Answer(0, ""),
        Step::Send(Command::DeleteAccount {
            new_device_id: "d4".into(),
        }),
        Step::Answer(204, ""),
        Step::Send(Command::RequestSignInLink {
            email: "a@example.com".into(),
            platform: None,
        }),
        Step::Answer(204, ""),
    ]
    .into_iter()
    // The sign-in's refresh push is the last request: a 401 settles it.
    .chain(sign_in())
    .chain([Step::Answer(401, "")])
    .chain(sign_in())
    .chain([
        Step::Answer(200, pushed),
        delete_listening("d5"),
        Step::Answer(401, ""),
    ])
    .chain(sign_in())
    .chain([Step::Send(Command::SignOut), Step::Answer(204, "")]);

    let mut last_request = None;
    for step in traffic {
        let command = match step {
            Step::Send(command) => command,
            Step::Answer(status, body) => Command::ServerResponse {
                id: last_request.expect("a request to answer"),
                status,
                body: body.into(),
            },
        };
        let update = core.dispatch(command.clone());
        for effect in &update.effects {
            if let Effect::ServerRequest { id, .. } = effect {
                last_request = Some(*id);
            }
        }
        assert_eq!(
            update.snapshot.timer.status_label, SLEEP_TIMER_ENDED,
            "{command:?}"
        );
    }
    // The traffic really landed: the last sign-in, then the sign-out.
    assert_eq!(core.snapshot().account.email, None);
}

#[test]
fn each_playback_and_timer_command_clears_the_completion_message() {
    for command in [
        Command::Play,
        Command::Pause,
        Command::TogglePlayback,
        Command::StartSleepTimer { minutes: 5 },
        Command::StartPomodoro { minutes: 5 },
        Command::StartStopwatch,
        Command::CancelTimer,
    ] {
        let mut core = sleep_timer_ended();
        let snap = core.dispatch(command.clone()).snapshot;
        assert_ne!(
            snap.timer.kind,
            TimerSnapshotKind::JustCompleted,
            "{command:?}"
        );
        assert_ne!(snap.timer.status_label, SLEEP_TIMER_ENDED, "{command:?}");
    }
}

#[test]
fn the_tick_that_ends_a_timer_and_crosses_the_threshold_does_both() {
    let mut core = Core::new();
    core.dispatch(Command::Restore {
        settings_json: String::new(),
        listening_json: String::new(),
        fallback_device_id: "device".into(),
        account_json: SIGNED_IN.into(),
    });
    core.dispatch(Command::Play);
    core.dispatch(Command::PlatformPlaybackStarted);
    for _ in 0..25 {
        core.dispatch(Command::Tick { elapsed_ms: 1_000 });
    }
    // Paused while the timer runs down, so the expiry tick is also the one
    // that brings listening to 30 seconds.
    core.dispatch(Command::StartSleepTimer { minutes: 1 });
    core.dispatch(Command::Pause);
    core.dispatch(Command::Tick { elapsed_ms: 55_000 });
    core.dispatch(Command::Play);
    core.dispatch(Command::PlatformPlaybackStarted);

    let update = core.dispatch(Command::Tick { elapsed_ms: 5_000 });
    assert_eq!(update.snapshot.timer.kind, TimerSnapshotKind::JustCompleted);
    assert_eq!(update.snapshot.timer.status_label, SLEEP_TIMER_ENDED);
    assert!(
        update.effects.contains(&Effect::ServerRequest {
            id: 1,
            method: HttpMethod::Put,
            path: "/listening".into(),
            bearer_token: Some("t".into()),
            body: Some(format!(
                r#"{{"deviceId":"device","deviceTotalMs":{LISTENING_SYNC_THRESHOLD_MS}}}"#
            )),
        }),
        "{:?}",
        update.effects
    );
}

// ---- wire shape -----------------------------------------------------------

// Every shell decodes these fields from the JSON snapshot by name; a rename
// would silently drop the presets or flip every "is a timer running" check.
#[test]
fn timer_fields_wire_shape() {
    let snap = snapshot_after([Command::StartSleepTimer { minutes: 15 }]);
    let json = serde_json::to_value(&snap).unwrap();
    assert_eq!(
        json["timerOptions"],
        serde_json::json!({
            "focusPresets": [
                { "minutes": 30, "label": "30 min", "shortLabel": "30m" },
                { "minutes": 60, "label": "1 hr", "shortLabel": "1h" },
                { "minutes": 480, "label": "8 hr", "shortLabel": "8h" }
            ],
            "sleepPresets": [
                { "minutes": 15, "label": "15 min", "shortLabel": "15m" },
                { "minutes": 30, "label": "30 min", "shortLabel": "30m" },
                { "minutes": 60, "label": "1 hr", "shortLabel": "1h" }
            ],
            "minMinutes": 1,
            "maxMinutes": 1440,
            "customFocusMinutes": 30,
            "customSleepMinutes": 15
        })
    );
    assert_eq!(json["timer"]["isActive"], serde_json::json!(true));
    assert_eq!(
        json["timer"]["statusLabel"],
        serde_json::json!("Paused · 15:00 left")
    );
}
