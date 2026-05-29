import { useCallback, useEffect, useRef, useState } from "react";
import init, { CascadeCore } from "../wasm/cascade_wasm.js";
import wasmUrl from "../wasm/cascade_wasm_bg.wasm?url";
import { WebAudioEngine } from "../audio/WebAudioEngine";
import type { Command, Effect, Snapshot, TimerKind } from "./types";

const SETTINGS_STORAGE_KEY = "cascade.settings.v1";
// Live session (play state + running timer/stopwatch) so a page reload picks
// up where it left off. Separate from settings, which the Rust core owns.
const SESSION_STORAGE_KEY = "cascade.session.v1";
const WATERFALL_URL = "/sounds/waterfall.ogg";
const TICK_INTERVAL_MS = 250;

interface SessionState {
  isPlaying: boolean;
  timerKind: TimerKind;
  totalMs: number;
  elapsedMs: number;
  savedAt: number;
}

interface UseCascadeResult {
  ready: boolean;
  snapshot: Snapshot | null;
  loadError: string | null;
  dispatch: (command: Command) => void;
}

/**
 * Owns the cascade-core WASM instance, the audio engine, the tick loop, and
 * the localStorage-backed settings persistence. The whole app reads from
 * `snapshot` and pushes commands through `dispatch`.
 */
export function useCascade(): UseCascadeResult {
  const coreRef = useRef<CascadeCore | null>(null);
  const audioRef = useRef<WebAudioEngine | null>(null);
  // Session captured from localStorage at boot (before any dispatch can
  // overwrite it), replayed once the core is ready.
  const pendingRestoreRef = useRef<SessionState | null>(null);
  const restoredRef = useRef(false);
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [ready, setReady] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        await init({ module_or_path: wasmUrl });
        if (cancelled) return;

        // Capture the live session now, before the persist effect can rewrite it.
        try {
          const sessionJson = localStorage.getItem(SESSION_STORAGE_KEY);
          pendingRestoreRef.current = sessionJson
            ? (JSON.parse(sessionJson) as SessionState)
            : null;
        } catch {
          pendingRestoreRef.current = null;
        }

        const savedJson = localStorage.getItem(SETTINGS_STORAGE_KEY);
        let core: CascadeCore;
        if (savedJson) {
          try {
            core = CascadeCore.restore(savedJson);
          } catch (err) {
            console.warn("Could not restore settings, starting fresh.", err);
            core = new CascadeCore();
          }
        } else {
          core = new CascadeCore();
        }
        coreRef.current = core;
        audioRef.current = new WebAudioEngine(WATERFALL_URL);
        setSnapshot(JSON.parse(core.snapshot()) as Snapshot);
        setReady(true);
      } catch (err) {
        setLoadError(err instanceof Error ? err.message : String(err));
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  const runEffects = useCallback(async (effects: Effect[]) => {
    const audio = audioRef.current;
    if (!audio) return;
    for (const effect of effects) {
      switch (effect.type) {
        case "startPlayback":
          try {
            await audio.start(effect.volumePercent);
            queueMicrotask(() =>
              dispatchInternal({ type: "platformPlaybackStarted" }),
            );
          } catch (err) {
            queueMicrotask(() =>
              dispatchInternal({
                type: "platformPlaybackError",
                message: err instanceof Error ? err.message : String(err),
              }),
            );
          }
          break;
        case "pausePlayback":
          await audio.pause();
          break;
        case "setPlatformVolume":
          audio.setVolume(effect.volumePercent);
          break;
        case "persistSettings":
          try {
            localStorage.setItem(SETTINGS_STORAGE_KEY, effect.json);
          } catch (err) {
            console.warn("Could not persist settings", err);
          }
          break;
      }
    }
  }, []);

  // `dispatch` from inside async effects needs a stable reference; the
  // exported `dispatch` (below) just delegates.
  const dispatchInternal = useCallback(
    (command: Command) => {
      const core = coreRef.current;
      if (!core) return;
      const updateJson = core.dispatch(JSON.stringify(command));
      const update = JSON.parse(updateJson) as {
        snapshot: Snapshot;
        effects: Effect[];
      };
      setSnapshot(update.snapshot);
      void runEffects(update.effects);
    },
    [runEffects],
  );

  const dispatch = useCallback(
    (command: Command) => {
      dispatchInternal(command);
    },
    [dispatchInternal],
  );

  // Tick loop. Only ticks when a timer is running — no point churning React
  // state otherwise.
  useEffect(() => {
    if (!ready) return;
    const isTimerActive =
      snapshot?.timer.kind === "sleep" ||
      snapshot?.timer.kind === "pomodoro" ||
      snapshot?.timer.kind === "stopwatch";
    if (!isTimerActive) return;

    let last = performance.now();
    let frame: number;
    const tick = () => {
      const now = performance.now();
      const elapsed = Math.round(now - last);
      if (elapsed >= TICK_INTERVAL_MS) {
        last = now;
        dispatchInternal({ type: "tick", elapsedMs: elapsed });
      }
      frame = window.setTimeout(tick, TICK_INTERVAL_MS);
    };
    frame = window.setTimeout(tick, TICK_INTERVAL_MS);
    return () => window.clearTimeout(frame);
  }, [ready, snapshot?.timer.kind, dispatchInternal]);

  // Media Session: lets macOS route the keyboard's play/pause media key (and
  // Control Center / Now Playing) to the app. The OS decides whether to send
  // "play" or "pause" based on the playbackState we report below.
  useEffect(() => {
    if (!ready || !("mediaSession" in navigator)) return;
    const ms = navigator.mediaSession;
    ms.metadata = new MediaMetadata({
      title: "Cascade",
      artist: "Waterfall loop",
      artwork: [
        { src: "/icon-192.png", sizes: "192x192", type: "image/png" },
        { src: "/icon-512.png", sizes: "512x512", type: "image/png" },
      ],
    });
    ms.setActionHandler("play", () => dispatchInternal({ type: "play" }));
    ms.setActionHandler("pause", () => dispatchInternal({ type: "pause" }));
    ms.setActionHandler("stop", () => dispatchInternal({ type: "pause" }));
    return () => {
      ms.setActionHandler("play", null);
      ms.setActionHandler("pause", null);
      ms.setActionHandler("stop", null);
    };
  }, [ready, dispatchInternal]);

  // Keep the OS Now Playing state in sync so the media key toggles correctly.
  useEffect(() => {
    if (!ready || !("mediaSession" in navigator)) return;
    navigator.mediaSession.playbackState = snapshot?.isPlaying
      ? "playing"
      : "paused";
  }, [ready, snapshot?.isPlaying]);

  // Restore the session (running timer/stopwatch + play state) once, after the
  // core is ready. Runs exactly once; the saved blob was captured at boot.
  useEffect(() => {
    if (!ready || restoredRef.current) return;
    restoredRef.current = true;
    const r = pendingRestoreRef.current;
    pendingRestoreRef.current = null;
    if (!r) return;

    // Catch the stopwatch/countdown up for the time the page was gone.
    const offline = Math.max(0, Date.now() - (r.savedAt ?? Date.now()));
    if (r.timerKind === "stopwatch") {
      dispatchInternal({ type: "startStopwatch" });
      dispatchInternal({ type: "tick", elapsedMs: r.elapsedMs + offline });
    } else if (r.timerKind === "sleep" || r.timerKind === "pomodoro") {
      const minutes = Math.max(1, Math.round(r.totalMs / 60000));
      dispatchInternal({
        type: r.timerKind === "sleep" ? "startSleepTimer" : "startPomodoro",
        minutes,
      });
      dispatchInternal({ type: "tick", elapsedMs: r.elapsedMs + offline });
    }
    // Reconcile playback to what it was before the reload.
    dispatchInternal({ type: r.isPlaying ? "play" : "pause" });
  }, [ready, dispatchInternal]);

  // Persist the live session on every change so a reload can resume it.
  useEffect(() => {
    if (!ready || !snapshot) return;
    const t = snapshot.timer;
    const elapsedMs =
      t.kind === "stopwatch"
        ? t.remainingMs
        : t.kind === "sleep" || t.kind === "pomodoro"
          ? Math.max(0, t.totalMs - t.remainingMs)
          : 0;
    const session: SessionState = {
      isPlaying: snapshot.isPlaying,
      timerKind: t.kind,
      totalMs: t.totalMs,
      elapsedMs,
      savedAt: Date.now(),
    };
    try {
      localStorage.setItem(SESSION_STORAGE_KEY, JSON.stringify(session));
    } catch {
      // Storage full / unavailable — non-fatal.
    }
  }, [ready, snapshot]);

  // Autoplay policy can leave a restored "playing" session suspended until the
  // user interacts. Resume the audio context on the first gesture.
  useEffect(() => {
    if (!ready) return;
    const resume = () => void audioRef.current?.resumeContext();
    window.addEventListener("pointerdown", resume, { once: true });
    window.addEventListener("keydown", resume, { once: true });
    return () => {
      window.removeEventListener("pointerdown", resume);
      window.removeEventListener("keydown", resume);
    };
  }, [ready]);

  return { ready, snapshot, loadError, dispatch };
}
