interface VolumeSliderProps {
  percent: number;
  isMuted: boolean;
  onChange: (percent: number) => void;
  onToggleMute: () => void;
}

export function VolumeSlider({
  percent,
  isMuted,
  onChange,
  onToggleMute,
}: VolumeSliderProps) {
  return (
    <div className="volume-slider">
      <label htmlFor="volume" className="volume-slider__label">
        <span className="volume-slider__label-left">
          <button
            type="button"
            className={`volume-slider__mute ${isMuted ? "is-muted" : ""}`}
            onClick={onToggleMute}
            aria-pressed={isMuted}
            aria-label={isMuted ? "Unmute" : "Mute"}
            title={isMuted ? "Unmute (timer keeps running)" : "Mute (keep timer running)"}
          >
            {isMuted ? "🔇" : "🔊"}
          </button>
          Volume
        </span>
        <span className="volume-slider__readout">{isMuted ? "Muted" : `${percent}%`}</span>
      </label>
      <input
        id="volume"
        type="range"
        min={0}
        max={100}
        step={1}
        value={percent}
        // The CSS uses --pct to position the filled-track gradient stop.
        style={{ ["--pct" as string]: `${percent}%` }}
        onChange={(e) => onChange(Number(e.target.value))}
      />
    </div>
  );
}
