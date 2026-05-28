//! `cascade-core` — the headless state machine for the Cascade waterfall player.
//!
//! The core owns *intent*: what playback should be doing, the volume, the
//! sleep / pomodoro timers, and the persisted settings. It does **not** own
//! audio output, file I/O, or the system clock. Time is supplied by the
//! platform via [`Command::Tick`].
//!
//! The shape is deliberately Clave-style: one [`dispatch`] entry point, every
//! call returns an [`Update`] containing a fresh [`Snapshot`] (for rendering)
//! and a list of [`Effect`]s (for the platform to execute).

pub mod command;
pub mod effect;
pub mod settings;
pub mod snapshot;
pub mod state;
pub mod timer;

pub use command::Command;
pub use effect::Effect;
pub use settings::{PersistedSettings, SETTINGS_VERSION};
pub use snapshot::{Snapshot, TimerSnapshot, TimerSnapshotKind};
pub use state::{PlaybackIntent, State, TimerMode};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// The result of dispatching a [`Command`]: a new [`Snapshot`] and a list of
/// [`Effect`]s the platform should execute. The reducer is pure — apply the
/// same sequence of commands and you get the same updates.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Update {
    pub snapshot: Snapshot,
    pub effects: Vec<Effect>,
}

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("settings JSON could not be parsed: {0}")]
    BadSettings(String),
    #[error("unsupported settings version {found} (expected {expected})")]
    UnsupportedSettingsVersion { found: u32, expected: u32 },
    #[error("invalid command: {0}")]
    InvalidCommand(String),
}

/// The core. Construct with [`Core::new`] or [`Core::restore`], then drive it
/// with [`Core::dispatch`].
#[derive(Debug, Clone)]
pub struct Core {
    state: State,
}

impl Core {
    /// Start a fresh session with the default settings.
    pub fn new() -> Self {
        Self {
            state: State::default(),
        }
    }

    /// Restore a session from previously persisted settings JSON.
    ///
    /// If the JSON is missing or fails to parse, the caller is expected to
    /// fall back to [`Core::new`] — the core does not try to guess.
    pub fn restore(settings_json: &str) -> Result<Self, CoreError> {
        let persisted: PersistedSettings = serde_json::from_str(settings_json)
            .map_err(|e| CoreError::BadSettings(e.to_string()))?;
        if persisted.version != SETTINGS_VERSION {
            return Err(CoreError::UnsupportedSettingsVersion {
                found: persisted.version,
                expected: SETTINGS_VERSION,
            });
        }
        Ok(Self {
            state: State::from_settings(persisted),
        })
    }

    /// Render the current state as a [`Snapshot`] without dispatching a
    /// command. Useful for the very first render before any user interaction.
    pub fn snapshot(&self) -> Snapshot {
        Snapshot::from_state(&self.state)
    }

    /// Apply a [`Command`] and return the resulting [`Update`].
    pub fn dispatch(&mut self, command: Command) -> Update {
        let mut effects = Vec::new();
        state::reduce(&mut self.state, command, &mut effects);
        Update {
            snapshot: Snapshot::from_state(&self.state),
            effects,
        }
    }

    /// Borrow the current internal state (useful for tests / debugging).
    pub fn state(&self) -> &State {
        &self.state
    }
}

impl Default for Core {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_core_is_paused_at_default_volume() {
        let core = Core::new();
        let snap = core.snapshot();
        assert!(!snap.is_playing);
        assert_eq!(snap.volume_percent, state::DEFAULT_VOLUME_PERCENT);
        assert_eq!(snap.timer.kind, TimerSnapshotKind::Off);
    }

    #[test]
    fn play_command_emits_start_effect_and_persist() {
        let mut core = Core::new();
        let update = core.dispatch(Command::Play);
        assert!(update.snapshot.is_playing);
        assert!(update
            .effects
            .iter()
            .any(|e| matches!(e, Effect::StartPlayback { .. })));
        assert!(update
            .effects
            .iter()
            .any(|e| matches!(e, Effect::PersistSettings { .. })));
    }

    #[test]
    fn restore_round_trips_settings() {
        let mut core = Core::new();
        core.dispatch(Command::SetVolume { percent: 42 });
        let persisted = core.state().to_settings();
        let json = serde_json::to_string(&persisted).unwrap();
        let restored = Core::restore(&json).unwrap();
        assert_eq!(restored.snapshot().volume_percent, 42);
    }
}
