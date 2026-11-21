// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import type { DeviceCheck } from '$lib/checks/api';
import { installAnimations, played } from '$lib/forms/animations.fixture';
import { check } from '../page.fixture.svelte';
import ChecksPanel from './ChecksPanel.svelte';

// How the rows of the Checks tab come and go (5.7.3, Reorder): a row fades where it stands. The reduced form of the
// fade is the role's own (`listRow`), tested with the role.

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));

const preference = vi.hoisted(() => ({ reduced: false }));
vi.mock('$lib/motion/preference', () => ({ motionPreference: () => preference }));

const shown = $state<{ checks: DeviceCheck[] }>({ checks: [] });
let app: ReturnType<typeof mount> | undefined;
const ok = () => Promise.resolve({ ok: true as const, value: undefined });

function show(checks: DeviceCheck[]): void {
  shown.checks = checks;
  app = mount(ChecksPanel, {
    target: document.body,
    props: {
      get checks() {
        return shown.checks;
      },
      history: null,
      now: () => new Date('2026-11-20T10:00:00Z'),
      onHistory: () => undefined,
      onCloseHistory: () => undefined,
      onPause: ok,
      onSave: ok,
      onDelete: ok,
    },
  });
  flushSync();
}

const rowOf = (name: string) => [...document.querySelectorAll('.rows > li')].find((row) => row.textContent.includes(name));

beforeEach(() => {
  installAnimations();
  played.length = 0;
  preference.reduced = false;
});

afterEach(() => {
  if (app) void unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

it('a deleted check fades out in its own row, and the row stays until the fade ends', () => {
  show([check({ id: 'a', name: 'ping' }), check({ id: 'b', name: 'web' })]);
  const leaving = rowOf('ping');
  played.length = 0;
  shown.checks = [check({ id: 'b', name: 'web' })];
  flushSync();
  expect(played.some((each) => each.element === leaving)).toBe(true);
  // The row stays in the list while it fades, and is gone when the fade ends.
  expect(leaving?.isConnected).toBe(true);
});

it('a new check fades in as a row of the list', () => {
  show([check({ id: 'b', name: 'web' })]);
  played.length = 0;
  shown.checks = [check({ id: 'a', name: 'ping' }), check({ id: 'b', name: 'web' })];
  flushSync();
  expect(played.some((each) => each.element === rowOf('ping'))).toBe(true);
});
