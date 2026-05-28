interface WaterfallBackdropProps {
  active: boolean;
  progress: number;
}

/**
 * Visual hint of "water flowing" — pure CSS gradients animated by
 * `is-playing`. The progress prop gently shifts the gradient stops so a
 * running timer has visible movement on top of the audio.
 */
export function WaterfallBackdrop({ active, progress }: WaterfallBackdropProps) {
  const tint = `hsl(${200 - progress * 40}, 70%, ${active ? 22 : 14}%)`;
  return (
    <div
      className={`waterfall-backdrop ${active ? "is-active" : ""}`}
      style={{ ["--tint" as string]: tint }}
      aria-hidden
    >
      <div className="waterfall-backdrop__layer waterfall-backdrop__layer--a" />
      <div className="waterfall-backdrop__layer waterfall-backdrop__layer--b" />
      <div className="waterfall-backdrop__layer waterfall-backdrop__layer--c" />
    </div>
  );
}
