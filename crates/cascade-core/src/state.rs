//! Internal state and the reducer.
//!
//! The reducer is the only place that mutates state. It is a pure function of
//! `(state, command) -> effects`, and it never reads the system clock, file
//! system, or network.

use serde::{Deserialize, Serialize};

use crate::account::{parse_sign_in_token, Account, AccountStatus, PendingRequest, Session};
use crate::command::Command;
use crate::effect::Effect;
use crate::listening::{ListeningLedger, PersistedListening, SyncReason};
use crate::settings::{PersistedSettings, SETTINGS_VERSION};
use crate::timer::{clamp_minutes, ActiveTimer, TimerKind, DEFAULT_TIMER_MINUTES};

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
    /// Set on the tick that an active timer hit zero, and cleared only by a
    /// command that changes playback intent or the timer (see
    /// [`clears_timer_completion`]). Surfaces in snapshots as
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
    /// [`Command::Restore`], separately from settings.
    pub listening: ListeningLedger,
    /// The optional sign-in. Restored from its own blob via
    /// [`Command::Restore`]'s `account_json`.
    pub account: Account,
    /// Whether [`Command::Restore`] has already run. Only the first restore
    /// takes effect, so a replayed stale blob can't undo a later reset.
    restored: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            intent: PlaybackIntent::Paused,
            volume_percent: None,
            muted: false,
            active_timer: None,
            timer_just_completed: None,
            default_sleep_minutes: Some(DEFAULT_TIMER_MINUTES),
            default_pomodoro_minutes: Some(DEFAULT_TIMER_MINUTES),
            last_error: None,
            audio_confirmed_playing: false,
            listening: ListeningLedger::default(),
            account: Account::default(),
            restored: false,
        }
    }
}

impl State {
    /// Take the persisted fields from `s`. Playback intent, timers and mute
    /// are live-session state and stay as they are.
    pub fn apply_settings(&mut self, s: &PersistedSettings) {
        self.volume_percent = Some(clamp_volume(s.volume_percent));
        self.default_sleep_minutes = s.default_sleep_minutes;
        self.default_pomodoro_minutes = s.default_pomodoro_minutes;
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

    /// The gain the shell should actually output right now, 0.0–1.0: zero
    /// while muted, otherwise the user's chosen level through the perceptual
    /// curve.
    pub fn output_gain(&self) -> f32 {
        if self.muted {
            0.0
        } else {
            percent_to_gain(self.effective_volume())
        }
    }
}

/// The perceptual (square-law) volume curve: gain = (percent / 100)². The
/// square is taken on integers so the one rounding step gives the shortest
/// float (60 % → 0.36, not 0.36000002).
fn percent_to_gain(percent: u8) -> f32 {
    let p = u32::from(percent);
    (p * p) as f32 / 10_000.0
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

fn push_persist_account(state: &State, effects: &mut Vec<Effect>) {
    effects.push(Effect::PersistAccount {
        json: state.account.to_json(),
    });
}

/// Start a listening sync if `begin` finds anything to send and the account
/// allows it, answering with one `PushListening`.
fn begin_listening_sync(
    state: &mut State,
    effects: &mut Vec<Effect>,
    begin: impl FnOnce(&mut ListeningLedger) -> Option<(String, u64)>,
) {
    let Some(session_token) = state.account.session_token() else {
        return;
    };
    if let Some((device_id, device_total_ms)) = begin(&mut state.listening) {
        effects.push(Effect::PushListening {
            device_id,
            device_total_ms,
            session_token,
        });
    }
}

/// Start the one account request, answering with `request`. The caller has
/// already checked that none is pending.
fn begin_account_request(
    state: &mut State,
    pending: PendingRequest,
    status: Option<AccountStatus>,
    request: Effect,
    effects: &mut Vec<Effect>,
) {
    state.account.pending = Some(pending);
    state.account.status = status;
    effects.push(request);
}

/// Drop the session and everything learned through it: the stored blob, the
/// cross-device total, and any sync still in flight for it.
fn forget_session(state: &mut State, effects: &mut Vec<Effect>) {
    state.account.session = None;
    push_persist_account(state, effects);
    state.listening.forget_server();
    push_persist_listening(state, effects);
}

/// Whether a tick should count as listening: tracking on, audio *confirmed*
/// playing (not merely intended), and not muted.
fn is_accruing(state: &State) -> bool {
    state.listening.tracking_enabled && state.audio_confirmed_playing && !state.muted
}

/// Whether `command` replaces a finished timer's message. Only the user's
/// playback and timer commands do: background traffic (ticks, sync and account
/// settles, platform reports) and small adjustments (volume, mute) leave it on
/// screen, since after expiry nothing else would ever show it.
/// Exhaustive on purpose, so a new command has to choose.
fn clears_timer_completion(command: &Command) -> bool {
    match command {
        Command::Play
        | Command::Pause
        | Command::TogglePlayback
        | Command::StartSleepTimer { .. }
        | Command::StartPomodoro { .. }
        | Command::StartStopwatch
        | Command::CancelTimer => true,
        Command::SetVolume { .. }
        | Command::ToggleMute
        | Command::Tick { .. }
        | Command::PlatformPlaybackStarted
        | Command::PlatformPlaybackPaused
        | Command::PlatformPlaybackError { .. }
        | Command::SetListeningTracking { .. }
        | Command::Restore { .. }
        | Command::BeginListeningSync { .. }
        | Command::ListeningSyncSucceeded { .. }
        | Command::ListeningSyncFailed { .. }
        | Command::ResetListeningData { .. }
        | Command::RequestSignInLink { .. }
        | Command::SubmitSignInLink { .. }
        | Command::SignOut
        | Command::DeleteListeningData
        | Command::DeleteAccount
        | Command::SignInLinkSent
        | Command::SignInVerified { .. }
        | Command::ListeningDataDeleted { .. }
        | Command::AccountDeleted { .. }
        | Command::AccountRequestFailed { .. } => false,
    }
}

/// The reducer. Mutates `state` and appends to `effects`.
pub fn reduce(state: &mut State, command: Command, effects: &mut Vec<Effect>) {
    if clears_timer_completion(&command) {
        state.timer_just_completed = None;
    }

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
            effects.push(Effect::SetPlatformVolume {
                gain: state.output_gain(),
            });
            push_persist(state, effects);
        }
        Command::ToggleMute => {
            state.muted = !state.muted;
            effects.push(Effect::SetPlatformVolume {
                gain: state.output_gain(),
            });
        }
        Command::StartSleepTimer { minutes } => {
            let minutes = clamp_minutes(minutes);
            state.active_timer = Some(ActiveTimer::start(TimerKind::Sleep, minutes));
            state.default_sleep_minutes = Some(minutes);
            push_persist(state, effects);
        }
        Command::StartPomodoro { minutes } => {
            let minutes = clamp_minutes(minutes);
            state.active_timer = Some(ActiveTimer::start(TimerKind::Pomodoro, minutes));
            state.default_pomodoro_minutes = Some(minutes);
            // Pomodoro implies "start playing now if not already".
            if !state.intent.is_playing() {
                start_playback(state, effects);
            }
            push_persist(state, effects);
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
            // so a process kill loses at most one tick of listening. Only a
            // counted tick can bring the unsynced total to the threshold, so
            // it is also the one place the routine sync is offered.
            if was_accruing {
                push_persist_listening(state, effects);
                begin_listening_sync(state, effects, ListeningLedger::begin_threshold_sync);
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
        Command::Restore {
            settings_json,
            listening_json,
            fallback_device_id,
            account_json,
        } => {
            if state.restored {
                return;
            }
            state.restored = true;
            // The shell hands back the opaque blobs it stored. A missing or
            // unparseable/unknown-version blob is ignored — the defaults are
            // already correct, and restore must never lower a live counter.
            if let Ok(settings) = serde_json::from_str::<PersistedSettings>(&settings_json) {
                if settings.version == SETTINGS_VERSION {
                    state.apply_settings(&settings);
                }
            }
            if let Ok(restored) = serde_json::from_str::<PersistedListening>(&listening_json) {
                if restored.version == crate::listening::LISTENING_VERSION {
                    let ledger = ListeningLedger::from_persisted(&restored);
                    state.listening.restore_from(&ledger);
                }
            }
            // Persist a newly adopted id right away, so the next launch sees
            // the same slot rather than another fallback.
            let mut persist_listening = state.listening.adopt_device_id(fallback_device_id);
            state.account.session = Account::session_from_json(&account_json);
            // A cross-device total with no account behind it is left over
            // from a sign-out before the core held the account.
            if state.account.session.is_none() && state.listening.server_total_ms.is_some() {
                state.listening.forget_server();
                persist_listening = true;
            }
            if persist_listening {
                push_persist_listening(state, effects);
            }
        }
        Command::BeginListeningSync { reason } => {
            begin_listening_sync(state, effects, |l| l.begin_sync(reason))
        }
        Command::ListeningSyncSucceeded { server_total_ms } => {
            if state.listening.sync_succeeded(server_total_ms) {
                push_persist_listening(state, effects);
            }
        }
        Command::ListeningSyncFailed { unauthorized } => {
            // A superseded sync's 401 refers to a session already dropped.
            if state.listening.sync_failed() && unauthorized {
                session_expired(state, effects);
            }
        }
        Command::ResetListeningData { new_device_id } => {
            state.listening.reset(new_device_id);
            push_persist_listening(state, effects);
        }

        // Every account request waits for the pending one; only sign-out
        // cuts in.
        Command::RequestSignInLink { .. }
        | Command::SubmitSignInLink { .. }
        | Command::DeleteListeningData
        | Command::DeleteAccount
            if state.account.busy() => {}
        Command::RequestSignInLink { email } => {
            let email = email.trim().to_string();
            if email.is_empty() {
                state.account.status = Some(AccountStatus::EnterEmail);
                return;
            }
            let request = Effect::SendSignInLink {
                email: email.clone(),
            };
            begin_account_request(
                state,
                PendingRequest::SignInLink { email },
                None,
                request,
                effects,
            );
        }
        Command::SubmitSignInLink { input } => match parse_sign_in_token(&input) {
            Some(token) => begin_account_request(
                state,
                PendingRequest::Verify,
                Some(AccountStatus::SigningIn),
                Effect::VerifySignInToken { token },
                effects,
            ),
            None => state.account.status = Some(AccountStatus::PasteFullLink),
        },
        Command::DeleteListeningData => {
            if let Some(session_token) = state.account.session_token() {
                begin_account_request(
                    state,
                    PendingRequest::DeleteListening,
                    None,
                    Effect::DeleteServerListening { session_token },
                    effects,
                );
            }
        }
        Command::DeleteAccount => {
            if let Some(session_token) = state.account.session_token() {
                begin_account_request(
                    state,
                    PendingRequest::DeleteAccount,
                    None,
                    Effect::DeleteServerAccount { session_token },
                    effects,
                );
            }
        }
        Command::SignOut => {
            state.account.pending = None;
            state.account.status = None;
            if let Some(session_token) = state.account.session_token() {
                effects.push(Effect::RevokeSession { session_token });
                forget_session(state, effects);
            }
        }
        Command::SignInLinkSent => {
            if let Some(PendingRequest::SignInLink { email }) = &state.account.pending {
                state.account.status = Some(AccountStatus::LinkSent {
                    email: email.clone(),
                });
                state.account.pending = None;
            }
        }
        Command::SignInVerified {
            session_token,
            email,
        } => {
            if !state.account.settle(PendingRequest::Verify) {
                return;
            }
            // Signing in over another account: nothing of its total may show.
            if state.account.session.is_some() {
                state.listening.forget_server();
                push_persist_listening(state, effects);
            }
            state.account.status = Some(AccountStatus::SignedIn {
                email: email.clone(),
            });
            state.account.session = Some(Session {
                session_token,
                email,
            });
            push_persist_account(state, effects);
            begin_listening_sync(state, effects, |l| l.begin_sync(SyncReason::Refresh));
        }
        Command::ListeningDataDeleted { new_device_id } => {
            if !state.account.settle(PendingRequest::DeleteListening) {
                return;
            }
            state.account.status = Some(AccountStatus::ListeningDeleted);
            state.listening.reset(new_device_id);
            push_persist_listening(state, effects);
        }
        Command::AccountDeleted { new_device_id } => {
            if !state.account.settle(PendingRequest::DeleteAccount) {
                return;
            }
            state.account.status = Some(AccountStatus::AccountDeleted);
            // The reset forgets the server total and supersedes any in-flight
            // sync too, so the slot and the sign-out share one listening write.
            state.listening.reset(new_device_id);
            push_persist_listening(state, effects);
            state.account.session = None;
            push_persist_account(state, effects);
        }
        Command::AccountRequestFailed { unauthorized } => {
            let Some(pending) = state.account.pending.take() else {
                return;
            };
            if unauthorized && state.account.session.is_some() {
                session_expired(state, effects);
                return;
            }
            state.account.status = Some(match pending {
                PendingRequest::SignInLink { .. } => AccountStatus::LinkFailed,
                PendingRequest::Verify => AccountStatus::LinkInvalid,
                PendingRequest::DeleteListening => AccountStatus::ListeningDeleteFailed,
                PendingRequest::DeleteAccount => AccountStatus::AccountDeleteFailed,
            });
        }
    }
}

/// The server rejected the session (a 401 on any request): sign out and tell
/// the user to sign in again. Listening carries on locally.
fn session_expired(state: &mut State, effects: &mut Vec<Effect>) {
    state.account.pending = None;
    state.account.status = Some(AccountStatus::SessionExpired);
    forget_session(state, effects);
}

fn start_playback(state: &mut State, effects: &mut Vec<Effect>) {
    if state.intent.is_playing() {
        return;
    }
    state.intent = PlaybackIntent::Playing;
    // Never start muted — a fresh play is always audible.
    state.muted = false;
    effects.push(Effect::StartPlayback {
        gain: state.output_gain(),
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
            .any(|e| matches!(e, Effect::SetPlatformVolume { gain } if *gain == 0.0)));

        // Timer keeps counting while muted.
        dispatch(&mut s, Command::Tick { elapsed_ms: 60_000 });
        assert!(s.active_timer.is_some());

        // Unmute → output restored to the stored volume.
        let unmuted = dispatch(&mut s, Command::ToggleMute);
        assert!(!s.muted);
        assert!(unmuted
            .iter()
            .any(|e| matches!(e, Effect::SetPlatformVolume { gain } if *gain == 0.64)));
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
            Effect::SetPlatformVolume { gain } if *gain == 1.0
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
    fn just_completed_stays_until_the_user_acts() {
        let mut s = State::default();
        dispatch(&mut s, Command::StartPomodoro { minutes: 1 });
        dispatch(&mut s, Command::Tick { elapsed_ms: 60_001 });
        assert_eq!(s.timer_just_completed, Some(TimerKind::Pomodoro));
        // Ticks and platform reports leave it; pressing play clears it.
        dispatch(&mut s, Command::Tick { elapsed_ms: 1_000 });
        dispatch(&mut s, Command::PlatformPlaybackPaused);
        assert_eq!(s.timer_just_completed, Some(TimerKind::Pomodoro));
        dispatch(&mut s, Command::Play);
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
            device_id: Some("stored".into()),
        })
        .unwrap();
        dispatch(
            &mut s,
            Command::Restore {
                settings_json: String::new(),
                listening_json: blob,
                fallback_device_id: "fallback".into(),
                account_json: r#"{"sessionToken":"t","email":"a@b.c"}"#.into(),
            },
        );
        assert_eq!(s.listening.device_total_ms, 7_200_000);
        assert_eq!(s.listening.server_total_ms, Some(10_000_000));
        assert!(!s.listening.tracking_enabled);
        assert_eq!(s.listening.device_id.as_deref(), Some("stored"));
    }

    #[test]
    fn reset_listening_data_zeros_slot_keeps_toggle() {
        let mut s = State::default();
        dispatch(&mut s, Command::Play);
        dispatch(&mut s, Command::PlatformPlaybackStarted);
        dispatch(&mut s, Command::Tick { elapsed_ms: 5_000 });
        dispatch(
            &mut s,
            Command::ResetListeningData {
                new_device_id: "rotated".into(),
            },
        );
        assert_eq!(s.listening.device_total_ms, 0);
        assert!(s.listening.tracking_enabled);
    }
}
