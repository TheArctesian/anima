// @vitest-environment jsdom
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { runs } from '../../motion/engine.fixture';
import { spring } from '../../motion/tokens';
import { PanelPlacement } from './placement.svelte';
import { snapsOf } from './resize';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('../../motion/engine.fixture')).fakeRun,
}));

const snaps = snapsOf(40, 900);
const preference = { reduced: false };
let panel: HTMLElement;
let placement: PanelPlacement;

beforeEach(() => {
  runs.length = 0;
  preference.reduced = false;
  panel = document.createElement('section');
  document.body.append(panel);
  placement = new PanelPlacement({ snaps: () => snaps, panel: () => panel, preference });
  placement.settle();
});

afterEach(() => {
  panel.remove();
});

const slid = () => panel.style.translate;

it('MOT-030: the panel starts at 33 % with its top edge that far up', () => {
  expect(placement.height).toBe(297);
  expect(slid()).toBe('0px 297px');
});

it('MOT-030: a drag moves the panel 1:1 with no spring', () => {
  placement.grab();
  placement.move(-100);
  expect(placement.height).toBe(397);
  expect(slid()).toBe('0px 197px');
  expect(runs).toEqual([]);
});

it('MOT-033: dragging past 66 % shows less than the pointer moved', () => {
  placement.grab();
  placement.move(-400);
  expect(placement.height).toBeGreaterThan(594);
  expect(placement.height).toBeLessThan(297 + 400);
});

it('MOT-031: a release springs with spring-momentum, starting with the pointer velocity', () => {
  placement.grab();
  placement.move(60);
  placement.release(-1200);
  expect(runs).toHaveLength(1);
  expect(runs[0]?.options).toMatchObject({
    type: 'spring',
    stiffness: spring.momentum.stiffness,
    damping: spring.momentum.damping,
    velocity: 1200,
  });
  expect(runs[0]?.keyframes.at(-1)).toBe(554);
});

it('MOT-032: the release stop comes from the projected height, not the release height', () => {
  placement.grab();
  placement.move(-10);
  placement.release(-1500);
  expect(placement.stop).toBe(0);
  expect(placement.height).toBe(40);
  expect(placement.open).toBe(false);
});

it('MOT-034: an upward flick from near collapsed opens the panel', () => {
  placement.press('Home');
  runs[0]?.finish();
  runs.length = 0;
  placement.grab();
  placement.move(-60);
  placement.release(500);
  expect(placement.stop).toBe(1);
});

it('MOT-030: with reduced motion a release moves at once', () => {
  preference.reduced = true;
  placement.grab();
  placement.move(10);
  placement.release(500);
  expect(runs).toEqual([]);
  expect(placement.stop).toBe(2);
  expect(slid()).toBe('0px 0px');
});

it('MOT-030: keys step through the three stops and report whether they were used', () => {
  expect(placement.press('ArrowUp')).toBe(true);
  expect(placement.height).toBe(594);
  expect(placement.press('ArrowDown')).toBe(true);
  expect(placement.press('ArrowDown')).toBe(true);
  expect(placement.height).toBe(40);
  expect(placement.press('End')).toBe(true);
  expect(placement.height).toBe(594);
  expect(placement.press('Home')).toBe(true);
  expect(placement.height).toBe(40);
  expect(placement.press('Tab')).toBe(false);
});

it('MOT-040: the disclosure collapses on spring-default and expands to where it was', () => {
  placement.press('End');
  runs.length = 0;
  placement.toggle();
  expect(placement.stop).toBe(0);
  expect(runs[0]?.options).toMatchObject({ stiffness: spring.default.stiffness, damping: spring.default.damping });
  placement.toggle();
  expect(placement.stop).toBe(2);
});

it('MOT-040: with reduced motion the disclosure changes height at once', () => {
  preference.reduced = true;
  placement.toggle();
  expect(runs).toEqual([]);
  expect(slid()).toBe('0px 554px');
});

it('MOT-030: a page that changes size in the middle of a drag keeps the dragged height between the new limits', () => {
  let current = snaps;
  placement = new PanelPlacement({ snaps: () => current, panel: () => panel, preference });
  placement.settle();
  placement.grab();
  placement.move(-200);
  expect(placement.height).toBe(497);
  current = snapsOf(40, 600);
  placement.settle();
  expect(placement.height).toBeGreaterThan(396);
  expect(placement.height).toBeLessThan(497);
  expect(slid()).toBe(`0px ${String(396 - placement.height)}px`);
  placement.release(0);
  expect(placement.height).toBe(396);
});

it('MOT-030: a page that changes size puts the panel at its stop at once', () => {
  const taller = snapsOf(40, 1200);
  placement = new PanelPlacement({ snaps: () => taller, panel: () => panel, preference });
  placement.settle();
  expect(placement.height).toBe(396);
  expect(runs).toEqual([]);
});

it('MOT-030: a grab in the middle of a spring takes hold where the panel is drawn, without a jump', () => {
  placement.press('End');
  runs[0]?.frame(300);
  expect(slid()).toBe('0px 300px');
  placement.grab();
  expect(placement.height).toBe(294);
  placement.move(-6);
  expect(placement.height).toBe(300);
  expect(slid()).toBe('0px 294px');
});
