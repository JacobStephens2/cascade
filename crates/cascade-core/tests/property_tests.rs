//! Property tests for the shared core.
//!
//! Unit tests pin specific transitions; these assert invariants that must hold
//! for *any* input — the cheap insurance the architecture brief asked for,
//! especially now that custom durations let the user pick any minute count.

use cascade_core::{Command, Core, Effect, TimerSnapshotKind};
use proptest::prelude::*;

const MAX_VOLUME: u8 = 100;
const MS_PER_MIN: u64 = 60_000;

/// A strategy that generates any command the platforms can dispatch, with
/// ranges that comfortably exceed real usage (volume past 100, timers past a
/// day, ticks up to an hour) so clamping/saturation is exercised.
fn command_strategy() -> impl Strategy<Value = Command> {
    prop_oneof![
        Just(Command::Play),
        Just(Command::Pause),
        Just(Command::TogglePlayback),
        any::<u8>().prop_map(|percent| Command::SetVolume { percent }),
        Just(Command::ToggleMute),
        (0u32..=2_000).prop_map(|minutes| Command::StartSleepTimer { minutes }),
        (0u32..=2_000).prop_map(|minutes| Command::StartPomodoro { minutes }),
        Just(Command::CancelTimer),
        (0u64..=3_600_000).prop_map(|elapsed_ms| Command::Tick { elapsed_ms }),
        Just(Command::PlatformPlaybackStarted),
        Just(Command::PlatformPlaybackPaused),
    ]
}

proptest! {
    /// Volume is always a valid percentage, and the emitted platform-volume
    /// effect carries the same clamped value the snapshot reports.
    #[test]
    fn volume_always_clamped(percent in any::<u8>()) {
        let mut core = Core::new();
        let update = core.dispatch(Command::SetVolume { percent });
        let clamped = percent.min(MAX_VOLUME);
        prop_assert_eq!(update.snapshot.volume_percent, clamped);
        let emitted = update.effects.iter().find_map(|e| match e {
            Effect::SetPlatformVolume { volume_percent } => Some(*volume_percent),
            _ => None,
        });
        prop_assert_eq!(emitted, Some(clamped));
    }

    /// Any custom duration sets total = minutes * 60_000 ms, for both flavors.
    #[test]
    fn custom_duration_sets_expected_total(minutes in 1u32..=1_440) {
        let mut pomo = Core::new();
        let u = pomo.dispatch(Command::StartPomodoro { minutes });
        prop_assert_eq!(u.snapshot.timer.total_ms, minutes as u64 * MS_PER_MIN);
        prop_assert_eq!(u.snapshot.timer.kind, TimerSnapshotKind::Pomodoro);

        let mut sleep = Core::new();
        let u = sleep.dispatch(Command::StartSleepTimer { minutes });
        prop_assert_eq!(u.snapshot.timer.total_ms, minutes as u64 * MS_PER_MIN);
        prop_assert_eq!(u.snapshot.timer.kind, TimerSnapshotKind::Sleep);
    }

    /// Ticking never makes the remaining time increase or underflow, and
    /// progress stays a finite fraction in [0, 1].
    #[test]
    fn timer_counts_down_without_underflow(
        minutes in 1u32..=600,
        ticks in prop::collection::vec(0u64..=120_000, 0..40),
    ) {
        let mut core = Core::new();
        core.dispatch(Command::StartSleepTimer { minutes });
        let mut last_remaining = core.snapshot().timer.remaining_ms;
        prop_assert_eq!(last_remaining, minutes as u64 * MS_PER_MIN);

        for delta in ticks {
            let s = core.dispatch(Command::Tick { elapsed_ms: delta }).snapshot;
            prop_assert!(s.timer.progress.is_finite());
            prop_assert!((0.0f32..=1.0).contains(&s.timer.progress));
            if s.timer.kind == TimerSnapshotKind::Sleep {
                prop_assert!(s.timer.remaining_ms <= last_remaining);
                prop_assert!(s.timer.remaining_ms <= s.timer.total_ms);
                last_remaining = s.timer.remaining_ms;
            } else {
                // Timer ended (JustCompleted then Off); stop checking.
                break;
            }
        }
    }

    /// A tick past the full duration ends the session: timer completes and,
    /// if playing, playback is paused.
    #[test]
    fn timer_expires_after_total_elapsed(minutes in 1u32..=120) {
        let mut core = Core::new();
        core.dispatch(Command::Play);
        core.dispatch(Command::StartSleepTimer { minutes });
        let total = minutes as u64 * MS_PER_MIN;
        let update = core.dispatch(Command::Tick { elapsed_ms: total + 1_000 });
        prop_assert_eq!(update.snapshot.timer.kind, TimerSnapshotKind::JustCompleted);
        prop_assert!(!update.snapshot.is_playing);
        prop_assert!(update.effects.iter().any(|e| matches!(e, Effect::PausePlayback)));
    }

    /// Settings survive a serialize → restore round-trip.
    #[test]
    fn settings_round_trip(
        percent in any::<u8>(),
        sleep in prop::option::of(1u32..=1_440),
        pomo in prop::option::of(1u32..=1_440),
    ) {
        let mut core = Core::new();
        core.dispatch(Command::SetVolume { percent });
        if let Some(m) = sleep { core.dispatch(Command::StartSleepTimer { minutes: m }); }
        if let Some(m) = pomo { core.dispatch(Command::StartPomodoro { minutes: m }); }

        let persisted = core.state().to_settings();
        let json = serde_json::to_string(&persisted).expect("serialize");
        let restored = Core::restore(&json).expect("restore");

        prop_assert_eq!(restored.snapshot().volume_percent, persisted.volume_percent);
        prop_assert_eq!(restored.state().default_sleep_minutes, persisted.default_sleep_minutes);
        prop_assert_eq!(restored.state().default_pomodoro_minutes, persisted.default_pomodoro_minutes);
    }

    /// Fuzz: an arbitrary command sequence never panics and always leaves the
    /// snapshot in a renderable, internally consistent state.
    #[test]
    fn arbitrary_command_sequences_hold_invariants(
        cmds in prop::collection::vec(command_strategy(), 1..60),
    ) {
        let mut core = Core::new();
        for cmd in cmds {
            let s = core.dispatch(cmd).snapshot;
            prop_assert!(s.volume_percent <= MAX_VOLUME);
            prop_assert!(s.timer.progress.is_finite());
            prop_assert!((0.0f32..=1.0).contains(&s.timer.progress));
            prop_assert!(s.timer.remaining_ms <= s.timer.total_ms.max(s.timer.remaining_ms));
            if s.timer.total_ms > 0 {
                prop_assert!(s.timer.remaining_ms <= s.timer.total_ms);
            }
            prop_assert!(!s.primary_button_label.is_empty());
        }
    }

    /// Muting while playing never pauses or cancels the timer, and silences
    /// output; a second toggle restores the chosen volume.
    #[test]
    fn mute_keeps_session_and_round_trips(volume in 0u8..=100, minutes in 1u32..=600) {
        let mut core = Core::new();
        core.dispatch(Command::SetVolume { percent: volume });
        core.dispatch(Command::StartPomodoro { minutes }); // auto-plays
        let timer_kind = core.snapshot().timer.kind;

        let muted = core.dispatch(Command::ToggleMute);
        prop_assert!(muted.snapshot.is_muted);
        prop_assert!(muted.snapshot.is_playing, "mute must not pause");
        prop_assert_eq!(muted.snapshot.timer.kind, timer_kind, "mute must not end the timer");
        let zeroed = muted.effects.iter().any(|e| matches!(e, Effect::SetPlatformVolume { volume_percent: 0 }));
        prop_assert!(zeroed);
        // No pause effect was emitted.
        prop_assert!(!muted.effects.iter().any(|e| matches!(e, Effect::PausePlayback)));

        let unmuted = core.dispatch(Command::ToggleMute);
        prop_assert!(!unmuted.snapshot.is_muted);
        let restored = unmuted.effects.iter().any(|e| matches!(e, Effect::SetPlatformVolume { volume_percent } if *volume_percent == volume));
        prop_assert!(restored);
    }

    /// Pausing — after any history — never starts playback and always lands paused.
    #[test]
    fn pause_never_starts_playback(cmds in prop::collection::vec(command_strategy(), 0..40)) {
        let mut core = Core::new();
        for cmd in cmds { core.dispatch(cmd); }
        let update = core.dispatch(Command::Pause);
        let started = update
            .effects
            .iter()
            .any(|e| matches!(e, Effect::StartPlayback { .. }));
        prop_assert!(!started);
        prop_assert!(!update.snapshot.is_playing);
    }
}
