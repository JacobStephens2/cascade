//! User and platform commands that drive the core.

use serde::{Deserialize, Serialize};

/// Everything that can happen to the core. User actions, platform reports,
/// and wall-clock ticks all funnel through here.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Command {
    /// User asked to start playback.
    Play,
    /// User asked to pause playback.
    Pause,
    /// User tapped the primary button; flip between Play and Pause.
    TogglePlayback,
    /// Set volume in the range `0..=100`. Values outside the range are clamped.
    SetVolume { percent: u8 },

    /// Start a plain sleep timer that pauses playback after `minutes` minutes.
    StartSleepTimer { minutes: u32 },
    /// Start a pomodoro / focus session of `minutes` minutes. When the timer
    /// expires playback pauses; the UI can distinguish "session complete" from
    /// "sleep" via [`crate::TimerSnapshotKind`].
    StartPomodoro { minutes: u32 },
    /// Cancel any running timer without touching playback.
    CancelTimer,

    /// Wall-clock tick from the platform. `elapsed_ms` is the delta since the
    /// previous tick. The core never reads the system clock; the UI ticks it.
    Tick { elapsed_ms: u64 },

    /// Platform reports that audio actually started playing (e.g. the browser
    /// finally honored a play() promise, or ExoPlayer reported STATE_READY).
    PlatformPlaybackStarted,
    /// Platform reports playback paused (user pressed media key, audio focus
    /// loss, end of file in a non-looping mode, etc.).
    PlatformPlaybackPaused,
    /// Platform reports a playback error that the user should know about.
    PlatformPlaybackError { message: String },
}
