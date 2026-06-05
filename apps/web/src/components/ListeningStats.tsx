import type { ListeningSnapshot } from "../core/types";

interface ListeningStatsProps {
  listening: ListeningSnapshot;
  onToggleTracking: (enabled: boolean) => void;
}

/**
 * Lifetime listening readout plus the on/off toggle. The total is a single
 * cumulative number — never a timeline — and stays on the device unless the
 * user signs in to sync it.
 */
export function ListeningStats({
  listening,
  onToggleTracking,
}: ListeningStatsProps) {
  const { trackingEnabled, totalLabel } = listening;
  return (
    <section className="listening" aria-label="Listening time">
      <div className="listening__readout">
        <span
          className={`listening__value ${trackingEnabled ? "" : "is-off"}`}
        >
          {totalLabel}
        </span>
        <span className="listening__caption">
          {trackingEnabled ? "lifetime listening" : "tracking paused"}
        </span>
      </div>
      <label className="listening__toggle">
        <input
          type="checkbox"
          checked={trackingEnabled}
          onChange={(e) => onToggleTracking(e.target.checked)}
        />
        <span>Track my listening time</span>
      </label>
    </section>
  );
}
