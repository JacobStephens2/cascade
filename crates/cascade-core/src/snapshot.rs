//! UI-facing snapshot. The whole point of this type is that the UI never has
//! to ask the core a follow-up question to render a frame.

use serde::{Deserialize, Serialize};

use crate::state::{State, DEFAULT_VOLUME_PERCENT};
use crate::timer::{format_remaining, TimerKind};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TimerSnapshotKind {
    Off,
    Sleep,
    Pomodoro,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub title: String,
    pub subtitle: String,
    pub is_playing: bool,
    pub volume_percent: u8,
    pub primary_button_label: String,
    pub timer: TimerSnapshot,
    pub error_message: Option<String>,
}

impl Snapshot {
    pub fn from_state(state: &State) -> Self {
        let timer = match (&state.active_timer, state.timer_just_completed) {
            (_, Some(kind)) => TimerSnapshot {
                kind: TimerSnapshotKind::JustCompleted,
                remaining_label: match kind {
                    TimerKind::Sleep => "Sleep timer ended".to_string(),
                    TimerKind::Pomodoro => "Session complete".to_string(),
                },
                remaining_ms: 0,
                total_ms: 0,
                progress: 1.0,
            },
            (Some(t), None) => {
                let progress = if t.total_ms == 0 {
                    0.0
                } else {
                    1.0 - (t.remaining_ms as f32 / t.total_ms as f32)
                };
                TimerSnapshot {
                    kind: match t.kind {
                        TimerKind::Sleep => TimerSnapshotKind::Sleep,
                        TimerKind::Pomodoro => TimerSnapshotKind::Pomodoro,
                    },
                    remaining_label: format_remaining(t.remaining_ms),
                    remaining_ms: t.remaining_ms,
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
            subtitle: "Waterfall loop".to_string(),
            is_playing: state.intent.is_playing(),
            volume_percent: state.volume_percent.unwrap_or(DEFAULT_VOLUME_PERCENT),
            primary_button_label: if state.intent.is_playing() {
                "Pause".to_string()
            } else {
                "Play".to_string()
            },
            timer,
            error_message: state.last_error.clone(),
        }
    }
}
