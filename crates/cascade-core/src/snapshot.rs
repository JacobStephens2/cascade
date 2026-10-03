//! UI-facing snapshot. The whole point of this type is that the UI never has
//! to ask the core a follow-up question to render a frame.

use serde::{Deserialize, Serialize};

use crate::listening::format_listening_total;
use crate::state::{State, DEFAULT_VOLUME_PERCENT};
use crate::timer::{
    clamp_minutes, format_remaining, TimerKind, TimerPreset, DEFAULT_TIMER_MINUTES,
    FOCUS_PRESET_MINUTES, MAX_TIMER_MINUTES, MIN_TIMER_MINUTES, SLEEP_PRESET_MINUTES,
};

/// Tick cadence while any timer is active (playing or paused), so the
/// countdown or stopwatch reads smoothly.
pub const TIMER_TICK_INTERVAL_MS: u64 = 250;
/// Tick cadence while audio is merely playing, so listening time keeps
/// accruing.
pub const PLAYBACK_TICK_INTERVAL_MS: u64 = 1_000;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TimerSnapshotKind {
    Off,
    Sleep,
    Pomodoro,
    /// Count-up stopwatch. `remaining_label`/`remaining_ms` carry the elapsed
    /// time, `total_ms` is 0, and `progress` is 0 (no end to progress toward).
    Stopwatch,
    /// Timer just finished on the most recent tick. UIs can show a chime /
    /// toast before transitioning back to `Off` on the next snapshot.
    JustCompleted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TimerSnapshot {
    pub kind: TimerSnapshotKind,
    /// `0:00` style countdown. Empty when `kind == Off`.
    pub remaining_label: String,
    pub remaining_ms: u64,
    pub total_ms: u64,
    /// 0.0 → 1.0 for progress UIs. `0.0` when no timer is running.
    pub progress: f32,
    /// A sleep timer, focus session or stopwatch is running: there is
    /// something to cancel. False when off and when just completed.
    pub is_active: bool,
    /// One-line timer status, e.g. `"Playing · 12:34 left"`,
    /// `"Muted · no timer"`, `"Stopwatch · 12:34"`, or the completion text.
    pub status_label: String,
}

/// The timer choices a shell offers: the timer presets, the custom-duration
/// limits, and what each custom field opens on.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TimerOptions {
    pub focus_presets: Vec<TimerPreset>,
    pub sleep_presets: Vec<TimerPreset>,
    pub min_minutes: u32,
    pub max_minutes: u32,
    /// The last-started focus length, clamped; 30 until one has been chosen.
    pub custom_focus_minutes: u32,
    /// The last-started sleep length, clamped; 30 until one has been chosen.
    pub custom_sleep_minutes: u32,
}

impl TimerOptions {
    fn from_state(state: &State) -> Self {
        let custom = |saved: Option<u32>| clamp_minutes(saved.unwrap_or(DEFAULT_TIMER_MINUTES));
        Self {
            focus_presets: FOCUS_PRESET_MINUTES.map(TimerPreset::new).to_vec(),
            sleep_presets: SLEEP_PRESET_MINUTES.map(TimerPreset::new).to_vec(),
            min_minutes: MIN_TIMER_MINUTES,
            max_minutes: MAX_TIMER_MINUTES,
            custom_focus_minutes: custom(state.default_pomodoro_minutes),
            custom_sleep_minutes: custom(state.default_sleep_minutes),
        }
    }
}

/// Listening-time view for the UI. `displayed_total_ms` is the number to show;
/// `total_label` is a ready-formatted version of it. `unsynced_ms` is what a
/// shell watches to decide when to sync.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListeningSnapshot {
    pub tracking_enabled: bool,
    /// This device's grow-only slot.
    pub device_total_ms: u64,
    /// The lifetime total to show the user (server aggregate + unsynced when
    /// signed in, else the device slot).
    pub displayed_total_ms: u64,
    /// Locally-accrued milliseconds the server hasn't acknowledged yet.
    pub unsynced_ms: u64,
    /// `displayed_total_ms` pre-formatted as e.g. `"12h 34m"`.
    pub total_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub title: String,
    pub subtitle: String,
    pub is_playing: bool,
    pub volume_percent: u8,
    /// Audio is silenced but the session/timer keeps running.
    pub is_muted: bool,
    /// The gain the shell should be outputting right now, 0.0–1.0: 0.0
    /// while muted, otherwise the curve applied to `volume_percent`. Shells
    /// show `volume_percent`/`is_muted` and play `output_gain`.
    pub output_gain: f32,
    pub primary_button_label: String,
    pub timer: TimerSnapshot,
    pub timer_options: TimerOptions,
    pub error_message: Option<String>,
    pub listening: ListeningSnapshot,
    /// How often the shell should send [`crate::Command::Tick`], in ms. `0`
    /// means stop ticking. The core owns the cadence; the shell owns the clock.
    pub tick_interval_ms: u64,
}

impl Snapshot {
    pub fn from_state(state: &State) -> Self {
        let (kind, remaining_label, remaining_ms, total_ms, progress) =
            match (&state.active_timer, state.timer_just_completed) {
                (_, Some(kind)) => (
                    TimerSnapshotKind::JustCompleted,
                    match kind {
                        TimerKind::Sleep => "Sleep timer ended".to_string(),
                        TimerKind::Pomodoro => "Session complete".to_string(),
                        // A stopwatch never expires, so this arm is unreachable in
                        // practice; keep the match exhaustive.
                        TimerKind::Stopwatch => String::new(),
                    },
                    0,
                    0,
                    1.0,
                ),
                // Count-up: the time fields carry elapsed, with no total/progress.
                (Some(t), None) if t.kind == TimerKind::Stopwatch => (
                    TimerSnapshotKind::Stopwatch,
                    format_remaining(t.elapsed_ms),
                    t.elapsed_ms,
                    0,
                    0.0,
                ),
                (Some(t), None) => {
                    let remaining = t.remaining_ms();
                    let progress = if t.total_ms == 0 {
                        0.0
                    } else {
                        // Clamp: f32 division on million-ms durations can round a
                        // hair outside [0,1], and UI progress bars expect a clean
                        // fraction.
                        (1.0 - (remaining as f32 / t.total_ms as f32)).clamp(0.0, 1.0)
                    };
                    (
                        match t.kind {
                            TimerKind::Sleep => TimerSnapshotKind::Sleep,
                            TimerKind::Pomodoro => TimerSnapshotKind::Pomodoro,
                            TimerKind::Stopwatch => unreachable!("handled above"),
                        },
                        format_remaining(remaining),
                        remaining,
                        t.total_ms,
                        progress,
                    )
                }
                (None, None) => (TimerSnapshotKind::Off, String::new(), 0, 0, 0.0),
            };
        let timer = TimerSnapshot {
            kind,
            status_label: status_label(state, kind, &remaining_label),
            remaining_label,
            remaining_ms,
            total_ms,
            progress,
            // A just-completed timer has nothing left to cancel.
            is_active: matches!(
                kind,
                TimerSnapshotKind::Sleep
                    | TimerSnapshotKind::Pomodoro
                    | TimerSnapshotKind::Stopwatch
            ),
        };

        Snapshot {
            title: "Cascade".to_string(),
            subtitle: "The Falls".to_string(),
            is_playing: state.intent.is_playing(),
            volume_percent: state.volume_percent.unwrap_or(DEFAULT_VOLUME_PERCENT),
            is_muted: state.muted,
            output_gain: state.output_gain(),
            primary_button_label: if state.intent.is_playing() {
                "Pause".to_string()
            } else {
                "Play".to_string()
            },
            timer,
            timer_options: TimerOptions::from_state(state),
            error_message: state.last_error.clone(),
            listening: ListeningSnapshot {
                tracking_enabled: state.listening.tracking_enabled,
                device_total_ms: state.listening.device_total_ms,
                displayed_total_ms: state.listening.displayed_total_ms(),
                unsynced_ms: state.listening.unsynced_ms(),
                total_label: format_listening_total(state.listening.displayed_total_ms()),
            },
            tick_interval_ms: tick_interval_ms(state),
        }
    }
}

/// The timer read as one line: playback state plus what the timer shows.
fn status_label(state: &State, kind: TimerSnapshotKind, remaining_label: &str) -> String {
    // Pausing clears mute, so "Muted" only ever means muted while playing.
    let playback = if state.muted {
        "Muted"
    } else if state.intent.is_playing() {
        "Playing"
    } else {
        "Paused"
    };
    match kind {
        TimerSnapshotKind::Off if state.intent.is_playing() => format!("{playback} · no timer"),
        TimerSnapshotKind::Off => playback.to_string(),
        TimerSnapshotKind::Sleep | TimerSnapshotKind::Pomodoro => {
            format!("{playback} · {remaining_label} left")
        }
        TimerSnapshotKind::Stopwatch => format!("Stopwatch · {remaining_label}"),
        TimerSnapshotKind::JustCompleted => remaining_label.to_string(),
    }
}

/// Fine ticks while any timer is active (a just-completed timer is no longer
/// active), coarse ticks while playback is intended, otherwise none.
fn tick_interval_ms(state: &State) -> u64 {
    if state.active_timer.is_some() {
        TIMER_TICK_INTERVAL_MS
    } else if state.intent.is_playing() {
        PLAYBACK_TICK_INTERVAL_MS
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Command, Core};

    fn interval_after(commands: impl IntoIterator<Item = Command>) -> u64 {
        let mut core = Core::new();
        for command in commands {
            core.dispatch(command);
        }
        core.snapshot().tick_interval_ms
    }

    #[test]
    fn new_core_does_not_tick() {
        assert_eq!(Core::new().snapshot().tick_interval_ms, 0);
    }

    #[test]
    fn plain_playback_ticks_coarsely() {
        assert_eq!(interval_after([Command::Play]), PLAYBACK_TICK_INTERVAL_MS);
    }

    #[test]
    fn any_active_timer_ticks_finely_whether_playing_or_paused() {
        let starts = [
            Command::StartSleepTimer { minutes: 10 },
            Command::StartPomodoro { minutes: 25 },
            Command::StartStopwatch,
        ];
        for start in starts {
            assert_eq!(
                interval_after([Command::Play, start.clone()]),
                TIMER_TICK_INTERVAL_MS,
                "playing + {start:?}"
            );
            assert_eq!(
                interval_after([start.clone(), Command::Pause]),
                TIMER_TICK_INTERVAL_MS,
                "paused + {start:?}"
            );
        }
    }

    #[test]
    fn cancelling_timer_while_playing_falls_back_to_coarse() {
        assert_eq!(
            interval_after([Command::Play, Command::StartStopwatch, Command::CancelTimer]),
            PLAYBACK_TICK_INTERVAL_MS
        );
    }

    #[test]
    fn expired_sleep_timer_stops_ticking() {
        let mut core = Core::new();
        core.dispatch(Command::Play);
        core.dispatch(Command::StartSleepTimer { minutes: 1 });
        let snap = core.dispatch(Command::Tick { elapsed_ms: 61_000 }).snapshot;
        assert_eq!(snap.timer.kind, TimerSnapshotKind::JustCompleted);
        assert!(!snap.is_playing);
        assert_eq!(snap.tick_interval_ms, 0);
    }

    #[test]
    fn pausing_with_stopwatch_running_keeps_fine_ticks() {
        assert_eq!(
            interval_after([Command::Play, Command::StartStopwatch, Command::Pause]),
            TIMER_TICK_INTERVAL_MS
        );
    }

    // Every shell reads `tickIntervalMs` off the JSON snapshot; a rename would
    // silently stop their tick loops.
    #[test]
    fn tick_interval_serializes_camel_case() {
        let mut core = Core::new();
        core.dispatch(Command::Play);
        let json = serde_json::to_string(&core.snapshot()).unwrap();
        assert!(
            json.contains(r#""tickIntervalMs":1000"#),
            "snapshot JSON: {json}"
        );
    }
}
