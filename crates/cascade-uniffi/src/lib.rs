//! Cascade core ↔ Kotlin bridge via UniFFI.
//!
//! The wire shape mirrors what the WASM crate does: every value crosses the
//! boundary as a JSON string. Two reasons:
//!
//! 1. **One source of truth.** The web and Android shells parse the same
//!    JSON; the Rust core does not have to maintain a parallel typed FFI
//!    surface that drifts away from the serde shape.
//! 2. **Coarse boundary.** A single `dispatch(command_json) -> update_json`
//!    is what the architecture brief recommends — fewer FFI calls, less
//!    chatter, easier to test.
//!
//! Persisted-settings IO is the platform's responsibility (the Android shell
//! stores `Effect::PersistSettings.json` in Jetpack DataStore).
//!
//! All errors come back as [`CascadeError`] so Kotlin can `try { … } catch
//! (CascadeException e) { … }` rather than parsing a string.

use std::sync::Mutex;

use cascade_core::{Command, Core};
use thiserror::Error;

uniffi::setup_scaffolding!();

#[derive(Debug, Error, uniffi::Error)]
pub enum CascadeError {
    // Field is named `reason` (not `message`) so that the generated Kotlin
    // exception class doesn't clash with `kotlin.Exception.message`.
    #[error("bad JSON: {reason}")]
    BadJson { reason: String },
    #[error("internal core error: {reason}")]
    Core { reason: String },
}

/// The UniFFI handle. Kotlin holds one of these; every UI event funnels
/// through `dispatch`, and every render reads from `snapshot`.
#[derive(uniffi::Object)]
pub struct CascadeBridge {
    inner: Mutex<Core>,
}

#[uniffi::export]
impl CascadeBridge {
    /// Fresh session with default settings.
    #[uniffi::constructor]
    pub fn new() -> std::sync::Arc<Self> {
        std::sync::Arc::new(Self {
            inner: Mutex::new(Core::new()),
        })
    }

    /// Restore from previously persisted settings JSON. Returns a fresh-default
    /// bridge if the JSON cannot be parsed — the caller is expected to log /
    /// surface the error and move on, not get stuck in a startup loop.
    #[uniffi::constructor]
    pub fn restore_or_new(settings_json: String) -> std::sync::Arc<Self> {
        let inner = Core::restore(&settings_json).unwrap_or_else(|_| Core::new());
        std::sync::Arc::new(Self {
            inner: Mutex::new(inner),
        })
    }

    /// Render the current snapshot as JSON.
    pub fn snapshot(&self) -> Result<String, CascadeError> {
        let core = self.inner.lock().expect("core mutex poisoned");
        serde_json::to_string(&core.snapshot()).map_err(|e| CascadeError::Core {
            reason: e.to_string(),
        })
    }

    /// Dispatch a JSON-encoded [`Command`] and return the resulting `Update`
    /// as JSON. Same wire shape as the WASM dispatch — the two shells share
    /// the same TypeScript / Kotlin DTOs (just regenerated per platform).
    pub fn dispatch(&self, command_json: String) -> Result<String, CascadeError> {
        let command: Command =
            serde_json::from_str(&command_json).map_err(|e| CascadeError::BadJson {
                reason: e.to_string(),
            })?;
        let mut core = self.inner.lock().expect("core mutex poisoned");
        let update = core.dispatch(command);
        serde_json::to_string(&update).map_err(|e| CascadeError::Core {
            reason: e.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatch_play_emits_start_playback() {
        let bridge = CascadeBridge::new();
        let update_json = bridge.dispatch(r#"{"type":"play"}"#.to_string()).unwrap();
        assert!(update_json.contains(r#""type":"startPlayback""#));
        assert!(update_json.contains(r#""volumePercent":"#));
    }
}
