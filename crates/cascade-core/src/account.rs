//! The Account — the optional sign-in (an email and a session) that syncs
//! listening time across a user's devices.
//!
//! The core holds the session and every account rule: what to send, when a
//! request may start, what the user is told, and what success, failure and a
//! 401 do. A shell only carries the requests it is handed (one effect per
//! request, settled with one outcome command) and stores the blob.

use serde::{Deserialize, Serialize};

/// Storage-schema version for the persisted account blob.
pub const ACCOUNT_VERSION: u32 = 1;

/// A signed-in session. The token never reaches the snapshot; it only rides on
/// the effects that need it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    pub session_token: String,
    pub email: String,
}

/// The one account request the shell is carrying, if any. Only one runs at a
/// time, so its outcome command needs no request identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PendingRequest {
    SignInLink { email: String },
    Verify,
    DeleteListening,
    DeleteAccount,
}

/// What the user was last told about their account. [`Self::label`] holds the
/// copy, so every shell shows the same words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountStatus {
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
    pub fn label(&self) -> String {
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

/// The account as the core holds it. Held inside [`crate::State`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Account {
    /// Whether the shell has handed the account to the core, by restoring
    /// with an `accountJson` (even an empty one). A shell that has not been
    /// ported keeps its own account: the core then syncs without a session
    /// and answers a 401 with [`crate::Effect::ClearSession`], as before.
    pub held_by_core: bool,
    pub session: Option<Session>,
    pub pending: Option<PendingRequest>,
    pub status: Option<AccountStatus>,
}

impl Account {
    /// An account request is out; every account command but sign-out waits.
    pub fn busy(&self) -> bool {
        self.pending.is_some()
    }

    pub fn session_token(&self) -> Option<String> {
        self.session.as_ref().map(|s| s.session_token.clone())
    }

    /// Whether a listening sync may start: the core holds a session, or the
    /// shell still keeps the account itself.
    pub fn may_sync(&self) -> bool {
        !self.held_by_core || self.session.is_some()
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

    /// Read a stored blob: the versioned shape, or the version-less one web
    /// and Apple stored before the core held the account. Empty, garbage, an
    /// unknown version or an empty token mean no account.
    pub fn session_from_json(json: &str) -> Option<Session> {
        let stored: PersistedAccount = serde_json::from_str(json).ok()?;
        if stored.version.is_some_and(|v| v != ACCOUNT_VERSION) || stored.session_token.is_empty() {
            return None;
        }
        Some(Session {
            session_token: stored.session_token,
            email: stored.email,
        })
    }
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
pub fn parse_sign_in_token(input: &str) -> Option<String> {
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
