//! Timer subsystem: sleep timer, pomodoro / focus sessions, and a stopwatch.
//!
//! Sleep and pomodoro are countdowns; the stopwatch counts up with no end so
//! the user can see how long they've been listening. All three are driven by
//! `Tick` events — the platform never reads `SystemTime`; it computes a delta
//! and hands it to the core. Internally every timer tracks `elapsed_ms`;
//! countdowns derive their remaining time from `total_ms - elapsed_ms`.

use serde::{Deserialize, Serialize};

/// Shortest timer the core will start, in minutes.
pub const MIN_TIMER_MINUTES: u32 = 1;
/// Longest timer the core will start, in minutes (one day).
pub const MAX_TIMER_MINUTES: u32 = 1440;
/// Length the custom field opens on before the user has chosen one.
pub const DEFAULT_TIMER_MINUTES: u32 = 30;

/// Timer presets offered for a focus session, in minutes.
pub const FOCUS_PRESET_MINUTES: [u32; 3] = [30, 60, 480];
/// Timer presets offered for a sleep timer, in minutes.
pub const SLEEP_PRESET_MINUTES: [u32; 3] = [15, 30, 60];

/// Clamp a requested timer length into the limits. A clamp can't fail, so
/// the start commands take any `u32`.
pub fn clamp_minutes(minutes: u32) -> u32 {
    minutes.clamp(MIN_TIMER_MINUTES, MAX_TIMER_MINUTES)
}

/// A ready-made timer length the core offers, with a full-width `label`
/// ("1 hr") and a compact `short_label` ("1h").
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TimerPreset {
    pub minutes: u32,
    pub label: String,
    pub short_label: String,
}

impl TimerPreset {
    /// Whole hours read as hours, anything else as minutes.
    pub fn new(minutes: u32) -> Self {
        let (label, short_label) = if minutes >= 60 && minutes.is_multiple_of(60) {
            let hours = minutes / 60;
            (format!("{hours} hr"), format!("{hours}h"))
        } else {
            (format!("{minutes} min"), format!("{minutes}m"))
        };
        Self {
            minutes,
            label,
            short_label,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TimerKind {
    Sleep,
    Pomodoro,
    /// Count-up timer with no end. Never expires, never pauses playback.
    Stopwatch,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ActiveTimer {
    pub kind: TimerKind,
    /// Countdown length in ms. `0` means unbounded (a stopwatch).
    pub total_ms: u64,
    /// Wall-clock time elapsed since the timer started, accumulated from ticks.
    pub elapsed_ms: u64,
}

impl ActiveTimer {
    /// Start a countdown (sleep / pomodoro) of `minutes`.
    pub fn start(kind: TimerKind, minutes: u32) -> Self {
        Self {
            kind,
            total_ms: (minutes as u64).saturating_mul(60_000),
            elapsed_ms: 0,
        }
    }

    /// Start an open-ended stopwatch.
    pub fn stopwatch() -> Self {
        Self {
            kind: TimerKind::Stopwatch,
            total_ms: 0,
            elapsed_ms: 0,
        }
    }

    /// True for sleep / pomodoro (bounded); false for the stopwatch.
    pub fn is_countdown(&self) -> bool {
        self.total_ms > 0
    }

    /// Remaining time for a countdown; `0` for a stopwatch (no end).
    pub fn remaining_ms(&self) -> u64 {
        self.total_ms.saturating_sub(self.elapsed_ms)
    }

    /// Advance by `elapsed_ms`. Returns `true` only when a *countdown* crosses
    /// zero on this tick (the stopwatch never expires).
    pub fn tick(&mut self, elapsed_ms: u64) -> bool {
        let was_remaining = self.remaining_ms();
        self.elapsed_ms = self.elapsed_ms.saturating_add(elapsed_ms);
        self.is_countdown() && was_remaining > 0 && self.remaining_ms() == 0
    }

    pub fn is_expired(&self) -> bool {
        self.is_countdown() && self.remaining_ms() == 0
    }
}

/// Format a remaining-millisecond value as `H:MM:SS` (or `M:SS` when under an
/// hour). The snapshot includes this so every UI renders identical strings.
pub fn format_remaining(remaining_ms: u64) -> String {
    let total_seconds = remaining_ms / 1000;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_counts_down_and_flags_expiry() {
        let mut t = ActiveTimer::start(TimerKind::Sleep, 1); // 60_000ms
        assert!(!t.tick(30_000));
        assert_eq!(t.remaining_ms(), 30_000);
        assert!(!t.tick(29_999));
        assert!(t.tick(2)); // crosses zero
        assert_eq!(t.remaining_ms(), 0);
        assert!(t.is_expired());
        // Subsequent ticks never re-fire expiry.
        assert!(!t.tick(10_000));
    }

    #[test]
    fn stopwatch_counts_up_and_never_expires() {
        let mut t = ActiveTimer::stopwatch();
        assert!(!t.is_countdown());
        assert!(!t.tick(30_000));
        assert_eq!(t.elapsed_ms, 30_000);
        assert!(!t.tick(90_000));
        assert_eq!(t.elapsed_ms, 120_000);
        assert!(!t.is_expired());
        // Even after an hour it just keeps counting.
        assert!(!t.tick(3_600_000));
        assert_eq!(t.elapsed_ms, 3_720_000);
    }

    #[test]
    fn preset_labels_read_hours_or_minutes() {
        assert_eq!(TimerPreset::new(15).label, "15 min");
        assert_eq!(TimerPreset::new(15).short_label, "15m");
        assert_eq!(TimerPreset::new(90).label, "90 min");
        assert_eq!(TimerPreset::new(480).label, "8 hr");
        assert_eq!(TimerPreset::new(480).short_label, "8h");
    }

    #[test]
    fn format_under_an_hour_uses_short_form() {
        assert_eq!(format_remaining(125_000), "2:05");
        assert_eq!(format_remaining(0), "0:00");
    }

    #[test]
    fn format_over_an_hour_uses_long_form() {
        assert_eq!(format_remaining(3_661_000), "1:01:01");
        assert_eq!(format_remaining(8 * 3_600_000), "8:00:00");
    }
}
