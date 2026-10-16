//! Side-effect requests the core hands back to the platform layer.
//!
//! The core itself never plays audio, never persists files, never reads the
//! clock. It declares *what should happen*; the platform decides *how*.

use serde::{Deserialize, Serialize};

use crate::server::HttpMethod;

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
    /// Send one request to the sync server, exactly as described: `method`
    /// to `path` (relative to the shell's sync-server base URL), with
    /// `Authorization: Bearer {bearer_token}` when there is one, and `body`
    /// verbatim as `application/json` when there is one. The core owns the
    /// request table (see [`crate::server`]); the shell decides nothing per
    /// request.
    ///
    /// The shell settles every request, with no exceptions, by dispatching
    /// [`crate::Command::ServerResponse`] with the same `id`: the HTTP status
    /// and the response body, or status `0` when it was not sent or got no
    /// response (no sync server configured, a network error, a timeout).
    /// Until it is settled, the core's in-flight guard holds the next account
    /// request or listening sync. A response the core is no longer waiting
    /// for (a sign-out revoke, or a request sign-out or a reset superseded)
    /// changes nothing.
    ServerRequest {
        id: u64,
        method: HttpMethod,
        path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        bearer_token: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        body: Option<String>,
    },
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
