//! Thin `wasm-bindgen` wrapper around `cascade-core`.
//!
//! Every value crosses the boundary as JSON. That's deliberate — it forces the
//! Rust API to stay coarse-grained (the discipline we want to validate before
//! we build Clave), and it keeps the TypeScript side honest about what shapes
//! the core actually returns.

use cascade_core::{Command, Core, Snapshot, Update};
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

    /// Render the current snapshot without dispatching anything.
    #[wasm_bindgen]
    pub fn snapshot(&self) -> Result<JsValue, JsValue> {
        snapshot_to_js(&self.inner.snapshot())
    }

    /// Dispatch a [`Command`] (encoded as JSON because typed bindings make
    /// the boundary chatty). Returns the [`Update`] as a plain JS object.
    #[wasm_bindgen]
    pub fn dispatch(&mut self, command_json: &str) -> Result<JsValue, JsValue> {
        let command: Command = serde_json::from_str(command_json)
            .map_err(|e| JsValue::from_str(&format!("bad command JSON: {e}")))?;
        let update = self.inner.dispatch(command);
        update_to_js(&update)
    }
}

impl Default for CascadeCore {
    fn default() -> Self {
        Self::new()
    }
}

fn snapshot_to_js(snap: &Snapshot) -> Result<JsValue, JsValue> {
    serde_wasm_bindgen::to_value(snap).map_err(|e| JsValue::from_str(&e.to_string()))
}

fn update_to_js(update: &Update) -> Result<JsValue, JsValue> {
    serde_wasm_bindgen::to_value(update).map_err(|e| JsValue::from_str(&e.to_string()))
}
