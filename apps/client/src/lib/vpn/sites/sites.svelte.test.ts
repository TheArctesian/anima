// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';
import { installAnimations, played } from '$lib/forms/animations.fixture';
import { picture } from '../vpn.fixture';
import SitePaths from './SitePaths.svelte';

vi.hoisted(() => {
  // The window's preference is read once, so it must be set before the motion module loads (MOT-041).
  vi.stubGlobal('matchMedia', (query: string) => ({
    matches: query.includes('reduce'),
    media: query,
    addEventListener: () => undefined,
    removeEventListener: () => undefined,
  }));
});

let app: ReturnType<typeof mount> | undefined;

afterEach(() => {
  if (app) void unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

it('MOT-040: under reduced motion a site row fades in and does not scale or move', () => {
  installAnimations();
  played.length = 0;
  const state = $state({ sites: [] as ReturnType<typeof picture>['sites'] });
  app = mount(SitePaths, {
    target: document.body,
    props: {
      broker: 'READY',
      onEnrol: () => undefined,
      reasonId: 'why',
      get sites() {
        return state.sites;
      },
    },
  });
  flushSync();
  state.sites = picture().sites;
  flushSync();

  expect(document.querySelector('ul[aria-label="Sites"] > li[aria-label="Home"]')).not.toBeNull();
  const keyframes = played.flatMap((call) => call.keyframes.map((frame) => Object.keys(frame))).flat();
  expect(keyframes).toContain('opacity');
  expect(keyframes).not.toContain('scale');
  expect(keyframes).not.toContain('transform');
});
