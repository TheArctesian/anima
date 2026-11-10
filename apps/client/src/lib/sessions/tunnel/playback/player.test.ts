// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { guacamoleStream } from '$lib/fixtures/phase3';
import { installBlankCanvas } from '../fake/canvas.fixture';
import Guacamole from '../vendor/guacamole.js';
import { FAKE_TERMINAL_SOCKET, FakeTerminalSocket } from '../terminal/fake/socket.fixture';
import { openRecordingLink } from './link';
import { openGuacamolePlayer, type GuacamolePlayer, type PlayerEvents } from './player';
import { openTranscript } from './transcript';

/** The K6 stream (one frame at 1000 ms) and two more synthetic frames, so a seek has somewhere to go. */
const LONGER = `${guacamoleStream.text}4.rect,1.0,1.0,1.0,2.10,2.10;5.cfill,2.14,1.0,1.0,1.0,1.0,3.255;4.sync,4.2500;4.size,1.0,3.800,3.600;4.sync,4.4000;`;

let restoreCanvas: () => void;

/**
 * An image that loads as soon as it has a source. jsdom never loads one, and a seek that restores a keyframe draws
 * its layers from data URLs; the display waits for them before `seek` resolves.
 */
class LoadingImage {
  onload: (() => void) | null = null;
  onerror: (() => void) | null = null;
  set src(_url: string) {
    queueMicrotask(() => this.onload?.());
  }
}

beforeEach(() => {
  restoreCanvas = installBlankCanvas();
  // Keyframes are stored as data URLs; jsdom has no encoder and prints "Not implemented".
  vi.spyOn(HTMLCanvasElement.prototype, 'toDataURL').mockReturnValue('data:,');
  vi.stubGlobal('Image', LoadingImage);
});

afterEach(() => {
  vi.useRealTimers();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  restoreCanvas();
});

/** Let the Blob reads that replay a frame finish; they wait on I/O, not on the (fake) clock. */
async function settled(): Promise<void> {
  for (let turn = 0; turn < 20; turn += 1) await new Promise((resolve) => setImmediate(resolve));
}

/** A player over `transcript`, once it has read all of it. */
function whenLoaded(transcript: Blob, events: PlayerEvents = {}): Promise<GuacamolePlayer> {
  return new Promise((resolve) => {
    const player: GuacamolePlayer = openGuacamolePlayer(transcript, {
      ...events,
      onloaded: () => {
        resolve(player);
      },
    });
  });
}

/** A player over `text` sent in frames of `frameBytes`, as the playback link delivers it, once it has loaded. */
function loaded(text: string, events: PlayerEvents = {}, frameBytes = 7): Promise<GuacamolePlayer> {
  const transcript = openTranscript();
  const bytes = new TextEncoder().encode(text);
  for (let at = 0; at < bytes.length; at += frameBytes) transcript.receive(bytes.subarray(at, at + frameBytes));
  return whenLoaded(transcript.blob(), events);
}

describe('the Guacamole player (RA-041, K4 §12)', () => {
  it('RA-041: the K6 Guacamole stream loads through the transcript; its display takes the recorded size and its length is its one frame', async () => {
    const progress: number[] = [];
    const player = await loaded(guacamoleStream.text, { onprogress: (duration) => progress.push(duration) });
    expect(progress).toEqual([0]);
    expect(player.duration()).toBe(0);
    await player.seek(0);
    expect(player.size()).toEqual({ width: 640, height: 480 });
    expect(player.element).toBeInstanceOf(HTMLElement);
    player.dispose();
  });

  it('RA-041, K4 §12: the K6 stream received over the playback link, in frames that cut its instructions, seeks to its frame', async () => {
    const transcript = openTranscript();
    const ended = new Promise<string | undefined>((resolve) => {
      openRecordingLink({
        origin: 'https://hub.anima.ing',
        ticket: { tunnelPath: '/recording', token: 'synthetic' },
        WebSocket: FAKE_TERMINAL_SOCKET,
        onbytes: (bytes) => transcript.receive(bytes),
        onend: (state) => {
          resolve(state.outcome);
        },
      });
    });
    const socket = FakeTerminalSocket.latest();
    socket.open();
    socket.control({ type: 'recording', format: guacamoleStream.format, sizeBytes: 512, complete: true });
    const bytes = new TextEncoder().encode(guacamoleStream.text);
    for (let at = 0; at < bytes.length; at += 10) socket.output(bytes.subarray(at, at + 10));
    socket.control({ type: 'done' });
    expect(await ended).toBe('done');
    const player = await whenLoaded(transcript.blob());
    await player.seek(0);
    expect(player.size()).toEqual({ width: 640, height: 480 });
    player.dispose();
  });

  it('RA-041: the K6 stream plays to its end and stops by itself', async () => {
    const playing: boolean[] = [];
    const player = await loaded(guacamoleStream.text, { onplaying: (now) => playing.push(now) });
    player.play();
    await vi.waitFor(() => {
      expect(playing).toEqual([true, false]);
    });
    expect(player.playing()).toBe(false);
    expect(player.size()).toEqual({ width: 640, height: 480 });
    player.dispose();
  });

});

describe('seeking (RA-041)', () => {
  it('RA-041: seek shows the frame at the time (or one next to it), forward and back, and reports the position', async () => {
    const positions: number[] = [];
    const sizes: [number, number][] = [];
    const player = await loaded(LONGER, { onposition: (at) => positions.push(at), onresize: (w, h) => sizes.push([w, h]) });
    expect(player.duration()).toBe(3000);
    await player.seek(3000);
    expect(player.position()).toBe(3000);
    expect(player.size()).toEqual({ width: 800, height: 600 });
    expect(positions.at(-1)).toBe(3000);
    await player.seek(1500);
    expect(player.position()).toBe(1500);
    await player.seek(2900);
    expect([1500, 3000]).toContain(player.position());
    await player.seek(0);
    expect(player.position()).toBe(0);
    expect(sizes).toContainEqual([800, 600]);
    player.dispose();
  });

  it('RA-041: a seek out of range is clamped; a seek before any frame and after dispose resolves at once', async () => {
    const player = await loaded(LONGER);
    await player.seek(Number.POSITIVE_INFINITY);
    expect(player.position()).toBe(0);
    await player.seek(99_999);
    expect(player.position()).toBe(3000);
    await player.seek(-5);
    expect(player.position()).toBe(0);
    player.dispose();
    await expect(player.seek(3000)).resolves.toBeUndefined();
    const empty = openGuacamolePlayer(new Blob([]));
    await expect(empty.seek(10)).resolves.toBeUndefined();
    empty.dispose();
  });

  it('RA-041: play during a seek still resolves the seek', async () => {
    const player = await loaded(LONGER);
    const seeking = player.seek(3000);
    player.play();
    await expect(seeking).resolves.toBeUndefined();
    player.dispose();
  });

  it('RA-041: pause during a seek still resolves the seek', async () => {
    const player = await loaded(LONGER);
    const seeking = player.seek(3000);
    player.pause();
    await expect(seeking).resolves.toBeUndefined();
    player.dispose();
  });
});

describe('a seek and the display (RA-041)', () => {
  it('RA-041: seek resolves only once the display has drawn the frame, not when 1.5.5 calls back', async () => {
    const vendor = Guacamole as unknown as { Display: new () => Guacamole.Display };
    const Original = vendor.Display;
    const held: (() => void)[] = [];
    let holding = false;
    const Held = function HeldDisplay() {
      const display = new Original();
      const flush = display.flush.bind(display);
      display.flush = (callback) => {
        flush(() => {
          if (holding && callback) held.push(callback);
          else callback?.();
        });
      };
      return display;
    };
    // The vendor's Display reads its layer classes from `Guacamole.Display` itself.
    vendor.Display = Object.assign(Held, Original) as unknown as typeof Original;
    try {
      const player = await loaded(LONGER);
      holding = true;
      let resolved = false;
      const seeking = player.seek(3000).then(() => {
        resolved = true;
      });
      await settled();
      expect(player.position()).toBe(3000);
      expect(resolved).toBe(false);
      holding = false;
      for (const callback of held.splice(0)) callback();
      await seeking;
      expect(resolved).toBe(true);
      player.dispose();
    } finally {
      vendor.Display = Original;
    }
  });
});

describe('playing (RA-041)', () => {
  it('RA-041: play shows the next frame at once, then each later one after its recorded gap', async () => {
    const player = await loaded(LONGER);
    await player.seek(0);
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout', 'Date'] });
    player.play();
    expect(player.playing()).toBe(true);
    await settled();
    expect(player.position()).toBe(1500);
    await vi.advanceTimersByTimeAsync(1499);
    await settled();
    expect(player.position()).toBe(1500);
    await vi.advanceTimersByTimeAsync(1);
    await settled();
    expect(player.position()).toBe(3000);
    expect(player.playing()).toBe(false);
    player.dispose();
  });

  it('RA-041: pause holds the position while the clock runs on', async () => {
    const player = await loaded(LONGER);
    await player.seek(0);
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout', 'Date'] });
    player.play();
    await settled();
    player.pause();
    expect(player.playing()).toBe(false);
    await vi.advanceTimersByTimeAsync(5000);
    await settled();
    expect(player.position()).toBe(1500);
    player.dispose();
  });

});

describe('the display and faults (MAT-008, LOG-007)', () => {
  it('MAT-008: scale sets a fixed size for the bounds, ignores a factor that is not positive, and puts no transition or animation on the display', async () => {
    const player = await loaded(LONGER);
    await player.seek(0);
    player.scale(0.5);
    player.scale(Number.NaN);
    player.scale(-1);
    player.scale(0);
    expect(player.element.style.width).toBe('320px');
    expect(player.element.style.height).toBe('240px');
    const styles = [player.element, ...player.element.querySelectorAll<HTMLElement>('*')].map((node) => node.style);
    expect(styles.flatMap((style) => [style.transition, style.animation]).filter((value) => value !== '')).toEqual([]);
    player.dispose();
  });

  it('LOG-007: text the 1.5.5 parser refuses calls onunreadable with no arguments, and nothing reaches the console', async () => {
    const methods = Object.keys(console).filter((key): key is keyof Console => typeof (console as unknown as Record<string, unknown>)[key] === 'function');
    const spies = methods.map((method) => vi.spyOn(console, method).mockImplementation(() => undefined));
    const heard: unknown[][] = [];
    const player = openGuacamolePlayer(new Blob(['4.sync,4.1000;x.SECRET-CONTENT;']), {
      onunreadable: (...args: unknown[]) => heard.push(args),
    });
    await vi.waitFor(() => {
      expect(heard).toEqual([[]]);
    });
    player.dispose();
    expect(spies.flatMap((spy) => spy.mock.calls)).toEqual([]);
  });
});
