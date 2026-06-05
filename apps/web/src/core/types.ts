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
  | { type: "restoreListening"; json: string }
  | { type: "applySyncedTotal"; syncedThroughMs: number; serverTotalMs: number }
  | { type: "resetListeningData" };

export type Effect =
  | { type: "startPlayback"; volumePercent: number }
  | { type: "pausePlayback" }
  | { type: "setPlatformVolume"; volumePercent: number }
  | { type: "persistSettings"; json: string }
  | { type: "persistListening"; json: string };

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
}

export interface ListeningSnapshot {
  trackingEnabled: boolean;
  deviceTotalMs: number;
  displayedTotalMs: number;
  unsyncedMs: number;
  totalLabel: string;
}

export interface Snapshot {
  title: string;
  subtitle: string;
  isPlaying: boolean;
  volumePercent: number;
  isMuted: boolean;
  primaryButtonLabel: string;
  timer: TimerSnapshot;
  errorMessage: string | null;
  listening: ListeningSnapshot;
}

export interface Update {
  snapshot: Snapshot;
  effects: Effect[];
}
