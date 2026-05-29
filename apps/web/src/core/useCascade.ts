import { useCallback, useEffect, useRef, useState } from "react";
import init, { CascadeCore } from "../wasm/cascade_wasm.js";
import wasmUrl from "../wasm/cascade_wasm_bg.wasm?url";
import { WebAudioEngine } from "../audio/WebAudioEngine";
import type { Command, Effect, Snapshot } from "./types";

const SETTINGS_STORAGE_KEY = "cascade.settings.v1";
const WATERFALL_URL = "/sounds/waterfall.ogg";
const TICK_INTERVAL_MS = 250;

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
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [ready, setReady] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        await init({ module_or_path: wasmUrl });
        if (cancelled) return;

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

  return { ready, snapshot, loadError, dispatch };
}
