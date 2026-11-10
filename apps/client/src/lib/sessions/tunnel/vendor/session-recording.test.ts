// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { afterAll, beforeAll, describe, expect, it, vi } from 'vitest';
import Guacamole from './guacamole.js';

/**
 * Proof that the vendored `Guacamole.SessionRecording` plays what the hub tees (K4 §4 and §11, RA-041, D-12,
 * R-045): every gateway instruction relayed after guacd's `ready`, concatenated, without the internal
 * `0.,36.<uuid>;` instruction. The bytes are real guacd 1.6.0 VNC output (WP-47 T1b): the opening of a join
 * minus its `ready`, then the frames guacd sent after the owner left. Only the DOM boundary is faked
 * (canvas 2D context, `createImageBitmap`); nothing inside Guacamole is.
 */

const fixtures = join(import.meta.dirname, ...Array<string>(7).fill('..'), 'crates/hub/src/sessions/tunnel/tests/fixtures');
const fixture = (name: string): string => readFileSync(join(fixtures, name), 'utf8');
const READY = /^5\.ready,37\.[^;]*;/;

const opening = fixture('guacd-1.6.0-vnc-join-opening.guac');
const afterReady = opening.replace(READY, '');
const stream = afterReady + fixture('guacd-1.6.0-vnc-join-owner-left.guac');

/** The `sync` timestamps in the stream, in order. */
const syncs = [...stream.matchAll(/4\.sync,\d+\.(\d+),/g)].map((match) => Number(match[1]));
/** The `cfill` colour of each frame after the opening (all on layer 0 through mask 14, alpha 255). */
const fills = [...fixture('guacd-1.6.0-vnc-join-owner-left.guac').matchAll(/5\.cfill,2\.14,1\.0,\d+\.(\d+),\d+\.(\d+),\d+\.(\d+),3\.255;/g)].map(
  (match) => `rgba(${match[1]},${match[2]},${match[3]},1)`,
);
const at = (frame: number): number => (syncs[frame] ?? Number.NaN) - (syncs[0] ?? Number.NaN);

interface FakeContext {
  fillStyle: string;
  /** The `fillStyle` at each `fill()`, in order. */
  fills: string[];
  /** Every `fill` (with its colour) and `drawImage`, in order. */
  ops: string[];
}

const contexts = new WeakMap<HTMLCanvasElement, FakeContext>();

function fakeContext(): FakeContext {
  const state: FakeContext = { fillStyle: '', fills: [], ops: [] };
  const methods: Record<string, unknown> = {
    fill: () => {
      state.fills.push(state.fillStyle);
      state.ops.push(`fill ${state.fillStyle}`);
    },
    drawImage: () => state.ops.push('drawImage'),
    getImageData: (_x: number, _y: number, w: number, h: number) => ({ width: w, height: h, data: new Uint8ClampedArray(w * h * 4) }),
  };
  return new Proxy(state, {
    get: (target, key: string) => (key in target ? target[key as keyof FakeContext] : (methods[key] ?? (() => undefined))),
    set: (target, key: string, value: unknown) => {
      (target as unknown as Record<string, unknown>)[key] = value;
      return true;
    },
  });
}

const original = {
  getContext: HTMLCanvasElement.prototype.getContext,
  toDataURL: HTMLCanvasElement.prototype.toDataURL,
  imageSrc: Object.getOwnPropertyDescriptor(HTMLImageElement.prototype, 'src'),
  createImageBitmap: (window as { createImageBitmap?: unknown }).createImageBitmap,
};

beforeAll(() => {
  HTMLCanvasElement.prototype.getContext = function (this: HTMLCanvasElement) {
    let context = contexts.get(this);
    if (!context) {
      context = fakeContext();
      contexts.set(this, context);
    }
    return context as unknown as CanvasRenderingContext2D;
  } as unknown as typeof original.getContext;
  HTMLCanvasElement.prototype.toDataURL = () => 'data:image/png;base64,AAAA';
  // jsdom never loads images without the canvas package: a loaded 1 x 1 image stands in (state import uses it).
  Object.defineProperty(HTMLImageElement.prototype, 'src', {
    configurable: true,
    set(this: HTMLImageElement) {
      Object.defineProperty(this, 'width', { value: 1 });
      Object.defineProperty(this, 'height', { value: 1 });
      queueMicrotask(() => this.onload?.(new Event('load')));
    },
  });
  (window as { createImageBitmap?: unknown }).createImageBitmap = () => Promise.resolve({ width: 1, height: 1, close: () => undefined });
});

afterAll(() => {
  HTMLCanvasElement.prototype.getContext = original.getContext;
  HTMLCanvasElement.prototype.toDataURL = original.toDataURL;
  if (original.imageSrc) Object.defineProperty(HTMLImageElement.prototype, 'src', original.imageSrc);
  (window as { createImageBitmap?: unknown }).createImageBitmap = original.createImageBitmap;
});

async function load(text: string): Promise<Guacamole.SessionRecording> {
  const recording = new Guacamole.SessionRecording(new Blob([text]));
  await new Promise<void>((resolve, reject) => {
    recording.onload = resolve;
    recording.onerror = (message) => {
      reject(new Error(message));
    };
  });
  return recording;
}

/** Seeks and then waits until the display has drawn everything the seek replayed (image decoding is async). */
async function seek(recording: Guacamole.SessionRecording, position: number): Promise<void> {
  await new Promise<void>((resolve) => {
    recording.seek(position, resolve);
  });
  await new Promise<void>((resolve) => {
    recording.getDisplay().flush(resolve);
  });
}

/** Yields to the real event loop until `done`; the fake clock in the play test does not run FileReader. */
async function until(done: () => boolean): Promise<void> {
  for (let turn = 0; turn < 1000 && !done(); turn++) await new Promise((resolve) => setImmediate(resolve));
  expect(done()).toBe(true);
}

/** What the default layer's canvas last filled with: the colour of the last frame drawn on it. */
function lastFill(recording: Guacamole.SessionRecording): string | undefined {
  const canvas = recording.getDisplay().getElement().querySelector('canvas');
  return canvas ? contexts.get(canvas)?.fills.at(-1) : undefined;
}

/** The last operation drawn on the default layer's canvas. */
function lastOp(recording: Guacamole.SessionRecording): string | undefined {
  const canvas = recording.getDisplay().getElement().querySelector('canvas');
  return canvas ? contexts.get(canvas)?.ops.at(-1) : undefined;
}

describe('SessionRecording loads the stream the hub tees (RA-041, K4 §11)', () => {
  it('the fixtures make a stream of one opening frame and six later frames, with the ready removed', () => {
    expect(opening.startsWith('5.ready,')).toBe(true);
    expect(stream).not.toContain('5.ready,');
    expect(syncs).toHaveLength(7);
    expect(fills).toHaveLength(6);
    expect(new Set(fills).size).toBe(6);
  });

  it('K4 §11: loads the whole stream and the duration is the last sync timestamp minus the first', async () => {
    const recording = await load(stream);
    expect(recording.getDuration()).toBe(at(6));
    expect(at(6)).toBeGreaterThan(0);
    expect(recording.getPosition()).toBe(0);
  });

  it('K4 §11: onprogress reports each frame as it is parsed, ending at the duration', async () => {
    const recording = new Guacamole.SessionRecording(new Blob([stream]));
    const durations: number[] = [];
    recording.onprogress = (duration) => durations.push(duration);
    await new Promise<void>((resolve) => {
      recording.onload = resolve;
    });
    expect(durations).toEqual(syncs.map((_, frame) => at(frame)));
  });
});

describe('SessionRecording seeks and plays the stream the hub tees (RA-041, K4 §11)', () => {
  it('K4 §11: seek forward lands on the frame at that time, sized by `size` and drawn with that frame', async () => {
    const recording = await load(stream);
    const reached: number[] = [];
    recording.onseek = (position) => reached.push(position);

    await seek(recording, at(3));

    expect(recording.getPosition()).toBe(at(3));
    expect(reached.at(-1)).toBe(at(3));
    expect(recording.getDisplay().getWidth()).toBe(64);
    expect(recording.getDisplay().getHeight()).toBe(48);
    // Frames 1 to 6 carry the fills; frame 3 is the third of them (index 2).
    expect(lastFill(recording)).toBe(fills[2]);
  });

  it('K4 §11: a time just after a frame lands on that frame', async () => {
    const recording = await load(stream);
    await seek(recording, at(4) + 1);
    expect(recording.getPosition()).toBe(at(4));
    expect(lastFill(recording)).toBe(fills[3]);
  });

  it('K4 §11: seek backwards lands on the earlier frame and shows its draw, not the later one', async () => {
    const recording = await load(stream);
    await seek(recording, at(5));
    expect(lastFill(recording)).toBe(fills[4]);

    await seek(recording, at(2));

    expect(recording.getPosition()).toBe(at(2));
    expect(recording.getDisplay().getWidth()).toBe(64);
    expect(recording.getDisplay().getHeight()).toBe(48);
    expect(lastFill(recording)).toBe(fills[1]);
  });

  it('K4 §11: seek to zero returns to the opening frame', async () => {
    const recording = await load(stream);
    await seek(recording, at(4));
    expect(lastOp(recording)).toBe(`fill ${fills[3] ?? ''}`);
    await seek(recording, 0);
    expect(recording.getPosition()).toBe(0);
    expect(recording.getDisplay().getWidth()).toBe(64);
    expect(recording.getDisplay().getHeight()).toBe(48);
    // The state restored from the opening is drawn over frame 4's fill; the opening has no fill of its own.
    expect(lastOp(recording)).toBe('drawImage');
  });

  it('K4 §11: seeking past the end clamps to the duration and the last frame', async () => {
    const recording = await load(stream);
    await seek(recording, at(6) + 60_000);
    expect(recording.getPosition()).toBe(recording.getDuration());
    expect(lastFill(recording)).toBe(fills[5]);
  });
});

describe('SessionRecording plays the stream the hub tees (RA-041, K4 §11)', () => {
  it('K4 §11: play advances in real time, waits out the gap between the captures, then pauses at the end', async () => {
    const recording = await load(stream);
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout', 'Date'] });
    try {
      const played: string[] = [];
      recording.onplay = () => played.push('play');
      recording.onpause = () => played.push('pause');

      recording.play();
      expect(recording.isPlaying()).toBe(true);
      // Let frame 0 replay and frame 1 be scheduled; a paced player must not move on while the fake clock stands.
      await until(() => vi.getTimerCount() > 0);
      for (let turn = 0; turn < 50; turn++) await new Promise((resolve) => setImmediate(resolve));
      expect(recording.getPosition()).toBe(0);
      expect(lastFill(recording)).toBeUndefined();

      // No real yield between the advance and these checks: the gap has not fully passed.
      await vi.advanceTimersByTimeAsync(at(1) - 1);
      expect(recording.getPosition()).toBe(0);
      expect(recording.isPlaying()).toBe(true);
      expect(lastFill(recording)).toBeUndefined();

      // The captures are not contiguous: frame 1 comes `at(1)` ms after frame 0 and playback waits that long.
      await vi.advanceTimersByTimeAsync(1);
      await until(() => recording.getPosition() === at(1));
      expect(lastFill(recording)).toBe(fills[0]);

      await vi.advanceTimersByTimeAsync(at(2) - at(1));
      await until(() => recording.getPosition() === at(2));

      await vi.advanceTimersByTimeAsync(at(6) - at(2));
      await until(() => !recording.isPlaying());
      expect(recording.getPosition()).toBe(recording.getDuration());
      expect(recording.isPlaying()).toBe(false);
      expect(played).toEqual(['play', 'pause']);
      expect(lastFill(recording)).toBe(fills[5]);
    } finally {
      vi.useRealTimers();
    }
  });
});
