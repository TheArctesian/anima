// @vitest-environment jsdom
import { beforeEach, expect, it, vi } from 'vitest';
import { runs } from '../engine.fixture';
import { duration, spring } from '../tokens';
import { press } from './press';
import { roleScale } from './scales';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('../engine.fixture')).fakeRun,
}));

function pressable(reduced: boolean) {
  const button = document.createElement('button');
  document.body.append(button);
  const detach = press({ reduced })(button);
  return { button, detach };
}

function pointer(button: HTMLElement, type: string): void {
  button.dispatchEvent(new PointerEvent(type, { bubbles: true, button: 0 }));
}

beforeEach(() => {
  runs.length = 0;
});

it('MOT-001: the press starts on pointer-down, before any click', () => {
  const { button } = pressable(false);
  pointer(button, 'pointerdown');
  expect(runs.map((run) => run.keyframes.at(-1))).toEqual([roleScale.press, roleScale.press]);
  expect(runs[0]?.options).toMatchObject({ duration: duration.press / 1000 });
});

it('MOT-001: a click alone does not press', () => {
  const { button } = pressable(false);
  button.click();
  expect(runs).toEqual([]);
});

const releases = ['pointerup', 'pointercancel', 'pointerleave'];

it.each(releases)('5.7.3: %s springs the scale back to 1 with spring-default', (release) => {
  const { button } = pressable(false);
  pointer(button, 'pointerdown');
  runs.length = 0;
  pointer(button, release);
  expect(runs.map((run) => run.keyframes.at(-1))).toEqual([1, 1]);
  expect(runs[0]?.options).toMatchObject({ type: 'spring', stiffness: spring.default.stiffness });
});

it('MOT-040: under reduced motion the press does not scale', () => {
  const { button } = pressable(true);
  pointer(button, 'pointerdown');
  pointer(button, 'pointerup');
  expect(runs).toEqual([]);
  expect(button.style.scale).toBe('');
});

it('MOT-001: only the primary button presses', () => {
  const { button } = pressable(false);
  button.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true, button: 2 }));
  expect(runs).toEqual([]);
});

it('a pointer leaving without a press does nothing', () => {
  const { button } = pressable(false);
  pointer(button, 'pointerleave');
  expect(runs).toEqual([]);
  expect(button.style.scale).toBe('');
});

it('a detached press no longer reacts', () => {
  const { button, detach } = pressable(false);
  if (typeof detach === 'function') detach();
  pointer(button, 'pointerdown');
  expect(runs).toEqual([]);
});
