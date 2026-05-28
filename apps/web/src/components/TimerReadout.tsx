import type { TimerSnapshot } from "../core/types";

interface TimerReadoutProps {
  snapshot: TimerSnapshot;
}

export function TimerReadout({ snapshot }: TimerReadoutProps) {
  const { kind, remainingLabel, progress } = snapshot;

  if (kind === "off") {
    return (
      <div className="timer-readout timer-readout--idle">
        <span className="timer-readout__placeholder">No timer running</span>
      </div>
    );
  }

  if (kind === "justCompleted") {
    return (
      <div className="timer-readout timer-readout--done">
        <span className="timer-readout__time">{remainingLabel}</span>
      </div>
    );
  }

  const label = kind === "pomodoro" ? "Focus session" : "Sleep timer";
  const pct = Math.round(progress * 100);

  return (
    <div className={`timer-readout timer-readout--${kind}`}>
      <span className="timer-readout__kind">{label}</span>
      <span className="timer-readout__time">{remainingLabel}</span>
      <div
        className="timer-readout__bar"
        role="progressbar"
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={pct}
      >
        <div
          className="timer-readout__bar-fill"
          style={{ transform: `scaleX(${progress})` }}
        />
      </div>
    </div>
  );
}
