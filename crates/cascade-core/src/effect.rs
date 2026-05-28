//! Side-effect requests the core hands back to the platform layer.
//!
//! The core itself never plays audio, never persists files, never reads the
//! clock. It declares *what should happen*; the platform decides *how*.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Effect {
    /// Begin (or resume) playback of the waterfall loop at the given volume.
    StartPlayback {
        volume_percent: u8,
    },
    /// Stop / pause playback.
    PausePlayback,
    /// Update the platform's volume control without changing play/pause state.
    SetPlatformVolume {
        volume_percent: u8,
    },
    /// Persist the supplied settings JSON. The platform decides where
    /// (localStorage, DataStore, file system, …).
    PersistSettings {
        json: String,
    },
}
