/**
 * Mirrors the serde-tagged enums in `cascade-core`. The Rust side is the
 * source of truth; if these get out of sync the JSON shapes will fail to
 * round-trip and you'll see it immediately in dev.
 */

export type Command =
  | { type: "play" }
  | { type: "pause" }
  | { type: "togglePlayback" }
  | { type: "setVolume"; percent: number }
  | { type: "toggleMute" }
  | { type: "startSleepTimer"; minutes: number }
  | { type: "startPomodoro"; minutes: number }
  | { type: "startStopwatch" }
  | { type: "cancelTimer" }
  | { type: "tick"; elapsedMs: number }
  | { type: "platformPlaybackStarted" }
  | { type: "platformPlaybackPaused" }
  | { type: "platformPlaybackError"; message: string }
  | { type: "setListeningTracking"; enabled: boolean }
  | {
      type: "restore";
      settingsJson: string;
      listeningJson: string;
      fallbackDeviceId: string;
      /** The stored account blob, verbatim; `""` for none. */
      accountJson: string;
    }
  | { type: "beginListeningSync"; reason: SyncReason }
  | { type: "resetListeningData"; newDeviceId: string }
  /** `platform` names the app the emailed link hands off to, if any. */
  | { type: "requestSignInLink"; email: string; platform?: string }
  | { type: "submitSignInLink"; input: string }
  | { type: "signOut" }
  /** `newDeviceId` is a fresh UUID, adopted once the server confirms. */
  | { type: "deleteListeningData"; newDeviceId: string }
  | { type: "deleteAccount"; newDeviceId: string }
  /** Settles the `serverRequest` with the same `id`: the HTTP status (0 when
   * not sent or no response) and the response body verbatim. */
  | { type: "serverResponse"; id: number; status: number; body: string };

/** Why the shell is asking to sync; the core decides whether to send. The
 * routine threshold sync is the core's own, answered from a tick. */
export type SyncReason = "flush" | "refresh";

export type Effect =
  /** `gain` is the final output level, 0–1, mute and curve already applied. */
  | { type: "startPlayback"; gain: number }
  | { type: "pausePlayback" }
  | { type: "setPlatformVolume"; gain: number }
  | { type: "persistSettings"; json: string }
  | { type: "persistListening"; json: string }
  /** Send to the sync server exactly as described; settle with
   * `serverResponse`, always. */
  | ServerRequest
  /** Store verbatim; `""` means delete the stored account. */
  | { type: "persistAccount"; json: string };

/** One request to the sync server. `path` is relative to the base URL;
 * `body`, when present, is JSON to send verbatim. */
export interface ServerRequest {
  type: "serverRequest";
  id: number;
  method: HttpMethod;
  path: string;
  bearerToken?: string;
  body?: string;
}

export type HttpMethod = "POST" | "PUT" | "DELETE";

export type TimerKind =
  | "off"
  | "sleep"
  | "pomodoro"
  | "stopwatch"
  | "justCompleted";

export interface TimerSnapshot {
  kind: TimerKind;
  remainingLabel: string;
  remainingMs: number;
  totalMs: number;
  progress: number;
  /** A timer is running (something to cancel); false when off or just completed. */
  isActive: boolean;
  /** One-line status, e.g. "Playing · 12:34 left". */
  statusLabel: string;
}

/** A ready-made timer length the core offers. */
export interface TimerPreset {
  minutes: number;
  label: string;
  shortLabel: string;
}

/** The timer choices the core offers: presets, limits and the custom pre-fill. */
export interface TimerOptions {
  focusPresets: TimerPreset[];
  sleepPresets: TimerPreset[];
  minMinutes: number;
  maxMinutes: number;
  customFocusMinutes: number;
  customSleepMinutes: number;
}

export interface ListeningSnapshot {
  trackingEnabled: boolean;
  deviceTotalMs: number;
  displayedTotalMs: number;
  unsyncedMs: number;
  totalLabel: string;
}

/** The account view. Never carries the session token. */
export interface AccountSnapshot {
  /** Null when signed out. */
  email: string | null;
  /** "Syncing · {email}"; null when signed out. */
  signedInLabel: string | null;
  /** What the user was last told; null when there is nothing to say. */
  statusLabel: string | null;
  /** An account request is out; every account control but sign-out waits. */
  busy: boolean;
}

export interface Snapshot {
  title: string;
  subtitle: string;
  isPlaying: boolean;
  volumePercent: number;
  isMuted: boolean;
  /** Gain to output right now, 0–1: 0 while muted, else the curve applied. */
  outputGain: number;
  primaryButtonLabel: string;
  timer: TimerSnapshot;
  timerOptions: TimerOptions;
  errorMessage: string | null;
  listening: ListeningSnapshot;
  account: AccountSnapshot;
  /** How often to send `tick`, in ms; 0 means stop. The core owns the cadence. */
  tickIntervalMs: number;
}

export interface Update {
  snapshot: Snapshot;
  effects: Effect[];
}
