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
    /// `ListeningSyncFailed`, or no further sync will start.
    PushListening {
        device_id: String,
        device_total_ms: u64,
    },
    /// The server rejected the session (401). The shell drops its stored
    /// session token and tells the user to sign in again.
    ClearSession,
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
