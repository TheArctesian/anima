// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';
import { endTransitionsAtOnce } from '../transitions.fixture';
import { answer, fakeCalls, settle } from './calls.fixture';
import ScreenSharingButton from './ScreenSharingButton.svelte';

const placed = vi.hoisted(() => [] as string[]);
vi.mock('$lib/devices/actions/placement', () => ({
  placeNear: () => (panel: HTMLElement) => {
    placed.push(panel.textContent.trim());
  },
}));
vi.mock('$lib/motion/preference', () => ({ motionPreference: () => ({ reduced: false }) }));

endTransitionsAtOnce();

let app: ReturnType<typeof mount> | undefined;

afterEach(async () => {
  if (app) await unmount(app);
  app = undefined;
  document.body.replaceChildren();
  placed.length = 0;
});

it('R-079: the panel is placed again when its text changes, so a long failure stays inside the window', async () => {
  const { calls } = fakeCalls();
  calls.thisMachine.mockResolvedValue(null);
  calls.test.mockResolvedValue(answer(false));
  app = mount(ScreenSharingButton, {
    target: document.body,
    props: { device: { id: 'd', name: 'Nas', host: '100.64.0.9', os: 'LINUX', vnc: { enabled: false } }, calls, onReady: vi.fn() },
  });
  flushSync();
  document.querySelector('button')?.click();
  for (let turn = 0; turn < 5; turn += 1) await settle();
  flushSync();
  expect(placed.some((text) => text.startsWith('Checking port 5900'))).toBe(true);
  expect(placed.at(-1)).toContain('did not answer on port 5900');
});
