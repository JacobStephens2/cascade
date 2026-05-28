//! Timer subsystem: sleep timer and pomodoro / focus sessions.
//!
//! Both are countdown timers driven by `Tick` events. A pomodoro is just a
//! sleep timer with a different label so the UI can show "Focus session
//! complete" vs. "Sleep". The platform never reads `SystemTime`; it computes
//! a delta and hands it to the core.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TimerKind {
    Sleep,
    Pomodoro,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ActiveTimer {
    pub kind: TimerKind,
    pub total_ms: u64,
    pub remaining_ms: u64,
}

impl ActiveTimer {
    pub fn start(kind: TimerKind, minutes: u32) -> Self {
        let total = (minutes as u64).saturating_mul(60_000);
        Self {
            kind,
            total_ms: total,
            remaining_ms: total,
        }
    }

    /// Advance by `elapsed_ms`. Returns `true` if the timer just hit zero on
    /// this tick.
    pub fn tick(&mut self, elapsed_ms: u64) -> bool {
        if self.remaining_ms == 0 {
            return false;
        }
        if elapsed_ms >= self.remaining_ms {
            self.remaining_ms = 0;
            true
        } else {
            self.remaining_ms -= elapsed_ms;
            false
        }
    }

    pub fn is_expired(&self) -> bool {
        self.remaining_ms == 0
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
        assert_eq!(t.remaining_ms, 30_000);
        assert!(!t.tick(29_999));
        assert!(t.tick(2)); // crosses zero
        assert_eq!(t.remaining_ms, 0);
        assert!(t.is_expired());
        // Subsequent ticks are no-ops.
        assert!(!t.tick(10_000));
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
