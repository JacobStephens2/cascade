import { useEffect, useState } from "react";
import { useCascade } from "./core/useCascade";
import { PlayButton } from "./components/PlayButton";
import { VolumeSlider } from "./components/VolumeSlider";
import { TimerControls } from "./components/TimerControls";
import { TimerReadout } from "./components/TimerReadout";
import { ListeningStats } from "./components/ListeningStats";
import { AccountControls } from "./components/AccountControls";
import { WaterfallBackdrop } from "./components/WaterfallBackdrop";
import { useSync } from "./sync/useSync";

export function App() {
  const { ready, snapshot, loadError, dispatch } = useCascade();
  const sync = useSync(snapshot, dispatch);
  const [showCustomTimer, setShowCustomTimer] = useState(false);

  // Space bar toggles play/pause anywhere — the canonical media-app gesture —
  // except while typing in a field. We preventDefault so it doesn't also
  // scroll the page or re-activate a focused button (which would double-toggle).
  useEffect(() => {
    if (!ready) return;
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.code !== "Space" && e.key !== " ") return;
      if (e.repeat) return;
      const el = document.activeElement as HTMLElement | null;
      const tag = el?.tagName;
      if (
        tag === "INPUT" ||
        tag === "TEXTAREA" ||
        tag === "SELECT" ||
        el?.isContentEditable
      ) {
        return;
      }
      e.preventDefault();
      dispatch({ type: "togglePlayback" });
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [ready, dispatch]);

  if (loadError) {
    return (
      <div className="cascade-shell cascade-shell--error">
        <h1>Couldn't start Cascade</h1>
        <pre>{loadError}</pre>
      </div>
    );
  }

  if (!ready || !snapshot) {
    return (
      <div className="cascade-shell cascade-shell--loading">
        <div className="cascade-loading-mark" aria-hidden />
        <p>Loading Cascade…</p>
      </div>
    );
  }

  const isPlaying = snapshot.isPlaying;
  const timerKind = snapshot.timer.kind;

  return (
    <div className={`cascade-shell ${isPlaying ? "is-playing" : "is-paused"}`}>
      <WaterfallBackdrop active={isPlaying} progress={snapshot.timer.progress} />

      <header className="cascade-header">
        <span className="cascade-mark">Cascade</span>
        <span className="cascade-sub">{snapshot.subtitle}</span>
      </header>

      <main className="cascade-main">
        <TimerReadout snapshot={snapshot.timer} />

        <PlayButton
          isPlaying={isPlaying}
          label={snapshot.primaryButtonLabel}
          onClick={() => dispatch({ type: "togglePlayback" })}
        />

        <VolumeSlider
          percent={snapshot.volumePercent}
          isMuted={snapshot.isMuted}
          onChange={(percent) => dispatch({ type: "setVolume", percent })}
          onToggleMute={() => dispatch({ type: "toggleMute" })}
        />

        <TimerControls
          activeKind={timerKind}
          showCustom={showCustomTimer}
          onToggleCustom={() => setShowCustomTimer((v) => !v)}
          onStartPomodoro={(minutes) => {
            setShowCustomTimer(false);
            dispatch({ type: "startPomodoro", minutes });
          }}
          onStartSleep={(minutes) => {
            setShowCustomTimer(false);
            dispatch({ type: "startSleepTimer", minutes });
          }}
          onStartStopwatch={() => {
            setShowCustomTimer(false);
            dispatch({ type: "startStopwatch" });
          }}
          onCancel={() => dispatch({ type: "cancelTimer" })}
        />

        {snapshot.errorMessage && (
          <div className="cascade-error" role="alert">
            {snapshot.errorMessage}
          </div>
        )}

        <ListeningStats
          listening={snapshot.listening}
          onToggleTracking={(enabled) =>
            dispatch({ type: "setListeningTracking", enabled })
          }
        />

        <AccountControls sync={sync} />
      </main>

      <footer className="cascade-footer">
        <span>Cascade · looped</span>
      </footer>
    </div>
  );
}
