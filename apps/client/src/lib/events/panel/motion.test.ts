// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { fakeHub } from '$lib/devices/hub.fixture';
import { endTransitionsAtOnce } from '$lib/devices/transitions.fixture';
import { pointer, stubPointerCapture } from '$lib/motion/drag/pointer.fixture';
import { runs } from '../../motion/engine.fixture';
import { duration, spring } from '../../motion/tokens';
import type { LogEvent } from '../api';
import LogsPanel from './LogsPanel.svelte';
import { NOW, event, eventsAnswer, settle } from './panel.fixture';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('../../motion/engine.fixture')).fakeRun,
}));

const preference = vi.hoisted(() => ({ reduced: false }));
vi.mock('$lib/motion/preference', () => ({ motionPreference: () => preference }));

const current = vi.hoisted(() => ({ hub: undefined as unknown }));
vi.mock('$lib/hub/context', () => ({ useHub: () => current.hub }));

endTransitionsAtOnce();

// jsdom's window is 768 px high and its header 0 px: the stops are 0, 253.44 and 506.88.
const THIRD_PX = 768 * 0.33;
const TWO_THIRDS_PX = 768 * 0.66;
const ROW = 20;
let app: ReturnType<typeof mount> | undefined;
let fake: ReturnType<typeof fakeHub>;

async function show(): Promise<void> {
  fake = fakeHub({ Events: eventsAnswer([event('e2', 1), event('e1', 2), event('e0', 3)]) });
  current.hub = fake.hub;
  app = mount(LogsPanel, { target: document.body, props: { devices: [], now: () => NOW } });
  await settle();
}

beforeEach(() => {
  runs.length = 0;
  preference.reduced = false;
  stubPointerCapture();
});

afterEach(() => {
  if (app) void unmount(app);
  app = undefined;
  document.body.innerHTML = '';
  vi.restoreAllMocks();
  vi.useRealTimers();
});

const edge = () => document.querySelector<HTMLElement>('[role="separator"]');
const panel = () => document.querySelector<HTMLElement>('section');
const content = () => document.querySelector<HTMLElement>('.content');
const valueNow = () => Number(edge()?.getAttribute('aria-valuenow'));
const button = (text: string) => [...document.querySelectorAll('button')].find((node) => node.textContent.includes(text));

/** Sends pointer events and lets the DOM catch up. */
function send(...events: PointerEvent[]): void {
  for (const sent of events) edge()?.dispatchEvent(sent);
  flushSync();
}

function press(key: string): void {
  edge()?.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true }));
  flushSync();
}

/** Rows stand `ROW` px apart in document order; the edge's top is `edgeTop`. */
function layOut(edgeTop = 0): void {
  vi.spyOn(Element.prototype, 'getBoundingClientRect').mockImplementation(function (this: Element) {
    const top = this.classList.contains('row')
      ? [...document.querySelectorAll('.row')].indexOf(this) * ROW
      : this.getAttribute('role') === 'separator'
        ? edgeTop
        : 0;
    return { left: 0, right: 0, top, bottom: top + ROW, width: 0, height: ROW, x: 0, y: top, toJSON: () => ({}) };
  });
}

async function arrive(over: LogEvent): Promise<void> {
  fake.push('EventAdded', { eventAdded: over });
  await settle();
}

const springsMoving = () => runs.filter((run) => run.options.stiffness === spring.move.stiffness);
const highlights = () => runs.filter((run) => run.keyframes[0] === 1 && run.keyframes[1] === 0);

describe('New log row', () => {
  it('moves the rows below down by FLIP on spring-move and fades a highlight over duration-highlight', async () => {
    await show();
    layOut();
    expect(runs).toEqual([]);
    await arrive(event('e3', 0));
    // The three rows that were there start one row higher; the new row starts one row above its place.
    const slides = springsMoving().filter((run) => run.keyframes[0] === -ROW && run.keyframes[1] === 0);
    expect(slides).toHaveLength(4);
    expect(springsMoving()[0]?.options).toMatchObject({ damping: spring.move.damping });
    expect(highlights()).toHaveLength(1);
    expect(highlights()[0]?.options).toMatchObject({ duration: duration.highlight / 1000 });
  });

  it('MOT-040: with reduced motion the rows change at once and the highlight remains', async () => {
    preference.reduced = true;
    await show();
    layOut();
    await arrive(event('e3', 0));
    expect(springsMoving()).toEqual([]);
    expect(document.querySelector('.row .message')?.textContent).toBe('message e3');
    expect(highlights()).toHaveLength(1);
    expect(highlights()[0]?.options).toMatchObject({ duration: duration.highlight / 1000 });
  });

  it('does not animate the rows of a first load', async () => {
    await show();
    expect(runs).toEqual([]);
  });
});

describe('Disclosure', () => {
  it('collapses on spring-default with the content fading over duration-base', async () => {
    await show();
    button('Collapse')?.click();
    flushSync();
    const slide = runs.find((run) => run.options.stiffness === spring.default.stiffness);
    expect(slide?.keyframes).toEqual([THIRD_PX, TWO_THIRDS_PX]);
    const fade = runs.find((run) => run.options.duration === duration.base / 1000);
    expect(fade?.keyframes).toEqual([1, 0]);
    expect(button('Expand')?.getAttribute('aria-expanded')).toBe('false');
    expect(content()?.inert).toBe(true);
  });

  it('MOT-040: with reduced motion the height changes at once and the content still fades over duration-base', async () => {
    preference.reduced = true;
    await show();
    button('Collapse')?.click();
    flushSync();
    expect(runs.filter((run) => run.options.type === 'spring')).toEqual([]);
    expect(panel()?.style.translate).toBe(`0px ${String(TWO_THIRDS_PX)}px`);
    expect(runs.find((run) => run.options.duration === duration.base / 1000)?.keyframes).toEqual([1, 0]);
  });

  it('expands to where it was and shows the content again', async () => {
    await show();
    button('Collapse')?.click();
    flushSync();
    button('Expand')?.click();
    flushSync();
    expect(valueNow()).toBe(Math.round(THIRD_PX));
    expect(content()?.inert).toBe(false);
  });
});

describe('Resize separator', () => {
  it('is a horizontal separator with a label and a value between its limits', async () => {
    await show();
    expect(edge()?.getAttribute('aria-orientation')).toBe('horizontal');
    expect(edge()?.getAttribute('aria-label')).toBe('Resize logs panel');
    expect(edge()?.getAttribute('aria-valuemin')).toBe('0');
    expect(edge()?.getAttribute('aria-valuemax')).toBe(String(TWO_THIRDS_PX));
    expect(valueNow()).toBe(Math.round(THIRD_PX));
  });

  it('MOT-030: the keyboard reaches all three stops and aria-valuenow follows', async () => {
    await show();
    press('ArrowUp');
    expect(valueNow()).toBe(Math.round(TWO_THIRDS_PX));
    press('ArrowDown');
    press('ArrowDown');
    expect(valueNow()).toBe(0);
    press('End');
    expect(valueNow()).toBe(Math.round(TWO_THIRDS_PX));
    press('Home');
    expect(valueNow()).toBe(0);
  });

  it('MOT-030: the panel follows the pointer 1:1 from the grab point', async () => {
    await show();
    layOut(500);
    send(pointer('pointerdown', 0, 510));
    send(pointer('pointermove', 16, 410));
    expect(valueNow()).toBe(Math.round(THIRD_PX + 100));
    expect(panel()?.style.translate).toBe(`0px ${String(TWO_THIRDS_PX - THIRD_PX - 100)}px`);
  });

  it('MOT-032: a fast downward flick released near 33 % closes the panel and the spring starts with its velocity', async () => {
    await show();
    layOut(500);
    send(pointer('pointerdown', 0, 510));
    send(pointer('pointermove', 90, 560));
    send(pointer('pointermove', 100, 600));
    expect(valueNow()).toBe(Math.round(THIRD_PX - 90));
    send(pointer('pointerup', 100, 600));
    flushSync();
    expect(valueNow()).toBe(0);
    const snap = runs.find((run) => run.options.stiffness === spring.snappy.stiffness);
    expect(snap?.options.velocity).toBeCloseTo(900, 6);
  });

  it('MOT-032: a slow release near 33 % stays at 33 %', async () => {
    await show();
    layOut(500);
    send(pointer('pointerdown', 0, 510));
    send(pointer('pointermove', 400, 560));
    send(pointer('pointerup', 800, 560));
    flushSync();
    expect(valueNow()).toBe(Math.round(THIRD_PX));
  });

  it('MOT-031: a cancelled pointer settles the panel like a release', async () => {
    await show();
    layOut(500);
    send(pointer('pointerdown', 0, 510));
    send(pointer('pointermove', 400, 410));
    send(pointer('pointercancel', 800, 0));
    flushSync();
    expect(valueNow()).toBe(Math.round(THIRD_PX));
  });

  it('MOT-033: dragging past 66 % shows less than the pointer moved', async () => {
    await show();
    layOut(500);
    send(pointer('pointerdown', 0, 510));
    send(pointer('pointermove', 16, 110));
    expect(valueNow()).toBeGreaterThan(TWO_THIRDS_PX);
    expect(valueNow()).toBeLessThan(THIRD_PX + 400);
  });
});

describe('MOT-042: no animation timers', () => {
  it('a new row, a drag, a key and the disclosure start no timer', async () => {
    vi.useFakeTimers();
    await show();
    layOut(500);
    await arrive(event('e3', 0));
    send(pointer('pointerdown', 0, 510));
    send(pointer('pointermove', 16, 410));
    send(pointer('pointerup', 32, 410));
    press('Home');
    button('Expand')?.click();
    flushSync();
    expect(vi.getTimerCount()).toBe(0);
  });
});
