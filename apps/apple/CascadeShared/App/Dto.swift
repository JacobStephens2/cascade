import Foundation

/// Mirrors the serde-tagged enums in `cascade-core`. The Rust crate is the
/// source of truth — these types describe the JSON wire shape so we can talk
/// to the bridge in a typed way.

enum Command: Codable {
    case play
    case pause
    case togglePlayback
    case setVolume(percent: Int)
    case toggleMute
    case startSleepTimer(minutes: Int)
    case startPomodoro(minutes: Int)
    case startStopwatch
    case cancelTimer
    case tick(elapsedMs: UInt64)
    case platformPlaybackStarted
    case platformPlaybackPaused
    case platformPlaybackError(message: String)

    private enum CodingKeys: String, CodingKey { case type, percent, minutes, elapsedMs, message }

    func encode(to encoder: Encoder) throws {
        var c = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case .play: try c.encode("play", forKey: .type)
        case .pause: try c.encode("pause", forKey: .type)
        case .togglePlayback: try c.encode("togglePlayback", forKey: .type)
        case .setVolume(let percent):
            try c.encode("setVolume", forKey: .type)
            try c.encode(percent, forKey: .percent)
        case .toggleMute: try c.encode("toggleMute", forKey: .type)
        case .startSleepTimer(let minutes):
            try c.encode("startSleepTimer", forKey: .type)
            try c.encode(minutes, forKey: .minutes)
        case .startPomodoro(let minutes):
            try c.encode("startPomodoro", forKey: .type)
            try c.encode(minutes, forKey: .minutes)
        case .startStopwatch: try c.encode("startStopwatch", forKey: .type)
        case .cancelTimer: try c.encode("cancelTimer", forKey: .type)
        case .tick(let elapsedMs):
            try c.encode("tick", forKey: .type)
            try c.encode(elapsedMs, forKey: .elapsedMs)
        case .platformPlaybackStarted: try c.encode("platformPlaybackStarted", forKey: .type)
        case .platformPlaybackPaused: try c.encode("platformPlaybackPaused", forKey: .type)
        case .platformPlaybackError(let message):
            try c.encode("platformPlaybackError", forKey: .type)
            try c.encode(message, forKey: .message)
        }
    }

    init(from decoder: Decoder) throws {
        // Round-trip support: we mostly only encode, but decode is handy in tests.
        let c = try decoder.container(keyedBy: CodingKeys.self)
        let type = try c.decode(String.self, forKey: .type)
        switch type {
        case "play": self = .play
        case "pause": self = .pause
        case "togglePlayback": self = .togglePlayback
        case "setVolume": self = .setVolume(percent: try c.decode(Int.self, forKey: .percent))
        case "toggleMute": self = .toggleMute
        case "startSleepTimer": self = .startSleepTimer(minutes: try c.decode(Int.self, forKey: .minutes))
        case "startPomodoro": self = .startPomodoro(minutes: try c.decode(Int.self, forKey: .minutes))
        case "startStopwatch": self = .startStopwatch
        case "cancelTimer": self = .cancelTimer
        case "tick": self = .tick(elapsedMs: try c.decode(UInt64.self, forKey: .elapsedMs))
        case "platformPlaybackStarted": self = .platformPlaybackStarted
        case "platformPlaybackPaused": self = .platformPlaybackPaused
        case "platformPlaybackError": self = .platformPlaybackError(message: try c.decode(String.self, forKey: .message))
        default:
            throw DecodingError.dataCorruptedError(forKey: .type, in: c, debugDescription: "unknown command type: \(type)")
        }
    }
}

enum Effect: Decodable {
    case startPlayback(volumePercent: Int)
    case pausePlayback
    case setPlatformVolume(volumePercent: Int)
    case persistSettings(json: String)

    private enum CodingKeys: String, CodingKey { case type, volumePercent, json }

    init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        let type = try c.decode(String.self, forKey: .type)
        switch type {
        case "startPlayback":
            self = .startPlayback(volumePercent: try c.decode(Int.self, forKey: .volumePercent))
        case "pausePlayback":
            self = .pausePlayback
        case "setPlatformVolume":
            self = .setPlatformVolume(volumePercent: try c.decode(Int.self, forKey: .volumePercent))
        case "persistSettings":
            self = .persistSettings(json: try c.decode(String.self, forKey: .json))
        default:
            throw DecodingError.dataCorruptedError(forKey: .type, in: c, debugDescription: "unknown effect type: \(type)")
        }
    }
}

enum TimerKind: String, Codable {
    case off, sleep, pomodoro, stopwatch, justCompleted
}

struct TimerSnapshot: Decodable, Equatable {
    let kind: TimerKind
    let remainingLabel: String
    let remainingMs: UInt64
    let totalMs: UInt64
    let progress: Float
}

struct Snapshot: Decodable, Equatable {
    let title: String
    let subtitle: String
    let isPlaying: Bool
    let volumePercent: Int
    let isMuted: Bool
    let primaryButtonLabel: String
    let timer: TimerSnapshot
    let errorMessage: String?
}

struct Update: Decodable {
    let snapshot: Snapshot
    let effects: [Effect]
}
