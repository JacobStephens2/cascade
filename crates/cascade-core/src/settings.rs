//! Persisted settings — the JSON the platform stores between sessions.
//!
//! Settings are versioned so that future schema changes can be handled
//! explicitly rather than guessed at.

use serde::{Deserialize, Serialize};

pub const SETTINGS_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PersistedSettings {
    pub version: u32,
    pub volume_percent: u8,
    /// Last-started sleep-timer length in minutes, or `None` if the user has
    /// not chosen one. The snapshot's `timerOptions.customSleepMinutes`
    /// pre-fills the custom field from it.
    pub default_sleep_minutes: Option<u32>,
    /// Last-started pomodoro length in minutes; pre-fills
    /// `timerOptions.customFocusMinutes`.
    pub default_pomodoro_minutes: Option<u32>,
}
