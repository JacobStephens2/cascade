import Foundation
#if os(macOS)
import IOKit.pwr_mgt
#elseif os(iOS)
import UIKit
#endif

/// Keeps the device from sleeping during a focus / sleep-timer session.
///
/// Two implementations behind one tiny API:
///
/// - **macOS:** `IOPMAssertionCreateWithName(kIOPMAssertPreventUserIdleSystemSleep, …)`.
///   The display can still dim or sleep, but the kernel won't pull the rug
///   out from under our audio engine.
/// - **iOS:** `UIApplication.shared.isIdleTimerDisabled = true`. Together
///   with the `UIBackgroundModes: audio` Info.plist key, this is enough for
///   an 8-hour session to survive the screen locking.
///
/// Acquired on `Effect.startPlayback`; released on `Effect.pausePlayback`.
@MainActor
final class PowerAssertion {
    private var held = false
    #if os(macOS)
    private var assertionID: IOPMAssertionID = 0
    #endif

    func acquire(reason: String = "Cascade focus session in progress") {
        guard !held else { return }
        #if os(macOS)
        let result = IOPMAssertionCreateWithName(
            kIOPMAssertPreventUserIdleSystemSleep as CFString,
            IOPMAssertionLevel(kIOPMAssertionLevelOn),
            reason as CFString,
            &assertionID
        )
        if result == kIOReturnSuccess { held = true }
        #elseif os(iOS)
        UIApplication.shared.isIdleTimerDisabled = true
        held = true
        #endif
    }

    func release() {
        guard held else { return }
        #if os(macOS)
        IOPMAssertionRelease(assertionID)
        assertionID = 0
        #elseif os(iOS)
        UIApplication.shared.isIdleTimerDisabled = false
        #endif
        held = false
    }

    deinit {
        guard held else { return }
        #if os(macOS)
        IOPMAssertionRelease(assertionID)
        #endif
        // iOS: the system clears isIdleTimerDisabled when the app terminates;
        // touching UIApplication from `deinit` (non-isolated) isn't safe.
    }
}
