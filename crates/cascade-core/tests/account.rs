//! The account flow, driven only through `Core::dispatch` — no network.
//!
//! The core holds the session and every account rule, including the request
//! table; the shell carries each `ServerRequest` and settles it with a
//! `ServerResponse`. These tests pin the core half: what is sent, when a
//! request may start, what the user is told, and what success, failure and a
//! 401 do to the account and to listening.

mod support;

use cascade_core::{AccountSnapshot, Command, Core, Effect, HttpMethod, SyncReason};
use serde_json::json;
use support::{bare_response, pushes, request_id, requests, response};

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

/// Dispatch `command` and return the id of the one request it sent.
fn start(core: &mut Core, command: Command) -> u64 {
    request_id(&core.dispatch(command).effects)
}

/// Start a sync and have the server report `server_total_ms`.
fn sync(core: &mut Core, server_total_ms: u64) {
    let id = request_id(&begin(core, SyncReason::Refresh));
    core.dispatch(pushed_ok(id, server_total_ms));
}

fn begin(core: &mut Core, reason: SyncReason) -> Vec<Effect> {
    core.dispatch(Command::BeginListeningSync { reason })
        .effects
}

/// The listening push in `effects`, if any.
fn pushed(effects: &[Effect]) -> Option<Effect> {
    pushes(effects).first().map(|e| (*e).clone())
}

/// The push request the core should send, per the request table.
fn push_request(id: u64, device_id: &str, device_total_ms: u64, token: &str) -> Effect {
    Effect::ServerRequest {
        id,
        method: HttpMethod::Put,
        path: "/listening".into(),
        bearer_token: Some(token.into()),
        body: Some(format!(
            r#"{{"deviceId":"{device_id}","deviceTotalMs":{device_total_ms}}}"#
        )),
    }
}

fn pushed_ok(id: u64, server_total_ms: u64) -> Command {
    response(id, 200, json!({ "serverTotalMs": server_total_ms }))
}

fn verified_ok(id: u64, session_token: &str, email: &str) -> Command {
    response(
        id,
        200,
        json!({ "sessionToken": session_token, "email": email }),
    )
}

fn request_link(email: &str) -> Command {
    Command::RequestSignInLink {
        email: email.into(),
        platform: None,
    }
}

fn submit(input: &str) -> Command {
    Command::SubmitSignInLink {
        input: input.into(),
    }
}

fn delete_listening() -> Command {
    Command::DeleteListeningData {
        new_device_id: DEVICE_B.into(),
    }
}

fn delete_account() -> Command {
    Command::DeleteAccount {
        new_device_id: DEVICE_B.into(),
    }
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
        request_link("b@example.com"),
        submit("raw-token"),
        delete_listening(),
        delete_account(),
    ]
}

/// A response of every kind the shell can send, for request `id`.
fn every_response(id: u64) -> Vec<Command> {
    vec![
        bare_response(id, 204),
        verified_ok(id, "late", "late@example.com"),
        pushed_ok(id, 9_000_000),
        bare_response(id, 0),
        bare_response(id, 500),
        bare_response(id, 401),
    ]
}

// ---- request table ---------------------------------------------------------

#[test]
fn requests_are_numbered_from_one_and_never_reused() {
    let mut core = signed_in();
    assert_eq!(start(&mut core, delete_listening()), 1);
    core.dispatch(bare_response(1, 500));
    assert_eq!(request_id(&begin(&mut core, SyncReason::Refresh)), 2);
    assert_eq!(start(&mut core, submit("raw-token")), 3);
    assert_eq!(request_id(&core.dispatch(Command::SignOut).effects), 4);
}

#[test]
fn each_request_matches_the_servers_table() {
    let cases = [
        (
            signed_out(),
            request_link(EMAIL),
            Effect::ServerRequest {
                id: 1,
                method: HttpMethod::Post,
                path: "/auth/request".into(),
                bearer_token: None,
                body: Some(r#"{"email":"a@example.com"}"#.into()),
            },
        ),
        (
            signed_out(),
            Command::RequestSignInLink {
                email: EMAIL.into(),
                platform: Some("windows".into()),
            },
            Effect::ServerRequest {
                id: 1,
                method: HttpMethod::Post,
                path: "/auth/request".into(),
                bearer_token: None,
                body: Some(r#"{"email":"a@example.com","platform":"windows"}"#.into()),
            },
        ),
        (
            signed_out(),
            submit("raw-token"),
            Effect::ServerRequest {
                id: 1,
                method: HttpMethod::Post,
                path: "/auth/verify".into(),
                bearer_token: None,
                body: Some(r#"{"token":"raw-token"}"#.into()),
            },
        ),
        (
            signed_in(),
            Command::SignOut,
            Effect::ServerRequest {
                id: 1,
                method: HttpMethod::Post,
                path: "/auth/logout".into(),
                bearer_token: Some(TOKEN.into()),
                body: None,
            },
        ),
        (
            signed_in(),
            Command::BeginListeningSync {
                reason: SyncReason::Refresh,
            },
            push_request(1, DEVICE_A, 0, TOKEN),
        ),
        (
            signed_in(),
            delete_listening(),
            Effect::ServerRequest {
                id: 1,
                method: HttpMethod::Delete,
                path: "/listening".into(),
                bearer_token: Some(TOKEN.into()),
                body: None,
            },
        ),
        (
            signed_in(),
            delete_account(),
            Effect::ServerRequest {
                id: 1,
                method: HttpMethod::Delete,
                path: "/account".into(),
                bearer_token: Some(TOKEN.into()),
                body: None,
            },
        ),
    ];
    for (mut core, command, expected) in cases {
        let effects = core.dispatch(command.clone()).effects;
        assert_eq!(requests(&effects), vec![&expected], "{command:?}");
    }
}

#[test]
fn a_platform_is_escaped_into_the_link_request_body() {
    let mut core = signed_out();
    let effects = core
        .dispatch(Command::RequestSignInLink {
            email: r#"a"b@example.com"#.into(),
            platform: Some("win\"dows".into()),
        })
        .effects;
    let Some(Effect::ServerRequest {
        body: Some(body), ..
    }) = requests(&effects).pop()
    else {
        panic!("{effects:?}");
    };
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(body).unwrap(),
        json!({ "email": r#"a"b@example.com"#, "platform": "win\"dows" })
    );
}

// ---- sign-in link ----------------------------------------------------------

#[test]
fn requesting_a_link_sends_the_trimmed_email_and_goes_busy() {
    let mut core = signed_out();
    let update = core.dispatch(request_link("  a@example.com\n"));
    assert_eq!(
        update.effects,
        vec![Effect::ServerRequest {
            id: 1,
            method: HttpMethod::Post,
            path: "/auth/request".into(),
            bearer_token: None,
            body: Some(r#"{"email":"a@example.com"}"#.into()),
        }]
    );
    assert!(update.snapshot.account.busy);
    assert_eq!(update.snapshot.account.status_label, None);
}

#[test]
fn an_empty_or_blank_email_is_refused() {
    for email in ["", "   ", "\n\t"] {
        let mut core = signed_out();
        let update = core.dispatch(request_link(email));
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
    for success in [200, 202, 204, 299] {
        let mut core = signed_out();
        let id = start(&mut core, request_link(EMAIL));
        let update = core.dispatch(bare_response(id, success));
        assert!(update.effects.is_empty(), "{success}");
        assert!(!update.snapshot.account.busy, "{success}");
        assert_eq!(
            update.snapshot.account.status_label.as_deref(),
            Some("Check a@example.com for a sign-in link."),
            "{success}"
        );
    }
}

#[test]
fn a_failed_link_request_says_try_again() {
    for failure in [0, 199, 300, 400, 403, 500] {
        let mut core = signed_out();
        let id = start(&mut core, request_link(EMAIL));
        core.dispatch(bare_response(id, failure));
        assert_eq!(
            status(&core).as_deref(),
            Some("Couldn't send the sign-in link. Try again."),
            "{failure}"
        );
        assert!(!core.snapshot().account.busy, "{failure}");
    }
}

// ---- token parser ----------------------------------------------------------

fn submitted_token(input: &str) -> Option<String> {
    let mut core = signed_out();
    let effects = core.dispatch(submit(input)).effects;
    effects.into_iter().find_map(|e| match e {
        Effect::ServerRequest {
            body: Some(body), ..
        } => {
            let body: serde_json::Value = serde_json::from_str(&body).unwrap();
            body["token"].as_str().map(String::from)
        }
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
    let update = core.dispatch(submit("https://cascade.example/"));
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
    let update = core.dispatch(submit("raw-token"));
    assert_eq!(requests(&update.effects).len(), 1);
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
    let id = start(&mut core, submit("raw-token"));
    let update = core.dispatch(verified_ok(id, TOKEN, EMAIL));
    assert_eq!(
        update.effects,
        vec![
            Effect::PersistAccount {
                json: blob(TOKEN, EMAIL)
            },
            push_request(id + 1, DEVICE_A, 3_000, TOKEN),
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
fn a_verify_response_ignores_unknown_fields() {
    let mut core = signed_out();
    let id = start(&mut core, submit("raw-token"));
    let snap = core
        .dispatch(response(
            id,
            200,
            json!({ "sessionToken": TOKEN, "email": EMAIL, "expiresAt": 1 }),
        ))
        .snapshot;
    assert_eq!(snap.account.email, Some(EMAIL.into()));
}

#[test]
fn a_verified_sign_in_skips_the_refresh_while_a_sync_is_in_flight() {
    // Signed in, a sync goes out, then the user signs in again from a link.
    let mut core = signed_in();
    assert!(pushed(&begin(&mut core, SyncReason::Refresh)).is_some());
    let id = start(&mut core, submit("raw-token"));
    let effects = core
        .dispatch(verified_ok(id, "session-2", "b@example.com"))
        .effects;
    assert!(pushed(&effects).is_none(), "{effects:?}");
    assert_eq!(
        persisted_accounts(&effects),
        vec![blob("session-2", "b@example.com")]
    );
}

#[test]
fn an_invalid_link_says_so() {
    for failure in [0, 400, 410, 500] {
        let mut core = signed_out();
        let id = start(&mut core, submit("raw-token"));
        let update = core.dispatch(bare_response(id, failure));
        assert!(update.effects.is_empty(), "{failure}");
        assert_eq!(
            update.snapshot.account.status_label.as_deref(),
            Some("That sign-in link was invalid or expired."),
            "{failure}"
        );
        assert_eq!(update.snapshot.account.email, None, "{failure}");
    }
}

#[test]
fn a_verify_success_that_does_not_parse_is_an_invalid_link() {
    let bodies = [
        String::new(),
        "not json".into(),
        json!({ "email": EMAIL }).to_string(),
        json!({ "sessionToken": TOKEN }).to_string(),
        json!({ "sessionToken": "", "email": EMAIL }).to_string(),
        json!({ "sessionToken": 7, "email": EMAIL }).to_string(),
    ];
    for body in bodies {
        let mut core = signed_out();
        let id = start(&mut core, submit("raw-token"));
        let update = core.dispatch(Command::ServerResponse {
            id,
            status: 200,
            body: body.clone(),
        });
        assert!(update.effects.is_empty(), "{body:?}");
        assert!(!update.snapshot.account.busy, "{body:?}");
        assert_eq!(update.snapshot.account.email, None, "{body:?}");
        assert_eq!(
            update.snapshot.account.status_label.as_deref(),
            Some("That sign-in link was invalid or expired."),
            "{body:?}"
        );
    }
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
    for reason in [SyncReason::Flush, SyncReason::Refresh] {
        assert!(begin(&mut core, reason).is_empty(), "{reason:?}");
    }
}

#[test]
fn a_push_carries_the_session_token() {
    let mut core = signed_in();
    assert_eq!(
        pushed(&begin(&mut core, SyncReason::Refresh)),
        Some(push_request(1, DEVICE_A, 0, TOKEN))
    );
}

#[test]
fn an_account_request_and_a_sync_settle_in_either_order() {
    for sync_first in [true, false] {
        let mut core = signed_in();
        listen(&mut core, 2_000);
        let sync_id = request_id(&begin(&mut core, SyncReason::Refresh));
        let delete_id = start(&mut core, delete_listening());
        assert_ne!(sync_id, delete_id);

        let settle_sync = pushed_ok(sync_id, 1_000_000);
        let settle_delete = bare_response(delete_id, 500);
        let (first, second) = if sync_first {
            (settle_sync, settle_delete)
        } else {
            (settle_delete, settle_sync)
        };
        core.dispatch(first);
        core.dispatch(second);

        let snap = core.snapshot();
        assert_eq!(snap.listening.unsynced_ms, 0, "sync first: {sync_first}");
        assert_eq!(
            snap.listening.displayed_total_ms, 1_000_000,
            "sync first: {sync_first}"
        );
        assert!(!snap.account.busy, "sync first: {sync_first}");
        assert_eq!(
            snap.account.status_label.as_deref(),
            Some("Couldn't delete listening data. Try again."),
            "sync first: {sync_first}"
        );
    }
}

#[test]
fn a_sync_response_does_not_settle_the_account_request() {
    let mut core = signed_in();
    let sync_id = request_id(&begin(&mut core, SyncReason::Refresh));
    start(&mut core, delete_account());
    core.dispatch(bare_response(sync_id, 204));
    let snap = core.snapshot();
    assert!(snap.account.busy);
    assert_eq!(snap.account.email, Some(EMAIL.into()));
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
        Some(&Effect::ServerRequest {
            id: 2,
            method: HttpMethod::Post,
            path: "/auth/logout".into(),
            bearer_token: Some(TOKEN.into()),
            body: None,
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
fn the_revokes_response_changes_nothing() {
    for revoke_answer in every_response(2) {
        let mut core = signed_in();
        listen(&mut core, 2_000);
        sync(&mut core, 1_000_000);
        let revoke_id = request_id(&core.dispatch(Command::SignOut).effects);
        assert_eq!(revoke_id, 2);
        let before = core.snapshot();
        let update = core.dispatch(revoke_answer.clone());
        assert!(update.effects.is_empty(), "{revoke_answer:?}");
        assert_eq!(update.snapshot, before, "{revoke_answer:?}");
    }
}

#[test]
fn a_sync_ack_that_lands_after_sign_out_is_dropped() {
    let mut core = signed_in();
    listen(&mut core, 2_000);
    let id = request_id(&begin(&mut core, SyncReason::Refresh));
    core.dispatch(Command::SignOut);

    let snap = core.dispatch(pushed_ok(id, 1_000_000)).snapshot;
    assert_eq!(snap.listening.displayed_total_ms, 2_000);
}

#[test]
fn a_second_account_shows_none_of_the_first_accounts_total() {
    let mut core = signed_in();
    listen(&mut core, 2_000);
    sync(&mut core, 1_000_000);
    core.dispatch(Command::SignOut);

    let id = start(&mut core, submit("raw-token"));
    let snap = core
        .dispatch(verified_ok(id, "session-2", "b@example.com"))
        .snapshot;
    assert_eq!(snap.listening.displayed_total_ms, 2_000);
}

#[test]
fn signing_in_over_an_account_forgets_its_total() {
    let mut core = signed_in();
    listen(&mut core, 2_000);
    sync(&mut core, 1_000_000);

    let id = start(&mut core, submit("raw-token"));
    let snap = core
        .dispatch(verified_ok(id, "session-2", "b@example.com"))
        .snapshot;
    assert_eq!(snap.listening.displayed_total_ms, 2_000);
}

#[test]
fn sign_out_while_signed_out_emits_nothing_but_clears_the_status() {
    let mut core = signed_out();
    core.dispatch(request_link(""));
    let update = core.dispatch(Command::SignOut);
    assert!(update.effects.is_empty());
    assert_eq!(update.snapshot.account.status_label, None);
}

// ---- busy and re-entrancy --------------------------------------------------

#[test]
fn every_account_request_is_ignored_while_one_is_pending() {
    let pending_starts = [
        (signed_out(), request_link(EMAIL)),
        (signed_out(), submit("t")),
        (signed_in(), delete_listening()),
        (signed_in(), delete_account()),
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
fn a_response_with_an_unknown_id_changes_nothing() {
    // Nothing pending, nothing in flight: no id is known.
    for unknown in [0, 1, 2, 3, u64::MAX] {
        for answer in every_response(unknown) {
            let mut core = signed_in();
            listen(&mut core, 2_000);
            sync(&mut core, 1_000_000);
            let before = core.snapshot();
            let update = core.dispatch(answer.clone());
            assert!(update.effects.is_empty(), "{answer:?}");
            assert_eq!(update.snapshot, before, "{answer:?}");
        }
    }
}

#[test]
fn a_response_for_another_id_leaves_the_pending_request_alone() {
    let mut core = signed_in();
    let id = start(&mut core, delete_listening());
    let before = core.snapshot();
    for other in [id - 1, id + 1] {
        for answer in every_response(other) {
            let update = core.dispatch(answer.clone());
            assert!(update.effects.is_empty(), "{answer:?}");
            assert_eq!(update.snapshot, before, "{answer:?}");
        }
    }
}

#[test]
fn a_request_is_settled_only_once() {
    let mut core = signed_in();
    let id = start(&mut core, delete_listening());
    core.dispatch(bare_response(id, 500));
    let before = core.snapshot();
    let update = core.dispatch(bare_response(id, 204));
    assert!(update.effects.is_empty());
    assert_eq!(update.snapshot, before);
}

#[test]
fn sign_out_cuts_in_and_drops_the_pending_request() {
    let mut core = signed_in();
    let id = start(&mut core, delete_account());
    let update = core.dispatch(Command::SignOut);
    assert!(!update.snapshot.account.busy);
    assert_eq!(requests(&update.effects).len(), 1, "the revoke");

    // The dropped request's late response is ignored.
    let before = core.snapshot();
    let late = core.dispatch(bare_response(id, 204));
    assert!(late.effects.is_empty());
    assert_eq!(late.snapshot, before);
}

#[test]
fn sign_out_drops_a_pending_sign_in() {
    let mut core = signed_out();
    let id = start(&mut core, submit("raw-token"));
    core.dispatch(Command::SignOut);
    let update = core.dispatch(verified_ok(id, TOKEN, EMAIL));
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
fn deletes_go_busy() {
    for request in [delete_listening(), delete_account()] {
        let mut core = signed_in();
        let update = core.dispatch(request.clone());
        assert_eq!(requests(&update.effects).len(), 1, "{request:?}");
        assert!(update.snapshot.account.busy, "{request:?}");
    }
}

#[test]
fn deletes_do_nothing_while_signed_out() {
    for request in [delete_listening(), delete_account()] {
        let mut core = signed_out();
        let update = core.dispatch(request.clone());
        assert!(update.effects.is_empty(), "{request:?}");
        assert!(!update.snapshot.account.busy, "{request:?}");
    }
}

#[test]
fn a_delete_keeps_the_old_id_until_the_server_confirms() {
    let mut core = signed_in();
    listen(&mut core, 2_000);
    let update = core.dispatch(delete_listening());
    assert!(persisted_listening(&update.effects).is_empty());
    assert_eq!(update.snapshot.listening.device_total_ms, 2_000);
}

#[test]
fn deleted_listening_data_rotates_the_id_and_zeroes_the_slot_in_one_write() {
    let mut core = signed_in();
    listen(&mut core, 2_000);
    sync(&mut core, 1_000_000);
    let id = start(&mut core, delete_listening());
    let update = core.dispatch(bare_response(id, 204));

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
        Some(push_request(id + 1, DEVICE_B, 0, TOKEN))
    );
}

#[test]
fn a_failed_listening_delete_leaves_the_data_untouched() {
    for failure in [0, 500] {
        let mut core = signed_in();
        listen(&mut core, 2_000);
        let id = start(&mut core, delete_listening());
        let update = core.dispatch(bare_response(id, failure));
        assert!(update.effects.is_empty(), "{failure}");
        assert_eq!(update.snapshot.listening.device_total_ms, 2_000);
        assert_eq!(
            update.snapshot.account.status_label.as_deref(),
            Some("Couldn't delete listening data. Try again.")
        );
        assert!(!update.snapshot.account.busy);
        assert_eq!(
            pushed(&begin(&mut core, SyncReason::Refresh)),
            Some(push_request(id + 1, DEVICE_A, 2_000, TOKEN)),
            "the old slot stays"
        );
    }
}

#[test]
fn a_deleted_account_rotates_the_slot_and_signs_out() {
    let mut core = signed_in();
    listen(&mut core, 2_000);
    sync(&mut core, 1_000_000);
    let id = start(&mut core, delete_account());
    let update = core.dispatch(bare_response(id, 200));

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
        requests(&update.effects).is_empty(),
        "the account is already gone server-side: no revoke"
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
    let sync_id = request_id(&begin(&mut core, SyncReason::Refresh));
    let id = start(&mut core, delete_account());
    core.dispatch(bare_response(id, 204));
    let snap = core.dispatch(pushed_ok(sync_id, 1_000_000)).snapshot;
    assert_eq!(snap.listening.displayed_total_ms, 0);
}

#[test]
fn a_failed_account_delete_stays_signed_in() {
    let mut core = signed_in();
    let id = start(&mut core, delete_account());
    let update = core.dispatch(bare_response(id, 0));
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
    assert!(requests(effects).is_empty(), "no revoke: {effects:?}");
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
    let id = request_id(&begin(&mut core, SyncReason::Refresh));
    let effects = core.dispatch(bare_response(id, 401)).effects;
    assert_signed_out_by_401(&mut core, &effects);
}

#[test]
fn a_401_from_any_account_request_signs_out() {
    let starts = [
        request_link(EMAIL),
        submit("t"),
        delete_listening(),
        delete_account(),
    ];
    for start_command in starts {
        let mut core = signed_in();
        listen(&mut core, 2_000);
        sync(&mut core, 1_000_000);
        let id = start(&mut core, start_command);
        let effects = core.dispatch(bare_response(id, 401)).effects;
        assert_signed_out_by_401(&mut core, &effects);
    }
}

#[test]
fn a_401_while_signed_out_is_an_ordinary_failure() {
    let mut core = signed_out();
    let id = start(&mut core, submit("raw-token"));
    let update = core.dispatch(bare_response(id, 401));
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
            Command::RequestSignInLink {
                email: "e".into(),
                platform: None,
            },
        ),
        (
            r#"{"type":"requestSignInLink","email":"e","platform":"windows"}"#,
            Command::RequestSignInLink {
                email: "e".into(),
                platform: Some("windows".into()),
            },
        ),
        (
            r#"{"type":"submitSignInLink","input":"i"}"#,
            Command::SubmitSignInLink { input: "i".into() },
        ),
        (r#"{"type":"signOut"}"#, Command::SignOut),
        (
            r#"{"type":"deleteListeningData","newDeviceId":"d"}"#,
            Command::DeleteListeningData {
                new_device_id: "d".into(),
            },
        ),
        (
            r#"{"type":"deleteAccount","newDeviceId":"d"}"#,
            Command::DeleteAccount {
                new_device_id: "d".into(),
            },
        ),
    ];
    for (json, expected) in cases {
        let parsed: Command = serde_json::from_str(json).expect(json);
        assert_eq!(parsed, expected, "{json}");
    }
}

#[test]
fn the_removed_settle_commands_no_longer_parse() {
    for json in [
        r#"{"type":"signInLinkSent"}"#,
        r#"{"type":"signInVerified","sessionToken":"t","email":"e"}"#,
        r#"{"type":"listeningDataDeleted","newDeviceId":"d"}"#,
        r#"{"type":"accountDeleted","newDeviceId":"d"}"#,
        r#"{"type":"accountRequestFailed","unauthorized":true}"#,
        r#"{"type":"listeningSyncSucceeded","serverTotalMs":5}"#,
        r#"{"type":"listeningSyncFailed","unauthorized":true}"#,
        r#"{"type":"deleteAccount"}"#,
    ] {
        assert!(serde_json::from_str::<Command>(json).is_err(), "{json}");
    }
}

#[test]
fn server_response_accepts_the_shells_camel_case_json() {
    let cases = [
        (
            r#"{"type":"serverResponse","id":3,"status":200,"body":"{\"serverTotalMs\":5}"}"#,
            Command::ServerResponse {
                id: 3,
                status: 200,
                body: r#"{"serverTotalMs":5}"#.into(),
            },
        ),
        (
            r#"{"type":"serverResponse","id":4,"status":0,"body":""}"#,
            Command::ServerResponse {
                id: 4,
                status: 0,
                body: String::new(),
            },
        ),
        (
            r#"{"type":"serverResponse","id":4,"status":0}"#,
            Command::ServerResponse {
                id: 4,
                status: 0,
                body: String::new(),
            },
        ),
    ];
    for (json, expected) in cases {
        let parsed: Command = serde_json::from_str(json).expect(json);
        assert_eq!(parsed, expected, "{json}");
        assert_eq!(
            serde_json::from_str::<Command>(&serde_json::to_string(&parsed).unwrap()).unwrap(),
            expected
        );
    }
}

#[test]
fn server_request_serializes_camel_case() {
    let cases = [
        (
            Effect::ServerRequest {
                id: 7,
                method: HttpMethod::Put,
                path: "/listening".into(),
                bearer_token: Some("s".into()),
                body: Some(r#"{"deviceId":"d","deviceTotalMs":7}"#.into()),
            },
            r#"{"type":"serverRequest","id":7,"method":"PUT","path":"/listening","bearerToken":"s","body":"{\"deviceId\":\"d\",\"deviceTotalMs\":7}"}"#,
        ),
        (
            Effect::ServerRequest {
                id: 1,
                method: HttpMethod::Post,
                path: "/auth/verify".into(),
                bearer_token: None,
                body: Some(r#"{"token":"t"}"#.into()),
            },
            r#"{"type":"serverRequest","id":1,"method":"POST","path":"/auth/verify","body":"{\"token\":\"t\"}"}"#,
        ),
        (
            Effect::ServerRequest {
                id: 2,
                method: HttpMethod::Delete,
                path: "/account".into(),
                bearer_token: Some("s".into()),
                body: None,
            },
            r#"{"type":"serverRequest","id":2,"method":"DELETE","path":"/account","bearerToken":"s"}"#,
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
