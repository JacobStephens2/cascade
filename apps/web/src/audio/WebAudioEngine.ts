/**
 * Web Audio playback engine. The cascade-core never touches this — it just
 * tells us, via `Effect`s, when to start / pause / set volume. We own the
 * AudioContext lifecycle, the loop, and the fade ramps.
 *
 * The looping strategy is `AudioBufferSourceNode.loop = true` with the
 * decoded waterfall buffer. With an OGG Vorbis source that gives gapless
 * playback (MP3 has unavoidable encoder padding around the seam).
 */
export class WebAudioEngine {
  private ctx: AudioContext | null = null;
  private buffer: AudioBuffer | null = null;
  private source: AudioBufferSourceNode | null = null;
  private gain: GainNode | null = null;
  private loaded: Promise<void> | null = null;

  constructor(private readonly audioUrl: string) {}

  /** Lazily build the AudioContext and decode the file. Idempotent. */
  async ensureLoaded(): Promise<void> {
    if (this.loaded) return this.loaded;
    this.loaded = (async () => {
      // Construct lazily — browsers won't even let us instantiate until a
      // user gesture has happened, on iOS.
      this.ctx = new AudioContext();
      this.gain = this.ctx.createGain();
      this.gain.gain.value = 0;
      this.gain.connect(this.ctx.destination);

      const response = await fetch(this.audioUrl);
      if (!response.ok) {
        throw new Error(`failed to fetch ${this.audioUrl}: ${response.status}`);
      }
      const arrayBuffer = await response.arrayBuffer();
      this.buffer = await this.ctx.decodeAudioData(arrayBuffer);
    })();
    return this.loaded;
  }

  /**
   * Resume the AudioContext if it's suspended. On a page reload the browser's
   * autoplay policy can leave a restored "playing" session muted until the
   * user interacts; calling this from the first gesture starts the (already
   * scheduled) loop sounding. Safe to call when idle.
   */
  async resumeContext(): Promise<void> {
    if (this.ctx && this.ctx.state === "suspended") {
      try {
        await this.ctx.resume();
      } catch {
        // Still blocked; will retry on the next gesture.
      }
    }
  }

  async start(volumePercent: number, fadeMs = 400): Promise<void> {
    await this.ensureLoaded();
    if (!this.ctx || !this.buffer || !this.gain) return;
    if (this.ctx.state === "suspended") {
      await this.ctx.resume();
    }

    // Tear down any previous source — AudioBufferSourceNodes are one-shot.
    if (this.source) {
      try {
        this.source.stop();
      } catch {
        // Source may already be stopped; that's fine.
      }
      this.source.disconnect();
      this.source = null;
    }

    const source = this.ctx.createBufferSource();
    source.buffer = this.buffer;
    source.loop = true;
    source.connect(this.gain);
    source.start();
    this.source = source;

    this.rampGain(this.percentToGain(volumePercent), fadeMs);
  }

  async pause(fadeMs = 400): Promise<void> {
    if (!this.ctx || !this.gain || !this.source) return;
    this.rampGain(0, fadeMs);
    const sourceAtStop = this.source;
    // Stop the buffer source after the fade completes so it doesn't keep
    // running silently. setTimeout is fine here — we're just cleaning up.
    setTimeout(() => {
      try {
        sourceAtStop.stop();
      } catch {
        // Already stopped.
      }
      sourceAtStop.disconnect();
      if (this.source === sourceAtStop) this.source = null;
    }, fadeMs + 50);
  }

  setVolume(volumePercent: number): void {
    if (!this.gain) return;
    this.rampGain(this.percentToGain(volumePercent), 80);
  }

  /** True when audio is currently routed to the destination. */
  get isActive(): boolean {
    return this.source !== null;
  }

  private rampGain(target: number, fadeMs: number): void {
    if (!this.ctx || !this.gain) return;
    const safeTarget = Number.isFinite(target) ? target : 0;
    const now = this.ctx.currentTime;
    const param = this.gain.gain;
    const startValue = Number.isFinite(param.value) ? param.value : 0;
    // cancelScheduledValues + setValueAtTime guards against overlapping ramps.
    param.cancelScheduledValues(now);
    param.setValueAtTime(startValue, now);
    param.linearRampToValueAtTime(safeTarget, now + fadeMs / 1000);
  }

  private percentToGain(percent: number): number {
    // Perceptually log-ish curve — a slider at 50 should sound roughly half
    // as loud, not "almost full volume". Square law is a decent cheap proxy.
    const safe = Number.isFinite(percent) ? percent : 60;
    const clamped = Math.max(0, Math.min(100, safe)) / 100;
    return clamped * clamped;
  }
}
