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
  | { type: "listeningSyncSucceeded"; serverTotalMs: number }
  | { type: "listeningSyncFailed"; unauthorized: boolean }
  | { type: "resetListeningData"; newDeviceId: string }
  | { type: "requestSignInLink"; email: string }
  | { type: "submitSignInLink"; input: string }
  | { type: "signOut" }
  | { type: "deleteListeningData" }
  | { type: "deleteAccount" }
  | { type: "signInLinkSent" }
  | { type: "signInVerified"; sessionToken: string; email: string }
  | { type: "listeningDataDeleted"; newDeviceId: string }
  | { type: "accountDeleted"; newDeviceId: string }
  | { type: "accountRequestFailed"; unauthorized: boolean };

/** Why the shell is asking to sync; the core decides whether to send. */
export type SyncReason = "threshold" | "flush" | "refresh";

export type Effect =
  /** `gain` is the final output level, 0–1, mute and curve already applied. */
  | { type: "startPlayback"; gain: number }
  | { type: "pausePlayback" }
  | { type: "setPlatformVolume"; gain: number }
  | { type: "persistSettings"; json: string }
  | { type: "persistListening"; json: string }
  | {
      type: "pushListening";
      deviceId: string;
      deviceTotalMs: number;
      /** Null only for a shell that keeps its own account; never for web. */
      sessionToken: string | null;
    }
  | { type: "clearSession" }
  | { type: "sendSignInLink"; email: string }
  | { type: "verifySignInToken"; token: string }
  /** Fire-and-forget: no settle command. */
  | { type: "revokeSession"; sessionToken: string }
  | { type: "deleteServerListening"; sessionToken: string }
  | { type: "deleteServerAccount"; sessionToken: string }
  /** Store verbatim; `""` means delete the stored account. */
  | { type: "persistAccount"; json: string };

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
