// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { runs } from '../motion/engine.fixture';
import { spring } from '../motion/tokens';
import { shown } from './bar.fixture.svelte';
import HeartbeatBar from './HeartbeatBar.svelte';
import { BAR_LENGTH, addBeat, type Beat } from './heartbeats';

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
  vi.restoreAllMocks();
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

const PITCH = 12;

/** Gives every cell a place `PITCH` pixels after the cell before it, as a laid-out page would. */
function layOutCells(): void {
  vi.spyOn(Element.prototype, 'getBoundingClientRect').mockImplementation(function (this: Element) {
    const left = this.classList.contains('cell') ? cells().indexOf(this) * PITCH : 0;
    return { left, right: left, top: 0, bottom: 0, width: 0, height: 0, x: left, y: 0, toJSON: () => ({}) };
  });
}

const row = () => document.querySelector<HTMLElement>('.row');

it('MOT-002: a new beat starts the row one cell to the right and springs to rest', () => {
  layOutCells();
  show([beat(1)]);
  add(beat(2));
  expect(row()?.style.translate).toBe(`${String(PITCH)}px 0px`);
  expect(runs[0]?.keyframes).toEqual([PITCH, 0]);
});

it('MOT-002: a beat mid-spring starts from where the row is drawn, one cell further', () => {
  layOutCells();
  show([beat(1)]);
  add(beat(2));
  runs[0]?.frame(5);
  expect(row()?.style.translate).toBe('5px 0px');
  add(beat(3));
  expect(runs[1]?.keyframes).toEqual([5 + PITCH, 0]);
});

it('MOT-002: a beat older than the newest, put in its place, does not move the bar', () => {
  show([beat(1), beat(3)]);
  shown.beats = addBeat(shown.beats, beat(2));
  flushSync();
  expect(cells().filter((cell) => cell.classList.contains('up'))).toHaveLength(3);
  expect(runs).toEqual([]);
});

it('MOT-002: a duplicate beat does not move the bar', () => {
  show([beat(1), beat(2)]);
  shown.beats = addBeat(shown.beats, beat(2));
  flushSync();
  shown.beats = [...shown.beats];
  flushSync();
  expect(runs).toEqual([]);
});

it('MOT-002: beats that load into an empty bar do not move it', () => {
  show([]);
  shown.beats = [beat(1), beat(2)];
  flushSync();
  expect(runs).toEqual([]);
});
