//! Internal state and the reducer.
//!
//! The reducer is the only place that mutates state. It is a pure function of
//! `(state, command) -> effects`, and it never reads the system clock, file
//! system, or network.

use serde::{Deserialize, Serialize};

use crate::command::Command;
use crate::effect::Effect;
use crate::listening::{ListeningLedger, PersistedListening};
use crate::settings::{PersistedSettings, SETTINGS_VERSION};
use crate::timer::{ActiveTimer, TimerKind};

pub const DEFAULT_VOLUME_PERCENT: u8 = 60;
pub const MIN_VOLUME_PERCENT: u8 = 0;
pub const MAX_VOLUME_PERCENT: u8 = 100;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PlaybackIntent {
    Playing,
    Paused,
}

impl PlaybackIntent {
    pub fn is_playing(self) -> bool {
        matches!(self, PlaybackIntent::Playing)
    }
}

/// Public alias matching the snapshot field for callers who want to inspect
/// what kind of timer is running without reaching into [`ActiveTimer`].
pub type TimerMode = TimerKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    pub intent: PlaybackIntent,
    /// `None` means "use [`DEFAULT_VOLUME_PERCENT`]". After the first user
    /// adjustment this is always `Some`.
    pub volume_percent: Option<u8>,
    /// Audio silenced without pausing the session. Transient — never persisted,
    /// and always cleared when playback starts or stops.
    pub muted: bool,
    pub active_timer: Option<ActiveTimer>,
    /// Set on the tick that an active timer hit zero, cleared on the next
    /// dispatch. Surfaces in snapshots as
    /// [`crate::TimerSnapshotKind::JustCompleted`].
    pub timer_just_completed: Option<TimerKind>,
    pub default_sleep_minutes: Option<u32>,
    pub default_pomodoro_minutes: Option<u32>,
    pub last_error: Option<String>,
    /// True only between [`Command::PlatformPlaybackStarted`] and the next
    /// pause/error — i.e. audio is *confirmed* playing, not merely intended.
    /// Listening accrues on this, not on `intent`, so an autoplay-blocked shell
    /// (intent says Playing, audio never started) accrues nothing.
    pub audio_confirmed_playing: bool,
    /// Listening-time ledger. Restored from its own blob via
    /// [`Command::RestoreListening`], not from settings.
    pub listening: ListeningLedger,
}

impl Default for State {
    fn default() -> Self {
        Self {
            intent: PlaybackIntent::Paused,
            volume_percent: None,
            muted: false,
            active_timer: None,
            timer_just_completed: None,
            default_sleep_minutes: Some(30),
            default_pomodoro_minutes: Some(30),
            last_error: None,
            audio_confirmed_playing: false,
            listening: ListeningLedger::default(),
        }
    }
}

impl State {
    pub fn from_settings(s: PersistedSettings) -> Self {
        Self {
            intent: PlaybackIntent::Paused,
            volume_percent: Some(clamp_volume(s.volume_percent)),
            muted: false,
            active_timer: None,
            timer_just_completed: None,
            default_sleep_minutes: s.default_sleep_minutes,
            default_pomodoro_minutes: s.default_pomodoro_minutes,
            last_error: None,
            audio_confirmed_playing: false,
            listening: ListeningLedger::default(),
        }
    }

    pub fn to_settings(&self) -> PersistedSettings {
        PersistedSettings {
            version: SETTINGS_VERSION,
            volume_percent: self.volume_percent.unwrap_or(DEFAULT_VOLUME_PERCENT),
            default_sleep_minutes: self.default_sleep_minutes,
            default_pomodoro_minutes: self.default_pomodoro_minutes,
        }
    }

    pub fn effective_volume(&self) -> u8 {
        self.volume_percent.unwrap_or(DEFAULT_VOLUME_PERCENT)
    }

    /// The volume the platform should actually output right now: zero while
    /// muted, otherwise the user's chosen level.
    pub fn output_volume(&self) -> u8 {
        if self.muted {
            0
        } else {
            self.effective_volume()
        }
    }
}

fn clamp_volume(v: u8) -> u8 {
    v.clamp(MIN_VOLUME_PERCENT, MAX_VOLUME_PERCENT)
}

fn push_persist(state: &State, effects: &mut Vec<Effect>) {
    if let Ok(json) = serde_json::to_string(&state.to_settings()) {
        effects.push(Effect::PersistSettings { json });
    }
}

fn push_persist_listening(state: &State, effects: &mut Vec<Effect>) {
    if let Ok(json) = serde_json::to_string(&state.listening.to_persisted()) {
        effects.push(Effect::PersistListening { json });
    }
}

/// Whether a tick should count as listening: tracking on, audio *confirmed*
/// playing (not merely intended), and not muted.
fn is_accruing(state: &State) -> bool {
    state.listening.tracking_enabled && state.audio_confirmed_playing && !state.muted
}

/// The reducer. Mutates `state` and appends to `effects`.
pub fn reduce(state: &mut State, command: Command, effects: &mut Vec<Effect>) {
    // Every dispatch starts by clearing the one-shot `just completed` flag so
    // the JustCompleted state is visible for exactly one snapshot.
    state.timer_just_completed = None;

    match command {
        Command::Play => start_playback(state, effects),
        Command::Pause => stop_playback(state, effects),
        Command::TogglePlayback => {
            if state.intent.is_playing() {
                stop_playback(state, effects);
            } else {
                start_playback(state, effects);
            }
        }
        Command::SetVolume { percent } => {
            let v = clamp_volume(percent);
            state.volume_percent = Some(v);
            // Adjusting the slider unmutes — the intuitive behavior.
            state.muted = false;
            effects.push(Effect::SetPlatformVolume { volume_percent: v });
            push_persist(state, effects);
        }
        Command::ToggleMute => {
            state.muted = !state.muted;
            effects.push(Effect::SetPlatformVolume {
                volume_percent: state.output_volume(),
            });
        }
        Command::StartSleepTimer { minutes } => {
            if minutes == 0 {
                state.active_timer = None;
            } else {
                state.active_timer = Some(ActiveTimer::start(TimerKind::Sleep, minutes));
                state.default_sleep_minutes = Some(minutes);
                push_persist(state, effects);
            }
        }
        Command::StartPomodoro { minutes } => {
            if minutes == 0 {
                state.active_timer = None;
            } else {
                state.active_timer = Some(ActiveTimer::start(TimerKind::Pomodoro, minutes));
                state.default_pomodoro_minutes = Some(minutes);
                // Pomodoro implies "start playing now if not already".
                if !state.intent.is_playing() {
                    start_playback(state, effects);
                }
                push_persist(state, effects);
            }
        }
        Command::StartStopwatch => {
            // Count-up timer; replaces any countdown, leaves playback alone.
            state.active_timer = Some(ActiveTimer::stopwatch());
        }
        Command::CancelTimer => {
            state.active_timer = None;
        }
        Command::Tick { elapsed_ms } => {
            // Accrue listening for the elapsed audible time *before* processing
            // timer expiry: the audio was playing during this delta even if the
            // timer is about to pause it. `accrue` clamps the per-tick delta, so
            // a sleep/wake gap can't inflate the counter.
            let was_accruing = is_accruing(state);
            if was_accruing {
                state.listening.accrue(elapsed_ms);
            }
            if let Some(t) = state.active_timer.as_mut() {
                let just_expired = t.tick(elapsed_ms);
                if just_expired {
                    let kind = t.kind;
                    state.timer_just_completed = Some(kind);
                    state.active_timer = None;
                    if state.intent.is_playing() {
                        stop_playback(state, effects);
                    }
                }
            }
            // Persist the new total on the tick we actually counted something,
            // so a process kill loses at most one tick of listening.
            if was_accruing {
                push_persist_listening(state, effects);
            }
        }
        Command::PlatformPlaybackStarted => {
            // Platform confirms audio is genuinely playing — start accruing and
            // clear any stale error.
            state.audio_confirmed_playing = true;
            state.last_error = None;
        }
        Command::PlatformPlaybackPaused => {
            // Platform paused us without user input (audio focus, error). Make
            // sure our intent matches reality so the UI doesn't lie, and stop
            // accruing — audio is no longer confirmed playing.
            state.intent = PlaybackIntent::Paused;
            state.audio_confirmed_playing = false;
        }
        Command::PlatformPlaybackError { message } => {
            state.last_error = Some(message);
            state.intent = PlaybackIntent::Paused;
            state.audio_confirmed_playing = false;
        }

        Command::SetListeningTracking { enabled } => {
            state.listening.tracking_enabled = enabled;
            push_persist_listening(state, effects);
        }
        Command::RestoreListening { json } => {
            // The shell hands back the opaque blob it stored. A missing or
            // unparseable/unknown-version blob is ignored — restore must never
            // lower a live counter, and the defaults are already correct.
            if let Ok(restored) = serde_json::from_str::<PersistedListening>(&json) {
                if restored.version == crate::listening::LISTENING_VERSION {
                    let ledger = ListeningLedger::from_persisted(&restored);
                    state.listening.restore_from(&ledger);
                }
            }
        }
        Command::ApplySyncedTotal {
            synced_through_ms,
            server_total_ms,
        } => {
            state
                .listening
                .apply_synced(synced_through_ms, server_total_ms);
            push_persist_listening(state, effects);
        }
        Command::ResetListeningData => {
            state.listening.reset();
            push_persist_listening(state, effects);
        }
    }
}

fn start_playback(state: &mut State, effects: &mut Vec<Effect>) {
    if state.intent.is_playing() {
        return;
    }
    state.intent = PlaybackIntent::Playing;
    // Never start muted — a fresh play is always audible.
    state.muted = false;
    effects.push(Effect::StartPlayback {
        volume_percent: state.effective_volume(),
    });
    push_persist(state, effects);
}

fn stop_playback(state: &mut State, effects: &mut Vec<Effect>) {
    if !state.intent.is_playing() {
        return;
    }
    state.intent = PlaybackIntent::Paused;
    // Mute is a live-playback concern; clear it so the next play isn't silent.
    state.muted = false;
    // Audio is no longer confirmed playing, so listening stops accruing until
    // the platform reports it started again.
    state.audio_confirmed_playing = false;
    effects.push(Effect::PausePlayback);
    push_persist(state, effects);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dispatch(state: &mut State, cmd: Command) -> Vec<Effect> {
        let mut effects = Vec::new();
        reduce(state, cmd, &mut effects);
        effects
    }

    #[test]
    fn toggle_starts_then_pauses() {
        let mut s = State::default();
        let e1 = dispatch(&mut s, Command::TogglePlayback);
        assert!(s.intent.is_playing());
        assert!(matches!(e1[0], Effect::StartPlayback { .. }));

        let e2 = dispatch(&mut s, Command::TogglePlayback);
        assert!(!s.intent.is_playing());
        assert!(matches!(e2[0], Effect::PausePlayback));
    }

    #[test]
    fn play_when_already_playing_is_noop() {
        let mut s = State::default();
        dispatch(&mut s, Command::Play);
        let again = dispatch(&mut s, Command::Play);
        assert!(again.is_empty(), "redundant play should not emit effects");
    }

    #[test]
    fn mute_silences_output_without_pausing_and_keeps_timer() {
        let mut s = State::default();
        dispatch(&mut s, Command::SetVolume { percent: 80 });
        dispatch(&mut s, Command::StartPomodoro { minutes: 30 }); // auto-plays

        // Mute → output 0, still playing, timer untouched.
        let muted = dispatch(&mut s, Command::ToggleMute);
        assert!(s.muted);
        assert!(s.intent.is_playing(), "mute must not pause playback");
        assert!(s.active_timer.is_some(), "mute must not cancel the timer");
        assert!(muted
            .iter()
            .any(|e| matches!(e, Effect::SetPlatformVolume { volume_percent: 0 })));

        // Timer keeps counting while muted.
        dispatch(&mut s, Command::Tick { elapsed_ms: 60_000 });
        assert!(s.active_timer.is_some());

        // Unmute → output restored to the stored volume.
        let unmuted = dispatch(&mut s, Command::ToggleMute);
        assert!(!s.muted);
        assert!(unmuted
            .iter()
            .any(|e| matches!(e, Effect::SetPlatformVolume { volume_percent: 80 })));
    }

    #[test]
    fn set_volume_unmutes() {
        let mut s = State::default();
        dispatch(&mut s, Command::Play);
        dispatch(&mut s, Command::ToggleMute);
        assert!(s.muted);
        dispatch(&mut s, Command::SetVolume { percent: 50 });
        assert!(!s.muted, "adjusting volume should unmute");
    }

    #[test]
    fn stopping_playback_clears_mute() {
        let mut s = State::default();
        dispatch(&mut s, Command::Play);
        dispatch(&mut s, Command::ToggleMute);
        dispatch(&mut s, Command::Pause);
        assert!(
            !s.muted,
            "pausing should clear mute so the next play is audible"
        );
    }

    #[test]
    fn set_volume_clamps_and_persists() {
        let mut s = State::default();
        let effects = dispatch(&mut s, Command::SetVolume { percent: 250 });
        assert_eq!(s.volume_percent, Some(100));
        assert!(effects.iter().any(|e| matches!(
            e,
            Effect::SetPlatformVolume {
                volume_percent: 100
            }
        )));
        assert!(effects
            .iter()
            .any(|e| matches!(e, Effect::PersistSettings { .. })));
    }

    #[test]
    fn pomodoro_auto_starts_playback() {
        let mut s = State::default();
        let effects = dispatch(&mut s, Command::StartPomodoro { minutes: 30 });
        assert!(s.intent.is_playing());
        assert!(effects
            .iter()
            .any(|e| matches!(e, Effect::StartPlayback { .. })));
        let t = s.active_timer.as_ref().unwrap();
        assert_eq!(t.kind, TimerKind::Pomodoro);
        assert_eq!(t.total_ms, 30 * 60_000);
    }

    #[test]
    fn stopwatch_counts_up_without_touching_playback() {
        let mut s = State::default();
        dispatch(&mut s, Command::Play);
        let effects = dispatch(&mut s, Command::StartStopwatch);
        // Starting a stopwatch emits no playback effects and keeps playing.
        assert!(effects.is_empty());
        assert!(s.intent.is_playing());
        let t = s.active_timer.as_ref().unwrap();
        assert_eq!(t.kind, TimerKind::Stopwatch);

        // Ticks accumulate elapsed; nothing pauses, ever.
        dispatch(&mut s, Command::Tick { elapsed_ms: 65_000 });
        let snap = crate::Snapshot::from_state(&s);
        assert_eq!(snap.timer.kind, crate::TimerSnapshotKind::Stopwatch);
        assert_eq!(snap.timer.remaining_label, "1:05");
        assert!(s.intent.is_playing());

        // Cancel stops it.
        dispatch(&mut s, Command::CancelTimer);
        assert!(s.active_timer.is_none());
    }

    #[test]
    fn sleep_timer_pauses_playback_on_expiry() {
        let mut s = State::default();
        dispatch(&mut s, Command::Play);
        dispatch(&mut s, Command::StartSleepTimer { minutes: 1 });
        // 30s in, still playing
        dispatch(&mut s, Command::Tick { elapsed_ms: 30_000 });
        assert!(s.intent.is_playing());
        // Cross the boundary.
        let effects = dispatch(&mut s, Command::Tick { elapsed_ms: 31_000 });
        assert!(!s.intent.is_playing());
        assert!(s.active_timer.is_none());
        assert_eq!(s.timer_just_completed, Some(TimerKind::Sleep));
        assert!(effects.iter().any(|e| matches!(e, Effect::PausePlayback)));
    }

    #[test]
    fn just_completed_clears_on_next_dispatch() {
        let mut s = State::default();
        dispatch(&mut s, Command::StartPomodoro { minutes: 1 });
        dispatch(&mut s, Command::Tick { elapsed_ms: 60_001 });
        assert_eq!(s.timer_just_completed, Some(TimerKind::Pomodoro));
        // Any subsequent dispatch (even another Tick) clears the flag.
        dispatch(&mut s, Command::Tick { elapsed_ms: 1_000 });
        assert!(s.timer_just_completed.is_none());
    }

    #[test]
    fn platform_pause_syncs_intent() {
        let mut s = State::default();
        dispatch(&mut s, Command::Play);
        assert!(s.intent.is_playing());
        dispatch(&mut s, Command::PlatformPlaybackPaused);
        assert!(!s.intent.is_playing());
    }

    #[test]
    fn platform_error_surfaces_and_pauses() {
        let mut s = State::default();
        dispatch(&mut s, Command::Play);
        dispatch(
            &mut s,
            Command::PlatformPlaybackError {
                message: "decode failed".into(),
            },
        );
        assert!(!s.intent.is_playing());
        assert_eq!(s.last_error.as_deref(), Some("decode failed"));
    }

    #[test]
    fn cancel_timer_does_not_pause_playback() {
        let mut s = State::default();
        dispatch(&mut s, Command::Play);
        dispatch(&mut s, Command::StartSleepTimer { minutes: 15 });
        dispatch(&mut s, Command::CancelTimer);
        assert!(s.active_timer.is_none());
        assert!(s.intent.is_playing());
    }

    // ---- listening-time accrual -------------------------------------------

    #[test]
    fn intent_alone_does_not_accrue_until_audio_confirmed() {
        let mut s = State::default();
        dispatch(&mut s, Command::Play); // intent only — autoplay may be blocked
        dispatch(&mut s, Command::Tick { elapsed_ms: 1_000 });
        assert_eq!(
            s.listening.device_total_ms, 0,
            "must not count time before the platform confirms audio started"
        );
        // Platform confirms → now ticks count.
        dispatch(&mut s, Command::PlatformPlaybackStarted);
        let effects = dispatch(&mut s, Command::Tick { elapsed_ms: 1_000 });
        assert_eq!(s.listening.device_total_ms, 1_000);
        assert!(
            effects
                .iter()
                .any(|e| matches!(e, Effect::PersistListening { .. })),
            "a counted tick should persist the listening blob"
        );
    }

    #[test]
    fn muting_stops_accrual_but_not_the_timer() {
        let mut s = State::default();
        dispatch(&mut s, Command::Play);
        dispatch(&mut s, Command::PlatformPlaybackStarted);
        dispatch(&mut s, Command::Tick { elapsed_ms: 1_000 });
        assert_eq!(s.listening.device_total_ms, 1_000);
        dispatch(&mut s, Command::ToggleMute);
        let effects = dispatch(&mut s, Command::Tick { elapsed_ms: 1_000 });
        assert_eq!(
            s.listening.device_total_ms, 1_000,
            "muted time is not listening"
        );
        assert!(!effects
            .iter()
            .any(|e| matches!(e, Effect::PersistListening { .. })));
    }

    #[test]
    fn disabling_tracking_stops_accrual_without_erasing_total() {
        let mut s = State::default();
        dispatch(&mut s, Command::Play);
        dispatch(&mut s, Command::PlatformPlaybackStarted);
        dispatch(&mut s, Command::Tick { elapsed_ms: 2_000 });
        dispatch(&mut s, Command::SetListeningTracking { enabled: false });
        dispatch(&mut s, Command::Tick { elapsed_ms: 2_000 });
        assert_eq!(
            s.listening.device_total_ms, 2_000,
            "off = no new accrual, no erase"
        );
    }

    #[test]
    fn pause_then_resume_requires_fresh_confirmation_to_accrue() {
        let mut s = State::default();
        dispatch(&mut s, Command::Play);
        dispatch(&mut s, Command::PlatformPlaybackStarted);
        dispatch(&mut s, Command::Tick { elapsed_ms: 1_000 });
        dispatch(&mut s, Command::Pause);
        // Paused: ticks don't count, and confirmation was cleared.
        dispatch(&mut s, Command::Tick { elapsed_ms: 5_000 });
        assert_eq!(s.listening.device_total_ms, 1_000);
        // Resume intent without confirmation still doesn't count.
        dispatch(&mut s, Command::Play);
        dispatch(&mut s, Command::Tick { elapsed_ms: 5_000 });
        assert_eq!(s.listening.device_total_ms, 1_000);
        // Fresh confirmation re-enables accrual.
        dispatch(&mut s, Command::PlatformPlaybackStarted);
        dispatch(&mut s, Command::Tick { elapsed_ms: 1_000 });
        assert_eq!(s.listening.device_total_ms, 2_000);
    }

    #[test]
    fn restore_listening_loads_blob_without_lowering_live_total() {
        let mut s = State::default();
        let blob = serde_json::to_string(&crate::listening::PersistedListening {
            version: crate::listening::LISTENING_VERSION,
            device_total_ms: 7_200_000,
            synced_through_ms: 3_600_000,
            server_total_ms: Some(10_000_000),
            tracking_enabled: false,
        })
        .unwrap();
        dispatch(&mut s, Command::RestoreListening { json: blob });
        assert_eq!(s.listening.device_total_ms, 7_200_000);
        assert_eq!(s.listening.server_total_ms, Some(10_000_000));
        assert!(!s.listening.tracking_enabled);
    }

    #[test]
    fn apply_synced_total_moves_baseline_and_persists() {
        let mut s = State::default();
        dispatch(&mut s, Command::Play);
        dispatch(&mut s, Command::PlatformPlaybackStarted);
        dispatch(&mut s, Command::Tick { elapsed_ms: 5_000 });
        let effects = dispatch(
            &mut s,
            Command::ApplySyncedTotal {
                synced_through_ms: 5_000,
                server_total_ms: 9_000_000,
            },
        );
        assert_eq!(s.listening.unsynced_ms(), 0);
        assert_eq!(s.listening.displayed_total_ms(), 9_000_000);
        assert!(effects
            .iter()
            .any(|e| matches!(e, Effect::PersistListening { .. })));
    }

    #[test]
    fn reset_listening_data_zeros_slot_keeps_toggle() {
        let mut s = State::default();
        dispatch(&mut s, Command::Play);
        dispatch(&mut s, Command::PlatformPlaybackStarted);
        dispatch(&mut s, Command::Tick { elapsed_ms: 5_000 });
        dispatch(&mut s, Command::ResetListeningData);
        assert_eq!(s.listening.device_total_ms, 0);
        assert!(s.listening.tracking_enabled);
    }
}
