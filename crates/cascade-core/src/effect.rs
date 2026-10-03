//! Side-effect requests the core hands back to the platform layer.
//!
//! The core itself never plays audio, never persists files, never reads the
//! clock. It declares *what should happen*; the platform decides *how*.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Effect {
    /// Begin (or resume) playback of the waterfall loop at the given output
    /// gain (0.0–1.0, curve already applied).
    StartPlayback { gain: f32 },
    /// Stop / pause playback.
    PausePlayback,
    /// Set the shell's output gain (0.0–1.0, mute and curve already applied)
    /// without changing play/pause state.
    SetPlatformVolume { gain: f32 },
    /// Persist the supplied settings JSON. The platform decides where
    /// (localStorage, DataStore, file system, …), and hands it back via
    /// [`crate::Command::Restore`] on the next launch.
    PersistSettings { json: String },
    /// Persist the listening blob. Stored in its own slot
    /// (`cascade.listening.v1`), separate from settings, so the two evolve and
    /// fail independently. The platform stores the string verbatim and hands it
    /// back via [`crate::Command::Restore`] on the next launch.
    PersistListening { json: String },
    /// PUT this device's slot to the listening endpoint, exactly as given.
    /// Emitted only in answer to [`crate::Command::BeginListeningSync`]; the
    /// shell must settle it with `ListeningSyncSucceeded` or
    /// `ListeningSyncFailed`, or no further sync will start. `session_token`
    /// is the bearer token to send; it is `None` only for a shell that
    /// restores without `accountJson`.
    PushListening {
        device_id: String,
        device_total_ms: u64,
        session_token: Option<String>,
    },

    /// POST a sign-in link request for `email`. Settle with
    /// `SignInLinkSent` or `AccountRequestFailed`.
    SendSignInLink { email: String },
    /// POST `token` for verification. Settle with `SignInVerified` or
    /// `AccountRequestFailed`.
    VerifySignInToken { token: String },
    /// POST a logout for `session_token`. Fire-and-forget: there is no settle
    /// command, and its result changes nothing — sign-out is local.
    RevokeSession { session_token: String },
    /// DELETE the server's listening data. Settle with `ListeningDataDeleted`
    /// or `AccountRequestFailed`.
    DeleteServerListening { session_token: String },
    /// DELETE the account. Settle with `AccountDeleted` or
    /// `AccountRequestFailed`.
    DeleteServerAccount { session_token: String },
    /// Persist the account blob, verbatim, and hand it back as `accountJson`
    /// in [`crate::Command::Restore`] on the next launch. An empty `json`
    /// means "delete the stored account".
    PersistAccount { json: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    // The shells read fields like `gain` off the JSON
    // payload. If serde stops camelCasing variant fields, the platforms get
    // `undefined` and the slider stops working — silently. Lock the wire shape.
    #[test]
    fn set_platform_volume_serializes_camel_case() {
        let json = serde_json::to_string(&Effect::SetPlatformVolume { gain: 0.25 }).unwrap();
        assert_eq!(json, r#"{"type":"setPlatformVolume","gain":0.25}"#);
    }

    #[test]
    fn start_playback_serializes_camel_case() {
        let json = serde_json::to_string(&Effect::StartPlayback { gain: 0.36 }).unwrap();
        assert_eq!(json, r#"{"type":"startPlayback","gain":0.36}"#);
    }
}
