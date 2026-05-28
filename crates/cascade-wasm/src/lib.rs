//! Thin `wasm-bindgen` wrapper around `cascade-core`.
//!
//! Every value crosses the boundary as a JSON string and is parsed on the JS
//! side. That gives us deterministic serde semantics (in particular,
//! `#[serde(rename_all = "camelCase")]` always wins) instead of negotiating
//! with `serde-wasm-bindgen`'s edge cases for tagged enums. It also matches
//! the architecture brief's "JSON across the boundary" recommendation.

use cascade_core::{Command, Core};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn init() {
    #[cfg(feature = "panic-hook")]
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub struct CascadeCore {
    inner: Core,
}

#[wasm_bindgen]
impl CascadeCore {
    /// Fresh session with default settings.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self { inner: Core::new() }
    }

    /// Restore from previously persisted settings JSON. Throws if the JSON is
    /// unparseable or comes from an incompatible settings version.
    #[wasm_bindgen]
    pub fn restore(settings_json: &str) -> Result<CascadeCore, JsError> {
        Core::restore(settings_json)
            .map(|inner| CascadeCore { inner })
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Render the current snapshot as JSON.
    #[wasm_bindgen]
    pub fn snapshot(&self) -> Result<String, JsValue> {
        serde_json::to_string(&self.inner.snapshot())
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Dispatch a [`Command`] (JSON in, JSON-encoded `Update` out).
    #[wasm_bindgen]
    pub fn dispatch(&mut self, command_json: &str) -> Result<String, JsValue> {
        let command: Command = serde_json::from_str(command_json)
            .map_err(|e| JsValue::from_str(&format!("bad command JSON: {e}")))?;
        let update = self.inner.dispatch(command);
        serde_json::to_string(&update).map_err(|e| JsValue::from_str(&e.to_string()))
    }
}

impl Default for CascadeCore {
    fn default() -> Self {
        Self::new()
    }
}
