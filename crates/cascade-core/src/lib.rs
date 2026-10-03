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

pub mod account;
pub mod command;
pub mod effect;
pub mod listening;
pub mod settings;
pub mod snapshot;
pub mod state;
pub mod timer;

pub use account::{PersistedAccount, ACCOUNT_VERSION};
pub use command::Command;
pub use effect::Effect;
pub use listening::{
    InFlight, ListeningLedger, PersistedListening, SyncReason, LISTENING_SYNC_THRESHOLD_MS,
    LISTENING_VERSION, MAX_TICK_ACCRUAL_MS,
};
pub use settings::{PersistedSettings, SETTINGS_VERSION};
pub use snapshot::{
    AccountSnapshot, ListeningSnapshot, Snapshot, TimerOptions, TimerSnapshot, TimerSnapshotKind,
    PLAYBACK_TICK_INTERVAL_MS, TIMER_TICK_INTERVAL_MS,
};
pub use state::{PlaybackIntent, State, TimerMode};
pub use timer::{TimerPreset, MAX_TIMER_MINUTES, MIN_TIMER_MINUTES};

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
    #[error("invalid command: {0}")]
    InvalidCommand(String),
}

/// The core. Construct with [`Core::new`], restore persisted data by
/// dispatching [`Command::Restore`] once, then drive it with
/// [`Core::dispatch`].
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
}
