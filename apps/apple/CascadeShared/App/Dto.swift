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
    case setListeningTracking(enabled: Bool)
    /// `accountJson` is the stored account blob, verbatim; `""` for none.
    /// Always sent, so the core holds the account.
    case restore(settingsJson: String, listeningJson: String, fallbackDeviceId: String, accountJson: String)
    case beginListeningSync(reason: SyncReason)
    case resetListeningData(newDeviceId: String)
    /// `platform` names the platform whose app the emailed link hands off to;
    /// nil leaves it out of the request.
    case requestSignInLink(email: String, platform: String? = nil)
    /// The whole pasted or opened sign-in link, or the bare token.
    case submitSignInLink(input: String)
    case signOut
    /// `newDeviceId` is a fresh UUID made when the user asks; the core adopts
    /// it only once the server confirms the delete.
    case deleteListeningData(newDeviceId: String)
    case deleteAccount(newDeviceId: String)
    /// Settles the `serverRequest` with the same `id`: the HTTP status (0 when
    /// it was not sent or got no response) and the response body verbatim.
    case serverResponse(id: UInt64, status: UInt16, body: String)

    private enum CodingKeys: String, CodingKey {
        case type, percent, minutes, elapsedMs, message, enabled, settingsJson, listeningJson,
             fallbackDeviceId, accountJson, reason,
             newDeviceId, email, platform, input, id, status, body
    }

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
        case .setListeningTracking(let enabled):
            try c.encode("setListeningTracking", forKey: .type)
            try c.encode(enabled, forKey: .enabled)
        case .restore(let settingsJson, let listeningJson, let fallbackDeviceId, let accountJson):
            try c.encode("restore", forKey: .type)
            try c.encode(settingsJson, forKey: .settingsJson)
            try c.encode(listeningJson, forKey: .listeningJson)
            try c.encode(fallbackDeviceId, forKey: .fallbackDeviceId)
            try c.encode(accountJson, forKey: .accountJson)
        case .beginListeningSync(let reason):
            try c.encode("beginListeningSync", forKey: .type)
            try c.encode(reason, forKey: .reason)
        case .resetListeningData(let newDeviceId):
            try c.encode("resetListeningData", forKey: .type)
            try c.encode(newDeviceId, forKey: .newDeviceId)
        case .requestSignInLink(let email, let platform):
            try c.encode("requestSignInLink", forKey: .type)
            try c.encode(email, forKey: .email)
            try c.encodeIfPresent(platform, forKey: .platform)
        case .submitSignInLink(let input):
            try c.encode("submitSignInLink", forKey: .type)
            try c.encode(input, forKey: .input)
        case .signOut: try c.encode("signOut", forKey: .type)
        case .deleteListeningData(let newDeviceId):
            try c.encode("deleteListeningData", forKey: .type)
            try c.encode(newDeviceId, forKey: .newDeviceId)
        case .deleteAccount(let newDeviceId):
            try c.encode("deleteAccount", forKey: .type)
            try c.encode(newDeviceId, forKey: .newDeviceId)
        case .serverResponse(let id, let status, let body):
            try c.encode("serverResponse", forKey: .type)
            try c.encode(id, forKey: .id)
            try c.encode(status, forKey: .status)
            try c.encode(body, forKey: .body)
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
        case "setListeningTracking": self = .setListeningTracking(enabled: try c.decode(Bool.self, forKey: .enabled))
        case "restore":
            self = .restore(
                settingsJson: try c.decode(String.self, forKey: .settingsJson),
                listeningJson: try c.decode(String.self, forKey: .listeningJson),
                fallbackDeviceId: try c.decode(String.self, forKey: .fallbackDeviceId),
                accountJson: try c.decodeIfPresent(String.self, forKey: .accountJson) ?? "")
        case "beginListeningSync":
            self = .beginListeningSync(reason: try c.decode(SyncReason.self, forKey: .reason))
        case "resetListeningData":
            self = .resetListeningData(newDeviceId: try c.decode(String.self, forKey: .newDeviceId))
        case "requestSignInLink":
            self = .requestSignInLink(
                email: try c.decode(String.self, forKey: .email),
                platform: try c.decodeIfPresent(String.self, forKey: .platform))
        case "submitSignInLink":
            self = .submitSignInLink(input: try c.decode(String.self, forKey: .input))
        case "signOut": self = .signOut
        case "deleteListeningData":
            self = .deleteListeningData(newDeviceId: try c.decode(String.self, forKey: .newDeviceId))
        case "deleteAccount":
            self = .deleteAccount(newDeviceId: try c.decode(String.self, forKey: .newDeviceId))
        case "serverResponse":
            self = .serverResponse(
                id: try c.decode(UInt64.self, forKey: .id),
                status: try c.decode(UInt16.self, forKey: .status),
                body: try c.decodeIfPresent(String.self, forKey: .body) ?? "")
        default:
            throw DecodingError.dataCorruptedError(forKey: .type, in: c, debugDescription: "unknown command type: \(type)")
        }
    }
}

/// Why the shell is asking the core to sync (`SyncReason` in the core). The
/// shell decides *when it can* talk; the core decides whether there is
/// anything to say. There is no `threshold`: the core's tick checks the
/// threshold itself (ADR 0001).
enum SyncReason: String, Codable {
    /// Backgrounding / closing: send any unsynced time at all.
    case flush
    /// Launched with an account or just signed in: always send, to fetch the
    /// cross-device aggregate.
    case refresh
}

enum Effect: Decodable {
    /// `gain` is the final output level, 0–1, mute and curve already applied.
    case startPlayback(gain: Float)
    case pausePlayback
    case setPlatformVolume(gain: Float)
    case persistSettings(json: String)
    case persistListening(json: String)
    /// One request to carry to the sync server and settle with
    /// `.serverResponse`, whichever dispatch produced it.
    case serverRequest(ServerRequest)
    /// Store verbatim; `""` means delete the stored account.
    case persistAccount(json: String)

    private enum CodingKeys: String, CodingKey {
        case type, gain, json
    }

    init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        let type = try c.decode(String.self, forKey: .type)
        switch type {
        case "startPlayback":
            self = .startPlayback(gain: try c.decode(Float.self, forKey: .gain))
        case "pausePlayback":
            self = .pausePlayback
        case "setPlatformVolume":
            self = .setPlatformVolume(gain: try c.decode(Float.self, forKey: .gain))
        case "persistSettings":
            self = .persistSettings(json: try c.decode(String.self, forKey: .json))
        case "persistListening":
            self = .persistListening(json: try c.decode(String.self, forKey: .json))
        case "serverRequest":
            self = .serverRequest(try ServerRequest(from: decoder))
        case "persistAccount":
            self = .persistAccount(json: try c.decode(String.self, forKey: .json))
        default:
            throw DecodingError.dataCorruptedError(forKey: .type, in: c, debugDescription: "unknown effect type: \(type)")
        }
    }
}

/// The HTTP verbs the core's request table uses, sent uppercase.
enum HttpMethod: String, Decodable {
    case post = "POST"
    case put = "PUT"
    case delete = "DELETE"
}

/// One request to the sync server, exactly as the core describes it: `method`
/// to `path` (relative to the sync-server base URL), with a bearer token and
/// a JSON `body` when there are any. The core owns what each one means.
struct ServerRequest: Decodable {
    let id: UInt64
    let method: HttpMethod
    let path: String
    let bearerToken: String?
    /// Sent verbatim as the JSON request body.
    let body: String?
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
    /// A timer is running (something to cancel); false when off or just completed.
    let isActive: Bool
    /// One-line status, e.g. "Playing · 12:34 left".
    let statusLabel: String
}

/// A ready-made timer length the core offers.
struct TimerPreset: Decodable, Equatable {
    let minutes: Int
    let label: String
    let shortLabel: String
}

/// The timer choices the core offers: presets, limits and the custom pre-fill.
struct TimerOptions: Decodable, Equatable {
    let focusPresets: [TimerPreset]
    let sleepPresets: [TimerPreset]
    let minMinutes: Int
    let maxMinutes: Int
    let customFocusMinutes: Int
    let customSleepMinutes: Int
}

struct ListeningSnapshot: Decodable, Equatable {
    let trackingEnabled: Bool
    let deviceTotalMs: UInt64
    let displayedTotalMs: UInt64
    let unsyncedMs: UInt64
    let totalLabel: String
}

/// The account view. Never carries the session token.
struct AccountSnapshot: Decodable, Equatable {
    /// nil when signed out.
    let email: String?
    /// "Syncing · {email}"; nil when signed out.
    let signedInLabel: String?
    /// What the user was last told; nil when there is nothing to say.
    let statusLabel: String?
    /// An account request is out; every account control but sign-out waits.
    let busy: Bool
}

struct Snapshot: Decodable, Equatable {
    let title: String
    let subtitle: String
    let isPlaying: Bool
    let volumePercent: Int
    let isMuted: Bool
    /// Gain to output right now, 0–1: 0 while muted, else the curve applied.
    let outputGain: Float
    let primaryButtonLabel: String
    let timer: TimerSnapshot
    let timerOptions: TimerOptions
    let errorMessage: String?
    let listening: ListeningSnapshot
    let account: AccountSnapshot
    /// How often to send `.tick`, in ms; 0 means stop. The core owns the cadence.
    let tickIntervalMs: UInt64
}

struct Update: Decodable {
    let snapshot: Snapshot
    let effects: [Effect]
}
