// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';
import { fakeHub } from '../../devices/hub.fixture';
import LiveSessions from './LiveSessions.svelte';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('../../motion/engine.fixture')).fakeRun,
}));

/** The keyframes each transition asked the browser to play. */
const played: Keyframe[][] = [];

function reducedMotion(): void {
  vi.stubGlobal('matchMedia', (query: string) => ({
    matches: query.includes('reduce'),
    addEventListener: () => undefined,
    removeEventListener: () => undefined,
  }));
  Object.defineProperty(Element.prototype, 'animate', {
    configurable: true,
    value: function animate(frames: Keyframe[]) {
      played.push(frames);
      const playback = { onfinish: null as (() => void) | null, cancel: () => undefined, finish: () => undefined, currentTime: 0 };
      queueMicrotask(() => playback.onfinish?.());
      return playback;
    },
  });
}

const animate = Element.prototype.animate;
let app: ReturnType<typeof mount> | undefined;

afterEach(async () => {
  if (app) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
  vi.unstubAllGlobals();
  Object.defineProperty(Element.prototype, 'animate', { configurable: true, value: animate });
  played.length = 0;
});

it('MOT-040, MOT-041: under reduced motion the popover opens and closes with opacity only, no scale', async () => {
  reducedMotion();
  const fake = fakeHub({ LiveSessions: { sessions: { edges: [] } } });
  app = mount(LiveSessions, { target: document.body, props: { hub: fake.hub } });
  flushSync();
  const control = document.querySelector<HTMLButtonElement>('.control') as HTMLButtonElement;
  control.click();
  flushSync();
  await vi.waitFor(() => {
    expect(played.length).toBeGreaterThan(0);
  });
  control.click();
  flushSync();
  await vi.waitFor(() => {
    expect(document.querySelector('.panel')).toBeNull();
  });
  const properties = new Set(played.flat().flatMap((frame) => Object.keys(frame)));
  expect(properties.has('opacity')).toBe(true);
  expect([...properties].filter((name) => /scale|transform/.test(name))).toEqual([]);
});
