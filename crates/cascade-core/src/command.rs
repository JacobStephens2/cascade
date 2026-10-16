//! User and platform commands that drive the core.

use serde::{Deserialize, Serialize};

use crate::listening::SyncReason;

/// Everything that can happen to the core. User actions, platform reports,
/// and wall-clock ticks all funnel through here.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Command {
    /// User asked to start playback.
    Play,
    /// User asked to pause playback.
    Pause,
    /// User tapped the primary button; flip between Play and Pause.
    TogglePlayback,
    /// Set volume in the range `0..=100`. Values outside the range are clamped.
    /// Adjusting volume also clears the muted state.
    SetVolume { percent: u8 },
    /// Toggle mute: silence audio output without pausing the session, so a
    /// running sleep/pomodoro timer keeps counting down. Implemented as a
    /// platform-volume change (output 0 while muted), not a pause — playback
    /// stays live so the timer and background audio session survive.
    ToggleMute,

    /// Start a plain sleep timer that pauses playback after `minutes` minutes.
    /// `minutes` is clamped into 1–1440 (so `0` starts a one-minute timer;
    /// [`Command::CancelTimer`] is what cancels) and saved as the new default.
    StartSleepTimer { minutes: u32 },
    /// Start a pomodoro / focus session of `minutes` minutes. When the timer
    /// expires playback pauses; the UI can distinguish "session complete" from
    /// "sleep" via [`crate::TimerSnapshotKind`]. `minutes` is clamped and saved
    /// like [`Command::StartSleepTimer`]'s.
    StartPomodoro { minutes: u32 },
    /// Start a count-up stopwatch so the user can see how long they've been
    /// listening. Replaces any running timer, never expires, and does not
    /// change playback. Stop it with [`Command::CancelTimer`].
    StartStopwatch,
    /// Cancel any running timer (countdown or stopwatch) without touching playback.
    CancelTimer,

    /// Wall-clock tick from the platform. `elapsed_ms` is the delta since the
    /// previous tick. The core never reads the system clock; the UI ticks it.
    /// A tick that counts listening also offers the routine sync, so its
    /// update may carry a listening push ([`crate::Effect::ServerRequest`])
    /// to settle.
    Tick { elapsed_ms: u64 },

    /// Platform reports that audio actually started playing (e.g. the browser
    /// finally honored a play() promise, or ExoPlayer reported STATE_READY).
    PlatformPlaybackStarted,
    /// Platform reports playback paused (user pressed media key, audio focus
    /// loss, end of file in a non-looping mode, etc.).
    PlatformPlaybackPaused,
    /// Platform reports a playback error that the user should know about.
    PlatformPlaybackError { message: String },

    /// Turn listening-time tracking on or off. On by default; this is the
    /// opt-out. Turning it off stops accrual immediately but never erases the
    /// total already counted — use [`Command::ResetListeningData`] for that.
    SetListeningTracking { enabled: bool },
    /// Restore persisted data at startup. Every shell boots the same way: a
    /// fresh core, then this command exactly once. Only the first restore a
    /// core receives takes effect; any later one changes nothing and returns
    /// no effects, so a stale blob can never undo a reset.
    ///
    /// `settings_json` and `listening_json` are the opaque strings previous
    /// [`crate::Effect::PersistSettings`] and [`crate::Effect::PersistListening`]
    /// effects handed the shell; the core owns their schemas, so the shell
    /// stores and returns them verbatim. An empty string means "no blob". A
    /// missing, unparseable or unknown-version blob is ignored and the defaults
    /// stay in place — restore raises no error, and never *lowers* a live
    /// listening counter. Settings restore touches only the persisted fields
    /// and emits no `PersistSettings` or volume effect: the core is paused.
    ///
    /// The core owns the device id but has no randomness, so the shell supplies
    /// `fallback_device_id`: a fresh random id (or, once, the id the shell
    /// used to store itself, so an existing server slot carries over). It is
    /// adopted — and persisted at once — only if the listening blob carries no
    /// id.
    ///
    /// `account_json` is the string the last [`crate::Effect::PersistAccount`]
    /// handed the shell, or the version-less `{ sessionToken, email }` a shell
    /// stored before the core held the account. Empty, garbage or missing
    /// means no account.
    Restore {
        settings_json: String,
        listening_json: String,
        fallback_device_id: String,
        #[serde(default)]
        account_json: String,
    },
    /// The shell is able to talk to the server (online) and asks whether
    /// there is anything to send. If so, the update carries one
    /// [`crate::Effect::ServerRequest`]: the listening push, PUT `/listening`.
    /// The shell carries it and settles it with [`Command::ServerResponse`].
    /// No effect means nothing to send, a sync is already in flight, or the
    /// core holds the account and is signed out.
    BeginListeningSync { reason: SyncReason },
    /// "Delete my listening data": zero this device's slot, forget the server
    /// aggregate, and rotate to `new_device_id` (a fresh random id from the
    /// shell) — all in one persisted write, so a crash can't leave a fresh id
    /// holding the old total, and a stale offline write can't resurrect the
    /// deleted one. Leaves the tracking toggle untouched.
    ResetListeningData { new_device_id: String },

    /// User asked for a sign-in link. The email is trimmed; an empty one is
    /// refused with a status and no effect. Answered with a
    /// [`crate::Effect::ServerRequest`] (POST `/auth/request`). `platform`
    /// names the platform whose app the emailed link should hand off to
    /// (the Windows shell sends `"windows"`), and is added to the request
    /// body only when given.
    ///
    /// Like every account request, it is ignored while another account
    /// request is pending (`account.busy`).
    RequestSignInLink {
        email: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        platform: Option<String>,
    },
    /// User pasted or opened a sign-in link: either the whole link or the bare
    /// token. Answered with a [`crate::Effect::ServerRequest`] (POST
    /// `/auth/verify`). Once verified, the core persists the account and
    /// starts a refresh sync for the cross-device total.
    SubmitSignInLink { input: String },
    /// Sign out locally. Always works, even while a request is pending: that
    /// request is dropped and its response ignored. Forgets the cross-device
    /// total and asks the server to revoke the session (POST `/auth/logout`),
    /// whose response changes nothing.
    SignOut,
    /// User asked to delete their synced listening data. Answered with a
    /// [`crate::Effect::ServerRequest`] (DELETE `/listening`). Once the server
    /// confirms, the core zeroes the slot and rotates to `new_device_id` (a
    /// fresh random id from the shell) in one persisted write, like
    /// [`Command::ResetListeningData`]. Held until then, so the slot only
    /// rotates once the server has actually deleted.
    DeleteListeningData { new_device_id: String },
    /// User asked to delete their account. Answered with a
    /// [`crate::Effect::ServerRequest`] (DELETE `/account`). Once the server
    /// confirms, the core resets listening like [`Command::DeleteListeningData`]
    /// and signs out.
    DeleteAccount { new_device_id: String },
    /// The shell's answer to a [`crate::Effect::ServerRequest`]: its `id`, the
    /// HTTP `status` (`0` when it was not sent or got no response) and the
    /// response `body` verbatim (empty when there is none). Every request is
    /// settled this way. The core routes it by `id` and reads it: any 2xx is
    /// success, a 401 while signed in signs out and tells the user to sign in
    /// again, and anything else, or a 2xx whose body does not parse, is a
    /// plain failure. A response the core is not waiting for changes nothing.
    ServerResponse {
        id: u64,
        status: u16,
        #[serde(default)]
        body: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_round_trips_camel_case() {
        // The JS hook dispatches `{ type: "tick", elapsedMs: 250 }`. Make sure
        // the Rust side actually accepts that shape (it's the only command with
        // a multi-word field, so it's the easy one to break).
        let cmd: Command = serde_json::from_str(r#"{"type":"tick","elapsedMs":250}"#).unwrap();
        assert_eq!(cmd, Command::Tick { elapsed_ms: 250 });
    }
}
