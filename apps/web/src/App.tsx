import { useState } from "react";
import { useCascade } from "./core/useCascade";
import { PlayButton } from "./components/PlayButton";
import { VolumeSlider } from "./components/VolumeSlider";
import { TimerControls } from "./components/TimerControls";
import { TimerReadout } from "./components/TimerReadout";
import { WaterfallBackdrop } from "./components/WaterfallBackdrop";

export function App() {
  const { ready, snapshot, loadError, dispatch } = useCascade();
  const [showCustomTimer, setShowCustomTimer] = useState(false);

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
        <p>Loading the falls…</p>
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
          onCancel={() => dispatch({ type: "cancelTimer" })}
        />

        {snapshot.errorMessage && (
          <div className="cascade-error" role="alert">
            {snapshot.errorMessage}
          </div>
        )}
      </main>

      <footer className="cascade-footer">
        <span>The Falls v3.1 · looped</span>
      </footer>
    </div>
  );
}
