import Foundation

/// Wire shapes for the WatchConnectivity link between the iPhone app and the
/// Apple Watch companion.
///
/// **Mode A — thin remote.** The Watch never owns Cascade truth: it sends
/// commands and renders whatever snapshot the iPhone returns. Settings, timer
/// math, the Rust core, and the audio engine all live on iPhone.
///
/// Versioning: every payload carries `version` so we can teach future shells
/// to gracefully ignore unknown shapes. Today's only version is `1`.

public let watchProtocolVersion: Int = 1

/// A timer preset as the watch shows it: the core's minutes and long label.
public struct WatchTimerPreset: Codable, Sendable, Equatable {
    public let minutes: Int
    public let label: String

    public init(minutes: Int, label: String) {
        self.minutes = minutes
        self.label = label
    }
}

/// Commands the Watch can send to the iPhone.
public enum WatchToPhoneCommand: Codable, Sendable, Equatable {
    case requestSnapshot
    case togglePlayback
    case play
    case pause
    case setVolume(percent: Int)
    /// Silence audio without pausing the session (timer keeps running).
    case toggleMute
    /// A preset or user-entered duration. `sleep` picks the timer flavor
    /// (sleep timer vs focus session); the iPhone maps it to the matching core
    /// command, and the core clamps `minutes`.
    case startCustom(minutes: Int, sleep: Bool)
    /// Start the count-up stopwatch.
    case startStopwatch
    case cancelTimer
}

/// Watch-flavored snapshot. Subset of the full Cascade `Snapshot` plus a
/// pre-formatted `statusLine` so the watch doesn't have to localize or
/// time-format anything.
///
/// Fields added after the first release decode with defaults, so a payload
/// from an older iPhone build still decodes.
public struct PhoneSnapshotForWatch: Codable, Sendable, Equatable {
    public let version: Int
    public let isPlaying: Bool
    public let volumePercent: Int
    /// Audio silenced while the session keeps running.
    public let isMuted: Bool
    /// "Playing · 42:17 left" / "Paused" / "Playing on iPhone" — already
    /// formatted for a wrist-sized label.
    public let statusLine: String
    /// 0.0–1.0 for the active session, or 0 when no timer is running.
    public let timerProgress: Float
    /// Empty when no timer is running.
    public let timerRemainingLabel: String
    /// A timer is running (something to cancel); false when off or just
    /// completed. Mirrors the core's `timer.isActive`.
    public let isTimerActive: Bool
    /// The core's focus timer presets.
    public let focusPresets: [WatchTimerPreset]
    /// Custom-duration limits, in minutes.
    public let minMinutes: Int
    public let maxMinutes: Int
    /// What the custom stepper opens on: the last-started focus length.
    public let customFocusMinutes: Int

    public init(
        version: Int = watchProtocolVersion,
        isPlaying: Bool,
        volumePercent: Int,
        isMuted: Bool = false,
        statusLine: String,
        timerProgress: Float,
        timerRemainingLabel: String,
        isTimerActive: Bool = false,
        focusPresets: [WatchTimerPreset] = [],
        minMinutes: Int = 1,
        maxMinutes: Int = 1440,
        customFocusMinutes: Int = 30
    ) {
        self.version = version
        self.isPlaying = isPlaying
        self.volumePercent = volumePercent
        self.isMuted = isMuted
        self.statusLine = statusLine
        self.timerProgress = timerProgress
        self.timerRemainingLabel = timerRemainingLabel
        self.isTimerActive = isTimerActive
        self.focusPresets = focusPresets
        self.minMinutes = minMinutes
        self.maxMinutes = maxMinutes
        self.customFocusMinutes = customFocusMinutes
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        let defaults = PhoneSnapshotForWatch.placeholder
        let remainingLabel = try c.decode(String.self, forKey: .timerRemainingLabel)
        self.init(
            version: try c.decode(Int.self, forKey: .version),
            isPlaying: try c.decode(Bool.self, forKey: .isPlaying),
            volumePercent: try c.decode(Int.self, forKey: .volumePercent),
            isMuted: try c.decode(Bool.self, forKey: .isMuted),
            statusLine: try c.decode(String.self, forKey: .statusLine),
            timerProgress: try c.decode(Float.self, forKey: .timerProgress),
            timerRemainingLabel: remainingLabel,
            // An older payload has no isTimerActive; a non-empty remaining
            // label is how the watch read "running" before.
            isTimerActive: try c.decodeIfPresent(Bool.self, forKey: .isTimerActive)
                ?? !remainingLabel.isEmpty,
            focusPresets: try c.decodeIfPresent([WatchTimerPreset].self, forKey: .focusPresets)
                ?? defaults.focusPresets,
            minMinutes: try c.decodeIfPresent(Int.self, forKey: .minMinutes) ?? defaults.minMinutes,
            maxMinutes: try c.decodeIfPresent(Int.self, forKey: .maxMinutes) ?? defaults.maxMinutes,
            customFocusMinutes: try c.decodeIfPresent(Int.self, forKey: .customFocusMinutes)
                ?? defaults.customFocusMinutes
        )
    }

    /// Sensible default for the watch's cold-start render before the iPhone
    /// has replied.
    public static let placeholder = PhoneSnapshotForWatch(
        isPlaying: false,
        volumePercent: 60,
        isMuted: false,
        statusLine: "Connecting…",
        timerProgress: 0,
        timerRemainingLabel: ""
    )
}
