import Foundation
import IOKit.pwr_mgt

/// Prevents the Mac from going to sleep mid-session.
///
/// `PreventUserIdleSystemSleep` is the right level for a focus / 8-hour
/// session: the display can still dim or sleep, but the kernel won't pull the
/// rug out from under our audio engine. Released as soon as playback stops.
@MainActor
final class PowerAssertion {
    private var assertionID: IOPMAssertionID = 0
    private var held = false

    func acquire(reason: String = "Cascade focus session in progress") {
        guard !held else { return }
        let result = IOPMAssertionCreateWithName(
            kIOPMAssertPreventUserIdleSystemSleep as CFString,
            IOPMAssertionLevel(kIOPMAssertionLevelOn),
            reason as CFString,
            &assertionID
        )
        if result == kIOReturnSuccess { held = true }
    }

    func release() {
        guard held else { return }
        IOPMAssertionRelease(assertionID)
        assertionID = 0
        held = false
    }

    deinit {
        if held {
            IOPMAssertionRelease(assertionID)
        }
    }
}
