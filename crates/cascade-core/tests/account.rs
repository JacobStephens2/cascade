//! The account flow, driven only through `Core::dispatch` — no network.
//!
//! The core holds the session and every account rule; the shell carries each
//! request effect and settles it with an outcome command. These tests pin the
//! core half: what is sent, when a request may start, what the user is told,
//! and what success, failure and a 401 do to the account and to listening.

use cascade_core::{AccountSnapshot, Command, Core, Effect, SyncReason};

const DEVICE_A: &str = "device-a";
const DEVICE_B: &str = "device-b";
const TOKEN: &str = "session-1";
const EMAIL: &str = "a@example.com";

fn blob(session_token: &str, email: &str) -> String {
    format!(r#"{{"version":1,"sessionToken":"{session_token}","email":"{email}"}}"#)
}

fn restore(account_json: &str) -> Command {
    Command::Restore {
        settings_json: String::new(),
        listening_json: String::new(),
        fallback_device_id: DEVICE_A.into(),
        account_json: account_json.into(),
    }
}

/// A core that holds the account and is signed out.
fn signed_out() -> Core {
    let mut core = Core::new();
    core.dispatch(restore(""));
    core
}

/// A core signed in as `EMAIL` with `TOKEN`.
fn signed_in() -> Core {
    let mut core = Core::new();
    core.dispatch(restore(&blob(TOKEN, EMAIL)));
    core
}

/// Accrue `ms` of confirmed listening.
fn listen(core: &mut Core, ms: u64) {
    core.dispatch(Command::Play);
    core.dispatch(Command::PlatformPlaybackStarted);
    let mut left = ms;
    while left > 0 {
        let step = left.min(1_000);
        core.dispatch(Command::Tick { elapsed_ms: step });
        left -= step;
    }
    core.dispatch(Command::Pause);
}

/// Start a sync and have the server report `server_total_ms`.
fn sync(core: &mut Core, server_total_ms: u64) {
    let effects = begin(core, SyncReason::Refresh);
    assert!(pushed(&effects).is_some(), "{effects:?}");
    core.dispatch(Command::ListeningSyncSucceeded { server_total_ms });
}

fn begin(core: &mut Core, reason: SyncReason) -> Vec<Effect> {
    core.dispatch(Command::BeginListeningSync { reason })
        .effects
}

fn pushed(effects: &[Effect]) -> Option<Effect> {
    effects
        .iter()
        .find(|e| matches!(e, Effect::PushListening { .. }))
        .cloned()
}

fn status(core: &Core) -> Option<String> {
    core.snapshot().account.status_label
}

fn persisted_accounts(effects: &[Effect]) -> Vec<String> {
    effects
        .iter()
        .filter_map(|e| match e {
            Effect::PersistAccount { json } => Some(json.clone()),
            _ => None,
        })
        .collect()
}

fn persisted_listening(effects: &[Effect]) -> Vec<String> {
    effects
        .iter()
        .filter_map(|e| match e {
            Effect::PersistListening { json } => Some(json.clone()),
            _ => None,
        })
        .collect()
}

/// Every account request command, so busy rules can be checked across all.
fn account_requests() -> Vec<Command> {
    vec![
        Command::RequestSignInLink {
            email: "b@example.com".into(),
        },
        Command::SubmitSignInLink {
            input: "raw-token".into(),
        },
        Command::DeleteListeningData,
        Command::DeleteAccount,
    ]
}

/// Every outcome command.
fn outcomes() -> Vec<Command> {
    vec![
        Command::SignInLinkSent,
        Command::SignInVerified {
            session_token: "late".into(),
            email: "late@example.com".into(),
        },
        Command::ListeningDataDeleted {
            new_device_id: DEVICE_B.into(),
        },
        Command::AccountDeleted {
            new_device_id: DEVICE_B.into(),
        },
        Command::AccountRequestFailed {
            unauthorized: false,
        },
        Command::AccountRequestFailed { unauthorized: true },
    ]
}

// ---- sign-in link ----------------------------------------------------------

#[test]
fn requesting_a_link_sends_the_trimmed_email_and_goes_busy() {
    let mut core = signed_out();
    let update = core.dispatch(Command::RequestSignInLink {
        email: "  a@example.com\n".into(),
    });
    assert_eq!(
        update.effects,
        vec![Effect::SendSignInLink {
            email: EMAIL.into()
        }]
    );
    assert!(update.snapshot.account.busy);
    assert_eq!(update.snapshot.account.status_label, None);
}

#[test]
fn an_empty_or_blank_email_is_refused() {
    for email in ["", "   ", "\n\t"] {
        let mut core = signed_out();
        let update = core.dispatch(Command::RequestSignInLink {
            email: email.into(),
        });
        assert!(update.effects.is_empty(), "{email:?}");
        assert!(!update.snapshot.account.busy, "{email:?}");
        assert_eq!(
            update.snapshot.account.status_label.as_deref(),
            Some("Enter your email address."),
            "{email:?}"
        );
    }
}

#[test]
fn a_sent_link_says_where_to_look() {
    let mut core = signed_out();
    core.dispatch(Command::RequestSignInLink {
        email: EMAIL.into(),
    });
    let update = core.dispatch(Command::SignInLinkSent);
    assert!(update.effects.is_empty());
    assert!(!update.snapshot.account.busy);
    assert_eq!(
        update.snapshot.account.status_label.as_deref(),
        Some("Check a@example.com for a sign-in link.")
    );
}

#[test]
fn a_failed_link_request_says_try_again() {
    let mut core = signed_out();
    core.dispatch(Command::RequestSignInLink {
        email: EMAIL.into(),
    });
    core.dispatch(Command::AccountRequestFailed {
        unauthorized: false,
    });
    assert_eq!(
        status(&core).as_deref(),
        Some("Couldn't send the sign-in link. Try again.")
    );
    assert!(!core.snapshot().account.busy);
}

// ---- token parser ----------------------------------------------------------

fn submitted_token(input: &str) -> Option<String> {
    let mut core = signed_out();
    let effects = core
        .dispatch(Command::SubmitSignInLink {
            input: input.into(),
        })
        .effects;
    effects.into_iter().find_map(|e| match e {
        Effect::VerifySignInToken { token } => Some(token),
        _ => None,
    })
}

#[test]
fn the_token_is_found_in_whatever_was_pasted() {
    let cases = [
        ("https://cascade.example/auth?token=abc123", Some("abc123")),
        (
            "https://cascade.example/?app=windows&token=abc123",
            Some("abc123"),
        ),
        ("  https://cascade.example/?token=abc123\n", Some("abc123")),
        ("cascade://auth?token=abc123", Some("abc123")),
        ("abc123", Some("abc123")),
        ("  abc123  ", Some("abc123")),
        ("https://cascade.example/?xtoken=abc123", None),
        (
            "https://cascade.example/?xtoken=bad&token=good",
            Some("good"),
        ),
        ("https://cascade.example/?token=abc123#done", Some("abc123")),
        (
            "https://cascade.example/?token=abc123&app=windows",
            Some("abc123"),
        ),
        ("https://cascade.example/?token=a%2Fb%3D", Some("a/b=")),
        ("https://cascade.example/?TOKEN=abc123", None),
        ("https://cascade.example/?token=", None),
        ("", None),
        ("   ", None),
        ("not a link", None),
        ("https://cascade.example/auth", None),
        ("key=value", None),
    ];
    for (input, expected) in cases {
        assert_eq!(submitted_token(input).as_deref(), expected, "{input:?}");
    }
}

#[test]
fn a_link_without_a_token_asks_for_the_full_link() {
    let mut core = signed_out();
    let update = core.dispatch(Command::SubmitSignInLink {
        input: "https://cascade.example/".into(),
    });
    assert!(update.effects.is_empty());
    assert!(!update.snapshot.account.busy);
    assert_eq!(
        update.snapshot.account.status_label.as_deref(),
        Some("Paste the full sign-in link.")
    );
}

// ---- verify ----------------------------------------------------------------

#[test]
fn submitting_a_link_says_signing_in_while_it_is_checked() {
    let mut core = signed_out();
    let update = core.dispatch(Command::SubmitSignInLink {
        input: "raw-token".into(),
    });
    assert_eq!(
        update.effects,
        vec![Effect::VerifySignInToken {
            token: "raw-token".into()
        }]
    );
    assert!(update.snapshot.account.busy);
    assert_eq!(
        update.snapshot.account.status_label.as_deref(),
        Some("Signing in…")
    );
}

#[test]
fn a_verified_sign_in_persists_the_account_and_refreshes_the_total() {
    let mut core = signed_out();
    listen(&mut core, 3_000);
    core.dispatch(Command::SubmitSignInLink {
        input: "raw-token".into(),
    });
    let update = core.dispatch(Command::SignInVerified {
        session_token: TOKEN.into(),
        email: EMAIL.into(),
    });
    assert_eq!(
        update.effects,
        vec![
            Effect::PersistAccount {
                json: blob(TOKEN, EMAIL)
            },
            Effect::PushListening {
                device_id: DEVICE_A.into(),
                device_total_ms: 3_000,
                session_token: TOKEN.into(),
            },
        ]
    );
    assert_eq!(
        update.snapshot.account,
        AccountSnapshot {
            email: Some(EMAIL.into()),
            signed_in_label: Some("Syncing · a@example.com".into()),
            status_label: Some("Signed in as a@example.com.".into()),
            busy: false,
        }
    );
}

#[test]
fn a_verified_sign_in_skips_the_refresh_while_a_sync_is_in_flight() {
    // Signed in, a sync goes out, then the user signs in again from a link.
    let mut core = signed_in();
    assert!(pushed(&begin(&mut core, SyncReason::Refresh)).is_some());
    core.dispatch(Command::SubmitSignInLink {
        input: "raw-token".into(),
    });
    let effects = core
        .dispatch(Command::SignInVerified {
            session_token: "session-2".into(),
            email: "b@example.com".into(),
        })
        .effects;
    assert!(pushed(&effects).is_none(), "{effects:?}");
    assert_eq!(
        persisted_accounts(&effects),
        vec![blob("session-2", "b@example.com")]
    );
}

#[test]
fn an_invalid_link_says_so() {
    let mut core = signed_out();
    core.dispatch(Command::SubmitSignInLink {
        input: "raw-token".into(),
    });
    let update = core.dispatch(Command::AccountRequestFailed {
        unauthorized: false,
    });
    assert!(update.effects.is_empty());
    assert_eq!(
        update.snapshot.account.status_label.as_deref(),
        Some("That sign-in link was invalid or expired.")
    );
    assert_eq!(update.snapshot.account.email, None);
}

#[test]
fn the_snapshot_never_carries_the_session_token() {
    let core = signed_in();
    let json = serde_json::to_string(&core.snapshot()).unwrap();
    assert!(!json.contains(TOKEN), "{json}");
}

#[test]
fn signed_out_the_account_section_is_empty() {
    assert_eq!(
        signed_out().snapshot().account,
        AccountSnapshot {
            email: None,
            signed_in_label: None,
            status_label: None,
            busy: false,
        }
    );
}

// ---- listening sync and the session ----------------------------------------

#[test]
fn nothing_syncs_while_signed_out() {
    let mut core = signed_out();
    listen(&mut core, 40_000);
    for reason in [
        SyncReason::Threshold,
        SyncReason::Flush,
        SyncReason::Refresh,
    ] {
        assert!(begin(&mut core, reason).is_empty(), "{reason:?}");
    }
}

#[test]
fn a_push_carries_the_session_token() {
    let mut core = signed_in();
    assert_eq!(
        pushed(&begin(&mut core, SyncReason::Refresh)),
        Some(Effect::PushListening {
            device_id: DEVICE_A.into(),
            device_total_ms: 0,
            session_token: TOKEN.into(),
        })
    );
}

// ---- sign-out --------------------------------------------------------------

#[test]
fn sign_out_revokes_forgets_and_persists() {
    let mut core = signed_in();
    listen(&mut core, 2_000);
    sync(&mut core, 1_000_000);
    assert_eq!(core.snapshot().listening.displayed_total_ms, 1_000_000);

    let update = core.dispatch(Command::SignOut);
    assert_eq!(
        update.effects.first(),
        Some(&Effect::RevokeSession {
            session_token: TOKEN.into()
        })
    );
    assert_eq!(persisted_accounts(&update.effects), vec![String::new()]);
    let listening = persisted_listening(&update.effects);
    assert_eq!(listening.len(), 1);
    assert!(
        !listening[0].contains(r#""serverTotalMs":1000000"#),
        "{}",
        listening[0]
    );
    assert_eq!(update.snapshot.listening.displayed_total_ms, 2_000);
    assert_eq!(update.snapshot.account.email, None);
    assert_eq!(update.snapshot.account.status_label, None);
}

#[test]
fn a_sync_ack_that_lands_after_sign_out_is_dropped() {
    let mut core = signed_in();
    listen(&mut core, 2_000);
    assert!(pushed(&begin(&mut core, SyncReason::Refresh)).is_some());
    core.dispatch(Command::SignOut);

    let snap = core
        .dispatch(Command::ListeningSyncSucceeded {
            server_total_ms: 1_000_000,
        })
        .snapshot;
    assert_eq!(snap.listening.displayed_total_ms, 2_000);
}

#[test]
fn a_second_account_shows_none_of_the_first_accounts_total() {
    let mut core = signed_in();
    listen(&mut core, 2_000);
    sync(&mut core, 1_000_000);
    core.dispatch(Command::SignOut);

    core.dispatch(Command::SubmitSignInLink {
        input: "raw-token".into(),
    });
    let snap = core
        .dispatch(Command::SignInVerified {
            session_token: "session-2".into(),
            email: "b@example.com".into(),
        })
        .snapshot;
    assert_eq!(snap.listening.displayed_total_ms, 2_000);
}

#[test]
fn signing_in_over_an_account_forgets_its_total() {
    let mut core = signed_in();
    listen(&mut core, 2_000);
    sync(&mut core, 1_000_000);

    core.dispatch(Command::SubmitSignInLink {
        input: "raw-token".into(),
    });
    let snap = core
        .dispatch(Command::SignInVerified {
            session_token: "session-2".into(),
            email: "b@example.com".into(),
        })
        .snapshot;
    assert_eq!(snap.listening.displayed_total_ms, 2_000);
}

#[test]
fn sign_out_while_signed_out_emits_nothing_but_clears_the_status() {
    let mut core = signed_out();
    core.dispatch(Command::RequestSignInLink { email: "".into() });
    let update = core.dispatch(Command::SignOut);
    assert!(update.effects.is_empty());
    assert_eq!(update.snapshot.account.status_label, None);
}

// ---- busy and re-entrancy --------------------------------------------------

#[test]
fn every_account_request_is_ignored_while_one_is_pending() {
    let pending_starts = [
        (
            signed_out(),
            Command::RequestSignInLink {
                email: EMAIL.into(),
            },
        ),
        (
            signed_out(),
            Command::SubmitSignInLink { input: "t".into() },
        ),
        (signed_in(), Command::DeleteListeningData),
        (signed_in(), Command::DeleteAccount),
    ];
    for (mut core, start) in pending_starts {
        core.dispatch(start.clone());
        let before = core.snapshot();
        assert!(before.account.busy, "{start:?}");
        for request in account_requests() {
            let update = core.dispatch(request.clone());
            assert!(update.effects.is_empty(), "{start:?} then {request:?}");
            assert_eq!(update.snapshot, before, "{start:?} then {request:?}");
        }
    }
}

#[test]
fn an_outcome_with_nothing_pending_changes_nothing() {
    for outcome in outcomes() {
        let mut core = signed_in();
        listen(&mut core, 2_000);
        sync(&mut core, 1_000_000);
        let before = core.snapshot();
        let update = core.dispatch(outcome.clone());
        assert!(update.effects.is_empty(), "{outcome:?}");
        assert_eq!(update.snapshot, before, "{outcome:?}");
    }
}

#[test]
fn an_outcome_for_another_request_changes_nothing() {
    // A delete is pending; a sign-in outcome can't be its answer.
    let mut core = signed_in();
    core.dispatch(Command::DeleteListeningData);
    let before = core.snapshot();
    for outcome in [
        Command::SignInLinkSent,
        Command::SignInVerified {
            session_token: "x".into(),
            email: "x@example.com".into(),
        },
        Command::AccountDeleted {
            new_device_id: DEVICE_B.into(),
        },
    ] {
        let update = core.dispatch(outcome.clone());
        assert!(update.effects.is_empty(), "{outcome:?}");
        assert_eq!(update.snapshot, before, "{outcome:?}");
    }
}

#[test]
fn sign_out_cuts_in_and_drops_the_pending_request() {
    let mut core = signed_in();
    core.dispatch(Command::DeleteAccount);
    let update = core.dispatch(Command::SignOut);
    assert!(!update.snapshot.account.busy);
    assert!(update.effects.contains(&Effect::RevokeSession {
        session_token: TOKEN.into()
    }));

    // The dropped request's late settle is ignored.
    let before = core.snapshot();
    let late = core.dispatch(Command::AccountDeleted {
        new_device_id: DEVICE_B.into(),
    });
    assert!(late.effects.is_empty());
    assert_eq!(late.snapshot, before);
}

#[test]
fn sign_out_drops_a_pending_sign_in() {
    let mut core = signed_out();
    core.dispatch(Command::SubmitSignInLink {
        input: "raw-token".into(),
    });
    core.dispatch(Command::SignOut);
    let update = core.dispatch(Command::SignInVerified {
        session_token: TOKEN.into(),
        email: EMAIL.into(),
    });
    assert!(update.effects.is_empty());
    assert_eq!(update.snapshot.account.email, None);
    assert!(!update.snapshot.account.busy);
}

#[test]
fn revoke_session_does_not_set_busy() {
    let mut core = signed_in();
    let snap = core.dispatch(Command::SignOut).snapshot;
    assert!(!snap.account.busy);
}

// ---- deletes ---------------------------------------------------------------

#[test]
fn deletes_carry_the_session_token() {
    let mut core = signed_in();
    let update = core.dispatch(Command::DeleteListeningData);
    assert_eq!(
        update.effects,
        vec![Effect::DeleteServerListening {
            session_token: TOKEN.into()
        }]
    );
    assert!(update.snapshot.account.busy);

    let mut core = signed_in();
    let update = core.dispatch(Command::DeleteAccount);
    assert_eq!(
        update.effects,
        vec![Effect::DeleteServerAccount {
            session_token: TOKEN.into()
        }]
    );
    assert!(update.snapshot.account.busy);
}

#[test]
fn deletes_do_nothing_while_signed_out() {
    for request in [Command::DeleteListeningData, Command::DeleteAccount] {
        let mut core = signed_out();
        let update = core.dispatch(request.clone());
        assert!(update.effects.is_empty(), "{request:?}");
        assert!(!update.snapshot.account.busy, "{request:?}");
    }
}

#[test]
fn deleted_listening_data_rotates_the_id_and_zeroes_the_slot_in_one_write() {
    let mut core = signed_in();
    listen(&mut core, 2_000);
    sync(&mut core, 1_000_000);
    core.dispatch(Command::DeleteListeningData);
    let update = core.dispatch(Command::ListeningDataDeleted {
        new_device_id: DEVICE_B.into(),
    });

    let writes = persisted_listening(&update.effects);
    assert_eq!(writes.len(), 1, "{:?}", update.effects);
    assert!(
        writes[0].contains(r#""deviceId":"device-b""#),
        "{}",
        writes[0]
    );
    assert!(writes[0].contains(r#""deviceTotalMs":0"#), "{}", writes[0]);
    assert!(persisted_accounts(&update.effects).is_empty());
    assert_eq!(update.snapshot.listening.displayed_total_ms, 0);
    assert_eq!(update.snapshot.account.email, Some(EMAIL.into()));
    assert_eq!(
        update.snapshot.account.status_label.as_deref(),
        Some("Listening data deleted.")
    );
    assert_eq!(
        pushed(&begin(&mut core, SyncReason::Refresh)),
        Some(Effect::PushListening {
            device_id: DEVICE_B.into(),
            device_total_ms: 0,
            session_token: TOKEN.into(),
        })
    );
}

#[test]
fn a_failed_listening_delete_leaves_the_data_untouched() {
    let mut core = signed_in();
    listen(&mut core, 2_000);
    core.dispatch(Command::DeleteListeningData);
    let update = core.dispatch(Command::AccountRequestFailed {
        unauthorized: false,
    });
    assert!(update.effects.is_empty());
    assert_eq!(update.snapshot.listening.device_total_ms, 2_000);
    assert_eq!(
        update.snapshot.account.status_label.as_deref(),
        Some("Couldn't delete listening data. Try again.")
    );
    assert!(!update.snapshot.account.busy);
}

#[test]
fn a_deleted_account_rotates_the_slot_and_signs_out() {
    let mut core = signed_in();
    listen(&mut core, 2_000);
    sync(&mut core, 1_000_000);
    core.dispatch(Command::DeleteAccount);
    let update = core.dispatch(Command::AccountDeleted {
        new_device_id: DEVICE_B.into(),
    });

    let writes = persisted_listening(&update.effects);
    assert_eq!(writes.len(), 1, "{:?}", update.effects);
    assert!(
        writes[0].contains(r#""deviceId":"device-b""#),
        "{}",
        writes[0]
    );
    assert!(writes[0].contains(r#""deviceTotalMs":0"#), "{}", writes[0]);
    assert_eq!(persisted_accounts(&update.effects), vec![String::new()]);
    assert!(
        !update
            .effects
            .iter()
            .any(|e| matches!(e, Effect::RevokeSession { .. })),
        "the account is already gone server-side"
    );
    assert_eq!(update.snapshot.listening.displayed_total_ms, 0);
    assert_eq!(update.snapshot.account.email, None);
    assert_eq!(
        update.snapshot.account.status_label.as_deref(),
        Some("Account deleted.")
    );
    assert!(begin(&mut core, SyncReason::Refresh).is_empty());
}

#[test]
fn a_deleted_account_drops_a_sync_ack_still_in_flight() {
    let mut core = signed_in();
    listen(&mut core, 2_000);
    assert!(pushed(&begin(&mut core, SyncReason::Refresh)).is_some());
    core.dispatch(Command::DeleteAccount);
    core.dispatch(Command::AccountDeleted {
        new_device_id: DEVICE_B.into(),
    });
    let snap = core
        .dispatch(Command::ListeningSyncSucceeded {
            server_total_ms: 1_000_000,
        })
        .snapshot;
    assert_eq!(snap.listening.displayed_total_ms, 0);
}

#[test]
fn a_failed_account_delete_stays_signed_in() {
    let mut core = signed_in();
    core.dispatch(Command::DeleteAccount);
    let update = core.dispatch(Command::AccountRequestFailed {
        unauthorized: false,
    });
    assert!(update.effects.is_empty());
    assert_eq!(update.snapshot.account.email, Some(EMAIL.into()));
    assert_eq!(
        update.snapshot.account.status_label.as_deref(),
        Some("Couldn't delete the account. Try again.")
    );
}

// ---- 401 -------------------------------------------------------------------

/// Assert `effects` and the core show a 401 sign-out.
fn assert_signed_out_by_401(core: &mut Core, effects: &[Effect]) {
    assert_eq!(persisted_accounts(effects), vec![String::new()]);
    assert_eq!(persisted_listening(effects).len(), 1, "{effects:?}");
    assert!(!effects
        .iter()
        .any(|e| matches!(e, Effect::RevokeSession { .. })));
    let snap = core.snapshot();
    assert_eq!(snap.account.email, None);
    assert!(!snap.account.busy);
    assert_eq!(
        snap.account.status_label.as_deref(),
        Some("Signed out — sign in again to sync.")
    );
    assert_eq!(snap.listening.displayed_total_ms, 2_000);
    assert!(begin(core, SyncReason::Refresh).is_empty());
}

#[test]
fn a_401_from_a_listening_sync_signs_out() {
    let mut core = signed_in();
    listen(&mut core, 2_000);
    sync(&mut core, 1_000_000);
    assert!(pushed(&begin(&mut core, SyncReason::Refresh)).is_some());
    let effects = core
        .dispatch(Command::ListeningSyncFailed { unauthorized: true })
        .effects;
    assert_signed_out_by_401(&mut core, &effects);
}

#[test]
fn a_401_from_any_account_request_signs_out() {
    let starts = [
        Command::RequestSignInLink {
            email: EMAIL.into(),
        },
        Command::SubmitSignInLink { input: "t".into() },
        Command::DeleteListeningData,
        Command::DeleteAccount,
    ];
    for start in starts {
        let mut core = signed_in();
        listen(&mut core, 2_000);
        sync(&mut core, 1_000_000);
        core.dispatch(start);
        let effects = core
            .dispatch(Command::AccountRequestFailed { unauthorized: true })
            .effects;
        assert_signed_out_by_401(&mut core, &effects);
    }
}

#[test]
fn a_401_while_signed_out_is_an_ordinary_failure() {
    let mut core = signed_out();
    core.dispatch(Command::SubmitSignInLink {
        input: "raw-token".into(),
    });
    let update = core.dispatch(Command::AccountRequestFailed { unauthorized: true });
    assert!(update.effects.is_empty());
    assert_eq!(
        update.snapshot.account.status_label.as_deref(),
        Some("That sign-in link was invalid or expired.")
    );
}

// ---- wire shape ------------------------------------------------------------
//
// Every shell hand-writes these JSON shapes. Lock them.

#[test]
fn account_commands_accept_the_shells_camel_case_json() {
    let cases = [
        (
            r#"{"type":"requestSignInLink","email":"e"}"#,
            Command::RequestSignInLink { email: "e".into() },
        ),
        (
            r#"{"type":"submitSignInLink","input":"i"}"#,
            Command::SubmitSignInLink { input: "i".into() },
        ),
        (r#"{"type":"signOut"}"#, Command::SignOut),
        (
            r#"{"type":"deleteListeningData"}"#,
            Command::DeleteListeningData,
        ),
        (r#"{"type":"deleteAccount"}"#, Command::DeleteAccount),
        (r#"{"type":"signInLinkSent"}"#, Command::SignInLinkSent),
        (
            r#"{"type":"signInVerified","sessionToken":"t","email":"e"}"#,
            Command::SignInVerified {
                session_token: "t".into(),
                email: "e".into(),
            },
        ),
        (
            r#"{"type":"listeningDataDeleted","newDeviceId":"d"}"#,
            Command::ListeningDataDeleted {
                new_device_id: "d".into(),
            },
        ),
        (
            r#"{"type":"accountDeleted","newDeviceId":"d"}"#,
            Command::AccountDeleted {
                new_device_id: "d".into(),
            },
        ),
        (
            r#"{"type":"accountRequestFailed","unauthorized":true}"#,
            Command::AccountRequestFailed { unauthorized: true },
        ),
    ];
    for (json, expected) in cases {
        let parsed: Command = serde_json::from_str(json).expect(json);
        assert_eq!(parsed, expected, "{json}");
    }
}

#[test]
fn account_effects_serialize_camel_case() {
    let cases = [
        (
            Effect::SendSignInLink { email: "e".into() },
            r#"{"type":"sendSignInLink","email":"e"}"#,
        ),
        (
            Effect::VerifySignInToken { token: "t".into() },
            r#"{"type":"verifySignInToken","token":"t"}"#,
        ),
        (
            Effect::RevokeSession {
                session_token: "s".into(),
            },
            r#"{"type":"revokeSession","sessionToken":"s"}"#,
        ),
        (
            Effect::DeleteServerListening {
                session_token: "s".into(),
            },
            r#"{"type":"deleteServerListening","sessionToken":"s"}"#,
        ),
        (
            Effect::DeleteServerAccount {
                session_token: "s".into(),
            },
            r#"{"type":"deleteServerAccount","sessionToken":"s"}"#,
        ),
        (
            Effect::PersistAccount { json: "j".into() },
            r#"{"type":"persistAccount","json":"j"}"#,
        ),
    ];
    for (effect, json) in cases {
        assert_eq!(serde_json::to_string(&effect).unwrap(), json);
    }
}

#[test]
fn the_account_snapshot_serializes_camel_case() {
    let json = serde_json::to_string(&signed_in().snapshot()).unwrap();
    assert!(
        json.contains(
            r#""account":{"email":"a@example.com","signedInLabel":"Syncing · a@example.com","statusLabel":null,"busy":false}"#
        ),
        "{json}"
    );
}
