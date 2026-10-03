//! Output gain, driven only through `Core::dispatch`.
//!
//! The core decides how loud the speaker plays: mute and the square-law curve
//! are applied here, and every shell writes the resulting gain straight into
//! its audio engine. These tests pin that rule on the effects and snapshot.

use cascade_core::{Command, Core, Effect, Update};

/// The gain on the `SetPlatformVolume` effect in `update`, if any.
fn platform_gain(update: &Update) -> Option<f32> {
    update.effects.iter().find_map(|e| match e {
        Effect::SetPlatformVolume { gain } => Some(*gain),
        _ => None,
    })
}

/// The gain on the `StartPlayback` effect in `update`, if any.
fn start_gain(update: &Update) -> Option<f32> {
    update.effects.iter().find_map(|e| match e {
        Effect::StartPlayback { gain } => Some(*gain),
        _ => None,
    })
}

#[test]
fn set_volume_applies_the_square_law_curve() {
    for (percent, gain) in [(0, 0.0), (50, 0.25), (100, 1.0)] {
        let mut core = Core::new();
        let update = core.dispatch(Command::SetVolume { percent });
        assert_eq!(platform_gain(&update), Some(gain), "{percent}%");
        assert_eq!(update.snapshot.output_gain, gain, "{percent}%");
    }
}

#[test]
fn start_playback_carries_the_curve_value() {
    let mut core = Core::new();
    core.dispatch(Command::SetVolume { percent: 50 });
    let update = core.dispatch(Command::Play);
    assert_eq!(start_gain(&update), Some(0.25));
    assert_eq!(update.snapshot.output_gain, 0.25);
}

#[test]
fn mute_gives_zero_and_unmute_restores_the_curve_value() {
    let mut core = Core::new();
    core.dispatch(Command::SetVolume { percent: 50 });
    core.dispatch(Command::Play);

    let muted = core.dispatch(Command::ToggleMute);
    assert_eq!(platform_gain(&muted), Some(0.0));
    assert_eq!(muted.snapshot.output_gain, 0.0);
    // The UI still shows the chosen level.
    assert_eq!(muted.snapshot.volume_percent, 50);
    assert!(muted.snapshot.is_muted);

    let unmuted = core.dispatch(Command::ToggleMute);
    assert_eq!(platform_gain(&unmuted), Some(0.25));
    assert_eq!(unmuted.snapshot.output_gain, 0.25);
}

#[test]
fn set_volume_while_muted_unmutes_with_the_curve_value() {
    let mut core = Core::new();
    core.dispatch(Command::Play);
    core.dispatch(Command::ToggleMute);

    let update = core.dispatch(Command::SetVolume { percent: 50 });
    assert!(!update.snapshot.is_muted);
    assert_eq!(platform_gain(&update), Some(0.25));
    assert_eq!(update.snapshot.output_gain, 0.25);
}

// Every shell reads `outputGain` off the JSON snapshot to recover the level
// after rebuilding its audio pipeline; a rename would silently break that.
#[test]
fn output_gain_serializes_camel_case() {
    let mut core = Core::new();
    core.dispatch(Command::SetVolume { percent: 50 });
    let json = serde_json::to_string(&core.snapshot()).unwrap();
    assert!(
        json.contains(r#""outputGain":0.25"#),
        "snapshot JSON: {json}"
    );
}
