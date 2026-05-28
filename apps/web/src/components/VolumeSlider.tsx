interface VolumeSliderProps {
  percent: number;
  onChange: (percent: number) => void;
}

export function VolumeSlider({ percent, onChange }: VolumeSliderProps) {
  return (
    <div className="volume-slider">
      <label htmlFor="volume" className="volume-slider__label">
        Volume
        <span className="volume-slider__readout">{percent}%</span>
      </label>
      <input
        id="volume"
        type="range"
        min={0}
        max={100}
        step={1}
        value={percent}
        onChange={(e) => onChange(Number(e.target.value))}
      />
    </div>
  );
}
