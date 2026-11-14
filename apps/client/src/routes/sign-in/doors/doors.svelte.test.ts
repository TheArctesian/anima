// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import Doors from './Doors.svelte';

let frames: FrameRequestCallback[];
let painted: number;

beforeEach(() => {
  frames = [];
  painted = 0;
  vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) => frames.push(callback));
  vi.stubGlobal('cancelAnimationFrame', () => undefined);
  vi.stubGlobal(
    'ResizeObserver',
    class {
      observe = (): void => undefined;
      disconnect = (): void => undefined;
    },
  );
  // jsdom draws nothing, so a stand-in counts the points painted.
  vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue({
    clearRect: () => undefined,
    fillRect: () => {
      painted += 1;
    },
  } as unknown as CanvasRenderingContext2D);
});

afterEach(() => {
  delete document.documentElement.dataset['motion'];
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

function runFrame(time: number): void {
  frames.shift()?.(time);
}

it('D13, MOT-040: with reduced motion the doors are drawn once and ask for no more frames', () => {
  document.documentElement.dataset['motion'] = 'reduce';
  const view = mount(Doors, { target: document.body });
  flushSync();
  runFrame(0);
  expect(painted).toBeGreaterThan(0);
  expect(frames).toHaveLength(0);
  unmount(view);
});

it('D13, MOT-043: with full motion the doors keep drifting, one frame after another', () => {
  document.documentElement.dataset['motion'] = 'full';
  const view = mount(Doors, { target: document.body });
  flushSync();
  runFrame(0);
  runFrame(16);
  expect(frames).toHaveLength(1);
  unmount(view);
});
