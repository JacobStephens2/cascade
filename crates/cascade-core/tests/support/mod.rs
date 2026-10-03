//! Shared helpers for suites that carry the core's server requests: find the
//! `ServerRequest`s an update emitted, and build the `ServerResponse` a shell
//! would settle one with.

#![allow(dead_code)]

use cascade_core::{Command, Effect};

/// Every `ServerRequest` in `effects`, in order.
pub fn requests(effects: &[Effect]) -> Vec<&Effect> {
    effects
        .iter()
        .filter(|e| matches!(e, Effect::ServerRequest { .. }))
        .collect()
}

/// The id of the one `ServerRequest` in `effects`. Panics unless there is
/// exactly one.
pub fn request_id(effects: &[Effect]) -> u64 {
    match requests(effects).as_slice() {
        [Effect::ServerRequest { id, .. }] => *id,
        other => panic!("expected one server request, got {other:?}"),
    }
}

/// The response a shell dispatches for request `id`: the HTTP `status` and a
/// JSON `body`.
pub fn response(id: u64, status: u16, body: serde_json::Value) -> Command {
    Command::ServerResponse {
        id,
        status,
        body: body.to_string(),
    }
}

/// A response with no body: an empty 2xx, a failure status, or `0` for a
/// request that was never sent or got no answer.
pub fn bare_response(id: u64, status: u16) -> Command {
    Command::ServerResponse {
        id,
        status,
        body: String::new(),
    }
}
