import Guacamole from '../vendor/guacamole.js';

/** What the player tells its owner. Times are milliseconds from the first frame. */
export interface PlayerEvents {
  /** More of the recording was read: its length so far. */
  readonly onprogress?: (durationMs: number) => void;
  /** The whole transcript was read; `duration()` is final. */
  readonly onloaded?: () => void;
  /** The shown frame changed, by play or by seek. */
  readonly onposition?: (positionMs: number) => void;
  /** Playback started or stopped (it stops by itself at the last frame). */
  readonly onplaying?: (playing: boolean) => void;
  /** The recorded display changed size, in pixels; the owner scales it to fit again. */
  readonly onresize?: (width: number, height: number) => void;
  /** The transcript could not be parsed; the frames before the fault still play. The vendor's text is not given. */
  readonly onunreadable?: () => void;
}

/** A recorded Guacamole session on a display of its own (RA-041). It holds no socket and sends nothing. */
export interface GuacamolePlayer {
  /**
   * The recorded display. The owner places it, sets its scale with `scale` to fit, and never moves or animates it
   * (MAT-008); playback takes no input.
   */
  readonly element: HTMLElement;
  /** The recorded display's size in pixels (0 before the first `size`). */
  size(): { readonly width: number; readonly height: number };
  /** Draw the display at `factor` times its size; a factor that is not a positive number is ignored. */
  scale(factor: number): void;
  play(): void;
  pause(): void;
  /**
   * Show the frame at `positionMs` (clamped to the recording), or one next to it when none starts then (1.5.5's
   * binary search); resolves when it is shown or another seek, play or pause replaced it.
   */
  seek(positionMs: number): Promise<void>;
  duration(): number;
  position(): number;
  playing(): boolean;
  /** Stop playback and reading, and drop the events; safe to call more than once. */
  dispose(): void;
}

/**
 * Play `transcript` (from `openTranscript`) with the 1.5.5 `Guacamole.SessionRecording`. Reading starts at once and
 * `onprogress` hears each frame; `play` and `seek` work on the frames read so far. 1.5.5 drops a seek in progress
 * without calling it back when play or pause starts, so both first end it (`cancel`), which resolves its promise.
 */
export function openGuacamolePlayer(transcript: Blob, events: PlayerEvents = {}): GuacamolePlayer {
  const recording = new Guacamole.SessionRecording(transcript);
  const display = recording.getDisplay();
  let frames = 0;
  let disposed = false;
  recording.onprogress = (duration) => {
    frames += 1;
    events.onprogress?.(duration);
  };
  recording.onload = () => events.onloaded?.();
  recording.onerror = () => events.onunreadable?.();
  recording.onseek = (position) => events.onposition?.(position);
  recording.onplay = () => events.onplaying?.(true);
  recording.onpause = () => events.onplaying?.(false);
  display.onresize = (width, height) => events.onresize?.(width, height);

  return {
    element: display.getElement(),
    size: () => ({ width: display.getWidth(), height: display.getHeight() }),
    scale: (factor) => {
      if (Number.isFinite(factor) && factor > 0) display.scale(factor);
    },
    play: () => {
      if (disposed) return;
      recording.cancel();
      recording.play();
    },
    pause: () => {
      recording.cancel();
      recording.pause();
    },
    seek: (positionMs) =>
      new Promise<void>((resolve) => {
        // 1.5.5 never calls back a seek on a recording with no frame yet.
        if (disposed || frames === 0) {
          resolve();
          return;
        }
        const target = Number.isFinite(positionMs) ? Math.min(Math.max(positionMs, 0), recording.getDuration()) : 0;
        recording.seek(target, resolve);
      }),
    duration: () => recording.getDuration(),
    position: () => recording.getPosition(),
    playing: () => recording.isPlaying(),
    dispose: () => {
      if (disposed) return;
      disposed = true;
      recording.onprogress = null;
      recording.onload = null;
      recording.onerror = null;
      recording.onseek = null;
      recording.onplay = null;
      recording.onpause = null;
      display.onresize = null;
      recording.cancel();
      recording.pause();
      recording.abort();
    },
  };
}
