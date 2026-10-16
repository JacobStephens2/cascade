//! The request table — every call the core makes to the sync server, and what
//! each response means.
//!
//! The core describes a request as one generic [`crate::Effect::ServerRequest`]
//! (method, path, bearer token, JSON body) and hears back through one generic
//! [`crate::Command::ServerResponse`] (HTTP status and body). Endpoints, bodies
//! and response parsing live here, once; a shell only sends what it is given
//! to its configured server and reports the status and body verbatim.

use serde::{Deserialize, Serialize};

use crate::account::Session;
use crate::effect::Effect;

/// The HTTP verbs the request table uses, sent uppercase.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Post,
    Put,
    Delete,
}

/// One row of the request table, before the core gives it an id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    method: HttpMethod,
    path: &'static str,
    bearer_token: Option<String>,
    body: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SignInLinkBody<'a> {
    email: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    platform: Option<&'a str>,
}

#[derive(Serialize)]
struct VerifyBody<'a> {
    token: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PushListeningBody<'a> {
    device_id: &'a str,
    device_total_ms: u64,
}

impl Request {
    /// POST `/auth/request` `{email[, platform]}`. `platform` names the
    /// shell's link hand-off, when it has one.
    pub fn sign_in_link(email: &str, platform: Option<&str>) -> Self {
        Self::post_json("/auth/request", &SignInLinkBody { email, platform })
    }

    /// POST `/auth/verify` `{token}`; read with [`parse_verified`].
    pub fn verify(token: &str) -> Self {
        Self::post_json("/auth/verify", &VerifyBody { token })
    }

    /// POST `/auth/logout` with the bearer token. Nothing waits for its
    /// response.
    pub fn revoke(session_token: String) -> Self {
        Self::bearer_only(HttpMethod::Post, "/auth/logout", session_token)
    }

    /// PUT `/listening` `{deviceId, deviceTotalMs}`; read with
    /// [`parse_server_total`].
    pub fn push_listening(device_id: &str, device_total_ms: u64, session_token: String) -> Self {
        Self {
            body: to_json(&PushListeningBody {
                device_id,
                device_total_ms,
            }),
            ..Self::bearer_only(HttpMethod::Put, "/listening", session_token)
        }
    }

    /// DELETE `/listening` with the bearer token.
    pub fn delete_listening(session_token: String) -> Self {
        Self::bearer_only(HttpMethod::Delete, "/listening", session_token)
    }

    /// DELETE `/account` with the bearer token.
    pub fn delete_account(session_token: String) -> Self {
        Self::bearer_only(HttpMethod::Delete, "/account", session_token)
    }

    fn post_json(path: &'static str, body: &impl Serialize) -> Self {
        Self {
            method: HttpMethod::Post,
            path,
            bearer_token: None,
            body: to_json(body),
        }
    }

    fn bearer_only(method: HttpMethod, path: &'static str, session_token: String) -> Self {
        Self {
            method,
            path,
            bearer_token: Some(session_token),
            body: None,
        }
    }

    /// The effect that asks the shell to carry this request as `id`.
    fn into_effect(self, id: u64) -> Effect {
        Effect::ServerRequest {
            id,
            method: self.method,
            path: self.path.into(),
            bearer_token: self.bearer_token,
            body: self.body,
        }
    }
}

/// The one source of request ids, held in [`crate::State`] and shared by every
/// module that sends. Starts at 1, is never persisted (a restart forgets every
/// request in flight along with the shell process that was carrying it), and
/// never reuses an id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestIds {
    next: u64,
}

impl Default for RequestIds {
    fn default() -> Self {
        Self { next: 1 }
    }
}

impl RequestIds {
    /// Hand the shell `request` under a fresh id, and return that id.
    pub fn send(&mut self, request: Request, effects: &mut Vec<Effect>) -> u64 {
        let id = self.next;
        self.next += 1;
        effects.push(request.into_effect(id));
        id
    }
}

fn to_json(body: &impl Serialize) -> Option<String> {
    serde_json::to_string(body).ok()
}

/// What a response's HTTP status means, before its body is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusClass {
    /// Any 2xx.
    Success,
    /// 401: the session is gone.
    Unauthorized,
    /// Anything else, including `0` (not sent, or no response).
    Failed,
}

impl StatusClass {
    pub fn of(status: u16) -> Self {
        match status {
            200..=299 => Self::Success,
            401 => Self::Unauthorized,
            _ => Self::Failed,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct VerifiedBody {
    session_token: String,
    email: String,
}

/// The session a successful verify returned. `None` when the body does not
/// parse or carries an empty token, which is no session.
pub fn parse_verified(body: &str) -> Option<Session> {
    let verified: VerifiedBody = serde_json::from_str(body).ok()?;
    (!verified.session_token.is_empty()).then_some(Session {
        session_token: verified.session_token,
        email: verified.email,
    })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PushedBody {
    server_total_ms: i64,
}

/// The cross-device total a successful push returned, read as a signed
/// integer and clamped at 0. `None` when the body does not parse.
pub fn parse_server_total(body: &str) -> Option<u64> {
    let pushed: PushedBody = serde_json::from_str(body).ok()?;
    Some(pushed.server_total_ms.max(0) as u64)
}
