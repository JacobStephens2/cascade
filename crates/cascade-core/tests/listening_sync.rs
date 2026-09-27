//! Listening-sync policy, driven only through `Core::dispatch` — no network.
//!
//! The shell owns *when* it can talk (reachability, lifecycle, auth); the core
//! owns *whether there is anything to say, and what*. These tests pin the core
//! half: the threshold, the payload, the synced high-water mark, the 401 rule,
//! and the device-id lifecycle.

use cascade_core::{Command, Core, Effect, SyncReason};

const DEVICE_A: &str = "device-a";
const DEVICE_B: &str = "device-b";

/// A core that has restored (nothing) with `DEVICE_A` as its fallback id and
/// has accrued `listened_ms` of confirmed audio.
fn core_with_listening(listened_ms: u64) -> Core {
    let mut core = Core::new();
    core.dispatch(Command::RestoreListening {
        json: String::new(),
        fallback_device_id: DEVICE_A.into(),
    });
    core.dispatch(Command::Play);
    core.dispatch(Command::PlatformPlaybackStarted);
    let mut left = listened_ms;
    while left > 0 {
        let step = left.min(1_000);
        core.dispatch(Command::Tick { elapsed_ms: step });
        left -= step;
    }
    core
}

/// The `PushListening` payload in `effects`, if any.
fn pushed(effects: &[Effect]) -> Option<(String, u64)> {
    effects.iter().find_map(|e| match e {
        Effect::PushListening {
            device_id,
            device_total_ms,
        } => Some((device_id.clone(), *device_total_ms)),
        _ => None,
    })
}

fn begin(core: &mut Core, reason: SyncReason) -> Option<(String, u64)> {
    pushed(&core.dispatch(Command::BeginListeningSync { reason }).effects)
}

#[test]
fn synced_through_is_exactly_what_was_sent() {
    let mut core = core_with_listening(40_000);
    let (device_id, sent) = begin(&mut core, SyncReason::Threshold).expect("40s is past threshold");
    assert_eq!(device_id, DEVICE_A);
    assert_eq!(sent, 40_000);

    // More listening lands while the PUT is in flight.
    core.dispatch(Command::Tick { elapsed_ms: 2_000 });

    let snap = core
        .dispatch(Command::ListeningSyncSucceeded {
            server_total_ms: 1_000_000,
        })
        .snapshot;
    // Only what was sent is acknowledged; the 2s accrued mid-flight stays
    // unsynced rather than being silently marked as delivered.
    assert_eq!(snap.listening.unsynced_ms, 2_000);
    assert_eq!(snap.listening.displayed_total_ms, 1_000_000 + 2_000);
}
