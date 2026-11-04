// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { runs } from '../motion/engine.fixture';
import { spring } from '../motion/tokens';
import { shown } from './bar.fixture.svelte';
import HeartbeatBar from './HeartbeatBar.svelte';
import { BAR_LENGTH, type Beat } from './heartbeats';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('../motion/engine.fixture')).fakeRun,
}));

const preference = vi.hoisted(() => ({ reduced: false }));
vi.mock('$lib/motion/preference', () => ({ motionPreference: () => preference }));

const beat = (minute: number, status: Beat['status'] = 'UP'): Beat => ({
  checkId: 'c1',
  at: new Date(Date.parse('2026-11-04T10:00:00Z') + minute * 60_000).toISOString(),
  status,
});
let app: ReturnType<typeof mount> | undefined;

function show(beats: readonly Beat[]): HTMLElement {
  shown.beats = beats;
  app = mount(HeartbeatBar, {
    target: document.body,
    props: {
      get beats() {
        return shown.beats;
      },
    },
  });
  flushSync();
  const bar = document.querySelector<HTMLElement>('[role="img"]');
  if (bar === null) throw new Error('no bar');
  return bar;
}

function add(next: Beat): void {
  shown.beats = [...shown.beats, next];
  flushSync();
}

const cells = () => [...document.querySelectorAll('.cell')];

beforeEach(() => {
  runs.length = 0;
  preference.reduced = false;
});

afterEach(() => {
  if (app) void unmount(app);
  app = undefined;
  document.body.innerHTML = '';
});

it('DEV-002: the bar has 50 cells whatever the number of beats', () => {
  show([beat(1)]);
  expect(cells()).toHaveLength(BAR_LENGTH);
});

it('DEV-002: empty cells sit on the left, the newest beat on the right', () => {
  show([beat(1, 'DOWN'), beat(2, 'UP')]);
  const classes = cells().map((cell) => cell.classList[1]);
  expect(classes.slice(0, 48).every((name) => name === 'empty')).toBe(true);
  expect(classes.slice(-2)).toEqual(['down', 'up']);
});

it.each(['UP', 'DOWN', 'DEGRADED', 'MAINTENANCE', 'UNKNOWN'] as const)('DEV-002: a %s beat has its status class', (status) => {
  show([beat(1, status)]);
  expect(cells().at(-1)?.classList.contains(status.toLowerCase())).toBe(true);
});

it('DEV-002: the bar is one image with a text summary and hidden cells', () => {
  const bar = show([beat(1), beat(2, 'DEGRADED')]);
  expect(bar.getAttribute('aria-label')).toBe('2 heartbeats, latest degraded');
  expect(document.querySelector('.row')?.getAttribute('aria-hidden')).toBe('true');
});

it('DEV-002: the first beats show without motion', () => {
  show([beat(1)]);
  expect(runs).toEqual([]);
});

it('MOT-002: a new beat moves the row left with spring-default', () => {
  show([beat(1)]);
  add(beat(2));
  expect(runs).toHaveLength(1);
  expect(runs[0]?.options).toMatchObject({ type: 'spring', stiffness: spring.default.stiffness, damping: spring.default.damping });
  expect(runs[0]?.keyframes.at(-1)).toBe(0);
});

it('MOT-002: a second beat mid-motion interrupts the first and springs again', () => {
  show([beat(1)]);
  add(beat(2));
  add(beat(3));
  expect(runs).toHaveLength(2);
  expect(runs[0]?.stops).toBe(1);
});

it('MOT-040: with reduced motion a new beat is instant', () => {
  preference.reduced = true;
  show([beat(1)]);
  add(beat(2));
  expect(runs).toEqual([]);
  expect(cells().at(-1)?.classList.contains('up')).toBe(true);
});
