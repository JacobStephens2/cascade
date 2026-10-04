//! The Account — the optional sign-in (an email and a session) that syncs
//! listening time across a user's devices.
//!
//! The core holds the session and every account rule: what to send, when a
//! request may start, what the user is told, and what success, failure and a
//! 401 do. A shell only carries the requests it is handed (each one generic
//! request, settled with one generic response) and stores the blob.
//!
//! The [`Account`] answers each account intent and each of its responses
//! itself, and tells the reducer only what follows for listening and storage,
//! as one [`AccountChange`].

use serde::{Deserialize, Serialize};

use crate::effect::Effect;
use crate::server::{parse_verified, Request, RequestIds, StatusClass};
use crate::snapshot::AccountSnapshot;

/// Storage-schema version for the persisted account blob.
pub const ACCOUNT_VERSION: u32 = 1;

/// A signed-in session. The token never reaches the snapshot; it only rides on
/// the effects that need it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    pub session_token: String,
    pub email: String,
}

/// The one account request the shell is carrying, if any, and the id its
/// response will carry. Only one runs at a time.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Pending {
    id: u64,
    request: PendingRequest,
}

/// What a pending account request is for, and what its success needs.
#[derive(Debug, Clone, PartialEq, Eq)]
enum PendingRequest {
    SignInLink {
        email: String,
    },
    Verify,
    /// Holds the fresh id to rotate to once the server confirms the delete.
    DeleteListening {
        new_device_id: String,
    },
    DeleteAccount {
        new_device_id: String,
    },
}

/// What the user was last told about their account. [`Self::label`] holds the
/// copy, so every shell shows the same words.
#[derive(Debug, Clone, PartialEq, Eq)]
enum AccountStatus {
    EnterEmail,
    LinkSent { email: String },
    LinkFailed,
    PasteFullLink,
    SigningIn,
    SignedIn { email: String },
    LinkInvalid,
    SessionExpired,
    ListeningDeleted,
    ListeningDeleteFailed,
    AccountDeleted,
    AccountDeleteFailed,
}

impl AccountStatus {
    fn label(&self) -> String {
        match self {
            Self::EnterEmail => "Enter your email address.".into(),
            Self::LinkSent { email } => format!("Check {email} for a sign-in link."),
            Self::LinkFailed => "Couldn't send the sign-in link. Try again.".into(),
            Self::PasteFullLink => "Paste the full sign-in link.".into(),
            Self::SigningIn => "Signing in…".into(),
            Self::SignedIn { email } => format!("Signed in as {email}."),
            Self::LinkInvalid => "That sign-in link was invalid or expired.".into(),
            Self::SessionExpired => "Signed out — sign in again to sync.".into(),
            Self::ListeningDeleted => "Listening data deleted.".into(),
            Self::ListeningDeleteFailed => "Couldn't delete listening data. Try again.".into(),
            Self::AccountDeleted => "Account deleted.".into(),
            Self::AccountDeleteFailed => "Couldn't delete the account. Try again.".into(),
        }
    }
}

/// What an account transition means for listening and storage. The reducer
/// carries it through to the listening ledger and the persisted blobs; an
/// outcome that only changes what the user is told is not a change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountChange {
    /// A session was verified. `replaced` when it signed in over another
    /// account, whose cross-device total must not show.
    SignedIn { replaced: bool },
    /// The session is gone: signed out, or a 401.
    SignedOut,
    /// The server deleted this device's listening data: rotate the slot.
    ListeningDeleted { new_device_id: String },
    /// The server deleted the account: rotate the slot, and the session is
    /// gone.
    AccountDeleted { new_device_id: String },
}

/// The account as the core holds it: the session, the one pending request,
/// and what the user was last told. Held inside [`crate::State`]; every
/// account rule lives here.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Account {
    session: Option<Session>,
    pending: Option<Pending>,
    status: Option<AccountStatus>,
}

impl Account {
    /// Take the session from a stored blob: the versioned shape, or the
    /// version-less one web and Apple stored before the core held the
    /// account. Empty, garbage, an unknown version or an empty token mean no
    /// account.
    pub fn restore(&mut self, json: &str) {
        self.session = session_from_json(json);
    }

    pub fn session_token(&self) -> Option<String> {
        self.session.as_ref().map(|s| s.session_token.clone())
    }

    /// The account view for the UI. Never carries the session token.
    pub fn snapshot(&self) -> AccountSnapshot {
        let email = self.session.as_ref().map(|s| s.email.clone());
        AccountSnapshot {
            signed_in_label: email.as_ref().map(|e| format!("Syncing · {e}")),
            email,
            status_label: self.status.as_ref().map(AccountStatus::label),
            busy: self.busy(),
        }
    }

    /// The blob for [`crate::Effect::PersistAccount`]: empty when signed out,
    /// meaning "delete the stored account".
    pub fn to_json(&self) -> String {
        self.session
            .as_ref()
            .and_then(|s| {
                serde_json::to_string(&PersistedAccount {
                    version: Some(ACCOUNT_VERSION),
                    session_token: s.session_token.clone(),
                    email: s.email.clone(),
                })
                .ok()
            })
            .unwrap_or_default()
    }

    /// Ask the server to email a sign-in link to `email`, trimmed. A blank
    /// email asks for one instead.
    pub fn request_sign_in_link(
        &mut self,
        email: &str,
        platform: Option<&str>,
        ids: &mut RequestIds,
        effects: &mut Vec<Effect>,
    ) {
        if self.busy() {
            return;
        }
        let email = email.trim().to_string();
        if email.is_empty() {
            self.status = Some(AccountStatus::EnterEmail);
            return;
        }
        let request = Request::sign_in_link(&email, platform);
        self.begin(
            PendingRequest::SignInLink { email },
            None,
            request,
            ids,
            effects,
        );
    }

    /// Redeem the sign-in token found in what the user pasted.
    pub fn submit_sign_in_link(
        &mut self,
        input: &str,
        ids: &mut RequestIds,
        effects: &mut Vec<Effect>,
    ) {
        if self.busy() {
            return;
        }
        match parse_sign_in_token(input) {
            Some(token) => self.begin(
                PendingRequest::Verify,
                Some(AccountStatus::SigningIn),
                Request::verify(&token),
                ids,
                effects,
            ),
            None => self.status = Some(AccountStatus::PasteFullLink),
        }
    }

    /// Ask the server to delete this account's listening data. The slot
    /// rotates to `new_device_id` only once the server confirms.
    pub fn delete_listening(
        &mut self,
        new_device_id: String,
        ids: &mut RequestIds,
        effects: &mut Vec<Effect>,
    ) {
        if self.busy() {
            return;
        }
        if let Some(session_token) = self.session_token() {
            self.begin(
                PendingRequest::DeleteListening { new_device_id },
                None,
                Request::delete_listening(session_token),
                ids,
                effects,
            );
        }
    }

    /// Ask the server to delete the account. The slot rotates to
    /// `new_device_id` only once the server confirms.
    pub fn delete_account(
        &mut self,
        new_device_id: String,
        ids: &mut RequestIds,
        effects: &mut Vec<Effect>,
    ) {
        if self.busy() {
            return;
        }
        if let Some(session_token) = self.session_token() {
            self.begin(
                PendingRequest::DeleteAccount { new_device_id },
                None,
                Request::delete_account(session_token),
                ids,
                effects,
            );
        }
    }

    /// Sign out, cutting in on any pending request. Revokes the session if
    /// there is one; nothing waits for the revoke's response.
    pub fn sign_out(
        &mut self,
        ids: &mut RequestIds,
        effects: &mut Vec<Effect>,
    ) -> Option<AccountChange> {
        self.pending = None;
        self.status = None;
        let session = self.session.take()?;
        ids.send(Request::revoke(session.session_token), effects);
        Some(AccountChange::SignedOut)
    }

    /// Whether the pending request is request `id`.
    pub fn awaits(&self, id: u64) -> bool {
        self.pending.as_ref().is_some_and(|p| p.id == id)
    }

    /// Settle the pending request with its response. The caller has already
    /// matched the response's id with [`Self::awaits`]. A 401 while signed in
    /// expires the session; any other failure only changes what the user is
    /// told.
    pub fn settle(&mut self, class: StatusClass, body: &str) -> Option<AccountChange> {
        let request = self.pending.take()?.request;
        if class == StatusClass::Unauthorized && self.session.is_some() {
            return Some(self.expire());
        }
        let succeeded = class == StatusClass::Success;
        let (status, change) = match request {
            PendingRequest::SignInLink { email } if succeeded => {
                (AccountStatus::LinkSent { email }, None)
            }
            PendingRequest::Verify if succeeded => match parse_verified(body) {
                Some(session) => {
                    let status = AccountStatus::SignedIn {
                        email: session.email.clone(),
                    };
                    let replaced = self.session.replace(session).is_some();
                    (status, Some(AccountChange::SignedIn { replaced }))
                }
                None => (AccountStatus::LinkInvalid, None),
            },
            PendingRequest::DeleteListening { new_device_id } if succeeded => (
                AccountStatus::ListeningDeleted,
                Some(AccountChange::ListeningDeleted { new_device_id }),
            ),
            PendingRequest::DeleteAccount { new_device_id } if succeeded => {
                self.session = None;
                (
                    AccountStatus::AccountDeleted,
                    Some(AccountChange::AccountDeleted { new_device_id }),
                )
            }
            PendingRequest::SignInLink { .. } => (AccountStatus::LinkFailed, None),
            PendingRequest::Verify => (AccountStatus::LinkInvalid, None),
            PendingRequest::DeleteListening { .. } => (AccountStatus::ListeningDeleteFailed, None),
            PendingRequest::DeleteAccount { .. } => (AccountStatus::AccountDeleteFailed, None),
        };
        self.status = Some(status);
        change
    }

    /// The server rejected the session (a 401 on any request, including a
    /// listening push): sign out and tell the user to sign in again. No
    /// revoke, since the session is already gone server-side.
    pub fn expire(&mut self) -> AccountChange {
        self.pending = None;
        self.status = Some(AccountStatus::SessionExpired);
        self.session = None;
        AccountChange::SignedOut
    }

    /// An account request is out; every account intent but sign-out waits.
    fn busy(&self) -> bool {
        self.pending.is_some()
    }

    /// Send the one account request, remembering its `purpose`.
    fn begin(
        &mut self,
        purpose: PendingRequest,
        status: Option<AccountStatus>,
        request: Request,
        ids: &mut RequestIds,
        effects: &mut Vec<Effect>,
    ) {
        let id = ids.send(request, effects);
        self.pending = Some(Pending {
            id,
            request: purpose,
        });
        self.status = status;
    }
}

/// Read a stored account blob into a session; see [`Account::restore`].
fn session_from_json(json: &str) -> Option<Session> {
    let stored: PersistedAccount = serde_json::from_str(json).ok()?;
    if stored.version.is_some_and(|v| v != ACCOUNT_VERSION) || stored.session_token.is_empty() {
        return None;
    }
    Some(Session {
        session_token: stored.session_token,
        email: stored.email,
    })
}

/// The persisted account blob. `version` is optional on read only, so the
/// version-less legacy shape loads; the core always writes it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PersistedAccount {
    #[serde(default)]
    pub version: Option<u32>,
    pub session_token: String,
    pub email: String,
}

/// Find the sign-in token in what the user pasted: a link carrying a query
/// parameter named exactly `token`, or the bare token itself.
fn parse_sign_in_token(input: &str) -> Option<String> {
    let input = input.trim();
    if let Some(value) = token_query_value(input) {
        let token = percent_decode(value);
        return (!token.is_empty()).then_some(token);
    }
    let is_raw_token = !input.is_empty()
        && !input
            .chars()
            .any(|c| c.is_whitespace() || matches!(c, '/' | '?' | '&' | '='));
    is_raw_token.then(|| input.to_string())
}

/// The raw value of the first `token=` that follows a `?` or `&`, up to the
/// next `&` or `#`.
fn token_query_value(input: &str) -> Option<&str> {
    const KEY: &str = "token=";
    input.match_indices(KEY).find_map(|(at, _)| {
        let preceded = input[..at].ends_with(['?', '&']);
        preceded.then(|| {
            let rest = &input[at + KEY.len()..];
            rest.split(['&', '#']).next().unwrap_or_default()
        })
    })
}

/// Decode `%XX` escapes. A malformed escape is kept as written.
fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = bytes
            .get(i + 1..i + 3)
            .and_then(|h| std::str::from_utf8(h).ok())
            .and_then(|h| u8::from_str_radix(h, 16).ok());
        match (bytes[i], hex) {
            (b'%', Some(byte)) => {
                out.push(byte);
                i += 3;
            }
            (byte, _) => {
                out.push(byte);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blob_serializes_camel_case_with_a_version() {
        let account = Account {
            session: Some(Session {
                session_token: "t".into(),
                email: "a@b.c".into(),
            }),
            ..Account::default()
        };
        assert_eq!(
            account.to_json(),
            r#"{"version":1,"sessionToken":"t","email":"a@b.c"}"#
        );
    }

    #[test]
    fn a_signed_out_blob_is_empty() {
        assert_eq!(Account::default().to_json(), "");
    }

    #[test]
    fn percent_decode_keeps_malformed_escapes() {
        assert_eq!(percent_decode("a%2Fb"), "a/b");
        assert_eq!(percent_decode("100%"), "100%");
        assert_eq!(percent_decode("%zz"), "%zz");
    }
}
