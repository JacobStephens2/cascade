//! UI-facing snapshot. The whole point of this type is that the UI never has
//! to ask the core a follow-up question to render a frame.

use serde::{Deserialize, Serialize};

use crate::listening::format_listening_total;
use crate::state::{State, DEFAULT_VOLUME_PERCENT};
use crate::timer::{format_remaining, TimerKind};

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
    pub primary_button_label: String,
    pub timer: TimerSnapshot,
    pub error_message: Option<String>,
    pub listening: ListeningSnapshot,
}

impl Snapshot {
    pub fn from_state(state: &State) -> Self {
        let timer = match (&state.active_timer, state.timer_just_completed) {
            (_, Some(kind)) => TimerSnapshot {
                kind: TimerSnapshotKind::JustCompleted,
                remaining_label: match kind {
                    TimerKind::Sleep => "Sleep timer ended".to_string(),
                    TimerKind::Pomodoro => "Session complete".to_string(),
                    // A stopwatch never expires, so this arm is unreachable in
                    // practice; keep the match exhaustive.
                    TimerKind::Stopwatch => String::new(),
                },
                remaining_ms: 0,
                total_ms: 0,
                progress: 1.0,
            },
            (Some(t), None) if t.kind == TimerKind::Stopwatch => TimerSnapshot {
                // Count-up: the time fields carry elapsed, with no total/progress.
                kind: TimerSnapshotKind::Stopwatch,
                remaining_label: format_remaining(t.elapsed_ms),
                remaining_ms: t.elapsed_ms,
                total_ms: 0,
                progress: 0.0,
            },
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
                TimerSnapshot {
                    kind: match t.kind {
                        TimerKind::Sleep => TimerSnapshotKind::Sleep,
                        TimerKind::Pomodoro => TimerSnapshotKind::Pomodoro,
                        TimerKind::Stopwatch => unreachable!("handled above"),
                    },
                    remaining_label: format_remaining(remaining),
                    remaining_ms: remaining,
                    total_ms: t.total_ms,
                    progress,
                }
            }
            (None, None) => TimerSnapshot {
                kind: TimerSnapshotKind::Off,
                remaining_label: String::new(),
                remaining_ms: 0,
                total_ms: 0,
                progress: 0.0,
            },
        };

        Snapshot {
            title: "Cascade".to_string(),
            subtitle: "The Falls".to_string(),
            is_playing: state.intent.is_playing(),
            volume_percent: state.volume_percent.unwrap_or(DEFAULT_VOLUME_PERCENT),
            is_muted: state.muted,
            primary_button_label: if state.intent.is_playing() {
                "Pause".to_string()
            } else {
                "Play".to_string()
            },
            timer,
            error_message: state.last_error.clone(),
            listening: ListeningSnapshot {
                tracking_enabled: state.listening.tracking_enabled,
                device_total_ms: state.listening.device_total_ms,
                displayed_total_ms: state.listening.displayed_total_ms(),
                unsynced_ms: state.listening.unsynced_ms(),
                total_label: format_listening_total(state.listening.displayed_total_ms()),
            },
        }
    }
}
