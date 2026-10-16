import { useEffect, useState } from "react";
import type { TimerOptions } from "../core/types";

interface TimerControlsProps {
  isActive: boolean;
  options: TimerOptions;
  showCustom: boolean;
  onToggleCustom: () => void;
  onStartPomodoro: (minutes: number) => void;
  onStartSleep: (minutes: number) => void;
  onStartStopwatch: () => void;
  onCancel: () => void;
}

export function TimerControls({
  isActive,
  options,
  showCustom,
  onToggleCustom,
  onStartPomodoro,
  onStartSleep,
  onStartStopwatch,
  onCancel,
}: TimerControlsProps) {
  // `null` until the user types, so the field shows the core's pre-fill for
  // the chosen mode.
  const [customText, setCustomText] = useState<string | null>(null);
  const [customMode, setCustomMode] = useState<"focus" | "sleep">("focus");
  const customMinutes =
    customText ??
    String(
      customMode === "focus"
        ? options.customFocusMinutes
        : options.customSleepMinutes,
    );
  const chooseMode = (mode: "focus" | "sleep") => {
    setCustomMode(mode);
    setCustomText(null);
  };

  // Collapse the custom panel automatically when a timer is running.
  useEffect(() => {
    if (isActive && showCustom) onToggleCustom();
    // We intentionally don't include onToggleCustom — it changes per render.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isActive]);

  if (isActive) {
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
          {options.focusPresets.map((p) => (
            <button
              key={p.minutes}
              type="button"
              className="chip"
              onClick={() => onStartPomodoro(p.minutes)}
            >
              {p.label}
            </button>
          ))}
        </div>
      </div>

      <div className="timer-controls__section">
        <h2 className="timer-controls__heading">Sleep timer</h2>
        <div className="timer-controls__row">
          {options.sleepPresets.map((p) => (
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

      <div className="timer-controls__section">
        <h2 className="timer-controls__heading">Stopwatch</h2>
        <div className="timer-controls__row">
          <button
            type="button"
            className="chip chip--ghost"
            onClick={onStartStopwatch}
          >
            Start stopwatch
          </button>
        </div>
      </div>

      <div className="timer-controls__section">
        <h2 className="timer-controls__heading">Custom</h2>
        <div className="timer-controls__row">
          <button
            type="button"
            className={`chip chip--ghost ${showCustom ? "is-active" : ""}`}
            onClick={onToggleCustom}
            aria-expanded={showCustom}
          >
            Custom…
          </button>
        </div>
      </div>

      {showCustom && (
        <form
          className="timer-controls__custom"
          onSubmit={(e) => {
            e.preventDefault();
            // Only a positive whole number crosses; the core clamps the rest.
            const minutes = Number(customMinutes);
            if (Number.isInteger(minutes) && minutes > 0) {
              setCustomText(null);
              if (customMode === "focus") onStartPomodoro(minutes);
              else onStartSleep(minutes);
            }
          }}
        >
          <div
            className="timer-controls__mode"
            role="radiogroup"
            aria-label="Custom timer mode"
          >
            <button
              type="button"
              role="radio"
              aria-checked={customMode === "focus"}
              className={`chip chip--ghost ${customMode === "focus" ? "is-active" : ""}`}
              onClick={() => chooseMode("focus")}
            >
              Focus
            </button>
            <button
              type="button"
              role="radio"
              aria-checked={customMode === "sleep"}
              className={`chip chip--ghost ${customMode === "sleep" ? "is-active" : ""}`}
              onClick={() => chooseMode("sleep")}
            >
              Sleep
            </button>
          </div>
          <label htmlFor="custom-minutes">
            Minutes
            <input
              id="custom-minutes"
              type="number"
              min={options.minMinutes}
              max={options.maxMinutes}
              step={1}
              value={customMinutes}
              onChange={(e) => setCustomText(e.target.value)}
            />
          </label>
          <button type="submit" className="chip chip--primary">
            {customMode === "focus" ? "Start session" : "Start sleep timer"}
          </button>
        </form>
      )}
    </div>
  );
}
