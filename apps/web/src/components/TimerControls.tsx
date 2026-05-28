import { useEffect, useState } from "react";
import type { TimerKind } from "../core/types";

interface TimerControlsProps {
  activeKind: TimerKind;
  showCustom: boolean;
  onToggleCustom: () => void;
  onStartPomodoro: (minutes: number) => void;
  onStartSleep: (minutes: number) => void;
  onCancel: () => void;
}

const POMODORO_PRESETS: { label: string; minutes: number }[] = [
  { label: "30 min", minutes: 30 },
  { label: "1 hr", minutes: 60 },
  { label: "8 hr", minutes: 8 * 60 },
];

const SLEEP_PRESETS: { label: string; minutes: number }[] = [
  { label: "15 min", minutes: 15 },
  { label: "30 min", minutes: 30 },
  { label: "1 hr", minutes: 60 },
];

export function TimerControls({
  activeKind,
  showCustom,
  onToggleCustom,
  onStartPomodoro,
  onStartSleep,
  onCancel,
}: TimerControlsProps) {
  const [customMinutes, setCustomMinutes] = useState("45");

  const timerRunning = activeKind === "sleep" || activeKind === "pomodoro";

  // Collapse the custom panel automatically when a timer is running.
  useEffect(() => {
    if (timerRunning && showCustom) onToggleCustom();
    // We intentionally don't include onToggleCustom — it changes per render.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [timerRunning]);

  if (timerRunning) {
    return (
      <div className="timer-controls timer-controls--running">
        <button
          type="button"
          className="timer-controls__cancel"
          onClick={onCancel}
        >
          Cancel timer
        </button>
      </div>
    );
  }

  return (
    <div className="timer-controls">
      <div className="timer-controls__section">
        <h2 className="timer-controls__heading">Focus session</h2>
        <div className="timer-controls__row">
          {POMODORO_PRESETS.map((p) => (
            <button
              key={p.minutes}
              type="button"
              className="chip"
              onClick={() => onStartPomodoro(p.minutes)}
            >
              {p.label}
            </button>
          ))}
          <button
            type="button"
            className={`chip chip--ghost ${showCustom ? "is-active" : ""}`}
            onClick={onToggleCustom}
          >
            Custom…
          </button>
        </div>
      </div>

      <div className="timer-controls__section">
        <h2 className="timer-controls__heading">Sleep timer</h2>
        <div className="timer-controls__row">
          {SLEEP_PRESETS.map((p) => (
            <button
              key={p.minutes}
              type="button"
              className="chip chip--ghost"
              onClick={() => onStartSleep(p.minutes)}
            >
              {p.label}
            </button>
          ))}
        </div>
      </div>

      {showCustom && (
        <form
          className="timer-controls__custom"
          onSubmit={(e) => {
            e.preventDefault();
            const n = Number(customMinutes);
            if (Number.isFinite(n) && n > 0 && n <= 24 * 60) {
              onStartPomodoro(Math.round(n));
            }
          }}
        >
          <label htmlFor="custom-minutes">
            Minutes
            <input
              id="custom-minutes"
              type="number"
              min={1}
              max={1440}
              step={1}
              value={customMinutes}
              onChange={(e) => setCustomMinutes(e.target.value)}
            />
          </label>
          <button type="submit" className="chip chip--primary">
            Start session
          </button>
        </form>
      )}
    </div>
  );
}
