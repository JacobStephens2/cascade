interface PlayButtonProps {
  isPlaying: boolean;
  label: string;
  onClick: () => void;
}

export function PlayButton({ isPlaying, label, onClick }: PlayButtonProps) {
  return (
    <button
      type="button"
      className={`play-button ${isPlaying ? "is-playing" : "is-paused"}`}
      onClick={onClick}
      aria-label={label}
    >
      <span className="play-button__ring" aria-hidden />
      <span className="play-button__ring play-button__ring--outer" aria-hidden />
      <span className="play-button__icon" aria-hidden>
        {isPlaying ? (
          <svg viewBox="0 0 32 32" width="48" height="48">
            <rect x="9" y="6" width="5" height="20" rx="1.5" />
            <rect x="18" y="6" width="5" height="20" rx="1.5" />
          </svg>
        ) : (
          <svg viewBox="0 0 32 32" width="48" height="48">
            <path d="M10 6 L26 16 L10 26 Z" />
          </svg>
        )}
      </span>
      <span className="visually-hidden">{label}</span>
    </button>
  );
}
