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

/// Compact preset enum the watch sends. The iPhone side maps these onto the
/// core's `startPomodoro(minutes:)` command.
public enum WatchSessionPreset: String, Codable, Sendable {
    case minutes30
    case minutes60
    case hours8

    public var minutes: Int {
        switch self {
        case .minutes30: return 30
        case .minutes60: return 60
        case .hours8:    return 480
        }
    }
}

/// Commands the Watch can send to the iPhone.
public enum WatchToPhoneCommand: Codable, Sendable, Equatable {
    case requestSnapshot
    case togglePlayback
    case play
    case pause
    case setVolume(percent: Int)
    case startSession(WatchSessionPreset)
    /// User-entered duration. `sleep` picks the timer flavor (sleep timer vs
    /// focus session); the iPhone maps it to the matching core command.
    case startCustom(minutes: Int, sleep: Bool)
    case cancelTimer
}

/// Watch-flavored snapshot. Subset of the full Cascade `Snapshot` plus a
/// pre-formatted `statusLine` so the watch doesn't have to localize or
/// time-format anything.
public struct PhoneSnapshotForWatch: Codable, Sendable, Equatable {
    public let version: Int
    public let isPlaying: Bool
    public let volumePercent: Int
    /// "Playing · 42:17 left" / "Paused" / "Playing on iPhone" — already
    /// formatted for a wrist-sized label.
    public let statusLine: String
    /// 0.0–1.0 for the active session, or 0 when no timer is running.
    public let timerProgress: Float
    /// Empty when no timer is running.
    public let timerRemainingLabel: String

    public init(
        version: Int = watchProtocolVersion,
        isPlaying: Bool,
        volumePercent: Int,
        statusLine: String,
        timerProgress: Float,
        timerRemainingLabel: String
    ) {
        self.version = version
        self.isPlaying = isPlaying
        self.volumePercent = volumePercent
        self.statusLine = statusLine
        self.timerProgress = timerProgress
        self.timerRemainingLabel = timerRemainingLabel
    }

    /// Sensible default for the watch's cold-start render before the iPhone
    /// has replied.
    public static let placeholder = PhoneSnapshotForWatch(
        isPlaying: false,
        volumePercent: 60,
        statusLine: "Connecting…",
        timerProgress: 0,
        timerRemainingLabel: ""
    )
}
