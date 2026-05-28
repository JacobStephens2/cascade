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

// ---------------------------------------------------------------------------
// C ABI for the Windows shell.
//
// UniFFI doesn't ship first-party C# bindings — `uniffi-bindgen-cs` is a
// community tool and lags Mozilla's other generators. Rather than depend on
// it, the Windows C# project loads this DLL via `[DllImport]` and calls a
// hand-rolled coarse C surface. Same wire shape (JSON in, JSON out) as the
// UniFFI-driven shells; same `Core` underneath.
//
// All returned strings are heap-allocated by Rust and **must** be freed by
// calling `cascade_free_string`. All handles must be freed by
// `cascade_free_handle` exactly once.
// ---------------------------------------------------------------------------

mod c_abi {
    use super::*;
    use std::ffi::{CStr, CString};
    use std::os::raw::c_char;
    use std::ptr;
    use std::sync::Mutex;

    pub struct CoreHandle {
        inner: Mutex<Core>,
    }

    fn into_cstr(s: String) -> *mut c_char {
        match CString::new(s) {
            Ok(c) => c.into_raw(),
            Err(_) => ptr::null_mut(),
        }
    }

    #[no_mangle]
    pub extern "C" fn cascade_new() -> *mut CoreHandle {
        Box::into_raw(Box::new(CoreHandle {
            inner: Mutex::new(Core::new()),
        }))
    }

    /// `settings_json` may be `NULL` for a fresh-default session. Unparseable
    /// JSON also falls back to a fresh default — the caller is expected to log
    /// it once and move on, not loop on the error.
    ///
    /// # Safety
    /// `settings_json`, if non-null, must point to a valid C string (null-
    /// terminated UTF-8).
    #[no_mangle]
    pub unsafe extern "C" fn cascade_restore_or_new(
        settings_json: *const c_char,
    ) -> *mut CoreHandle {
        let core = if settings_json.is_null() {
            Core::new()
        } else {
            match CStr::from_ptr(settings_json).to_str() {
                Ok(s) => Core::restore(s).unwrap_or_else(|_| Core::new()),
                Err(_) => Core::new(),
            }
        };
        Box::into_raw(Box::new(CoreHandle {
            inner: Mutex::new(core),
        }))
    }

    /// # Safety
    /// `handle` must come from `cascade_new` / `cascade_restore_or_new` and not
    /// have been freed.
    #[no_mangle]
    pub unsafe extern "C" fn cascade_snapshot(handle: *mut CoreHandle) -> *mut c_char {
        if handle.is_null() {
            return ptr::null_mut();
        }
        let handle = &*handle;
        let core = handle.inner.lock().expect("core mutex poisoned");
        match serde_json::to_string(&core.snapshot()) {
            Ok(s) => into_cstr(s),
            Err(_) => ptr::null_mut(),
        }
    }

    /// Dispatch a JSON-encoded command and return the JSON-encoded `Update`.
    /// Returns `NULL` if the command JSON is unparseable; check via the
    /// non-null contract.
    ///
    /// # Safety
    /// `handle` and `command_json` must be non-null and valid.
    #[no_mangle]
    pub unsafe extern "C" fn cascade_dispatch(
        handle: *mut CoreHandle,
        command_json: *const c_char,
    ) -> *mut c_char {
        if handle.is_null() || command_json.is_null() {
            return ptr::null_mut();
        }
        let json = match CStr::from_ptr(command_json).to_str() {
            Ok(s) => s,
            Err(_) => return ptr::null_mut(),
        };
        let command: Command = match serde_json::from_str(json) {
            Ok(c) => c,
            Err(_) => return ptr::null_mut(),
        };
        let handle = &*handle;
        let mut core = handle.inner.lock().expect("core mutex poisoned");
        let update = core.dispatch(command);
        match serde_json::to_string(&update) {
            Ok(s) => into_cstr(s),
            Err(_) => ptr::null_mut(),
        }
    }

    /// # Safety
    /// `s` must be a pointer previously returned by `cascade_snapshot` or
    /// `cascade_dispatch`. Calling this twice on the same pointer is UB.
    #[no_mangle]
    pub unsafe extern "C" fn cascade_free_string(s: *mut c_char) {
        if !s.is_null() {
            drop(CString::from_raw(s));
        }
    }

    /// # Safety
    /// `handle` must be a pointer previously returned by `cascade_new` or
    /// `cascade_restore_or_new`. Calling this twice on the same handle is UB.
    #[no_mangle]
    pub unsafe extern "C" fn cascade_free_handle(handle: *mut CoreHandle) {
        if !handle.is_null() {
            drop(Box::from_raw(handle));
        }
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

    #[test]
    fn c_abi_round_trip() {
        use std::ffi::{CStr, CString};

        unsafe {
            let handle = c_abi::cascade_new();
            assert!(!handle.is_null());

            let snap = c_abi::cascade_snapshot(handle);
            assert!(!snap.is_null());
            let snap_str = CStr::from_ptr(snap).to_str().unwrap();
            assert!(snap_str.contains(r#""isPlaying":false"#));
            c_abi::cascade_free_string(snap);

            let cmd = CString::new(r#"{"type":"play"}"#).unwrap();
            let upd = c_abi::cascade_dispatch(handle, cmd.as_ptr());
            assert!(!upd.is_null());
            let upd_str = CStr::from_ptr(upd).to_str().unwrap();
            assert!(upd_str.contains(r#""type":"startPlayback""#));
            c_abi::cascade_free_string(upd);

            c_abi::cascade_free_handle(handle);
        }
    }
}
