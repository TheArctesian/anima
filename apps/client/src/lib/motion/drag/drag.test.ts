// @vitest-environment jsdom
import { afterEach, beforeEach, expect, it } from 'vitest';
import { drag, type Dragged, type Released } from './drag';
import { pointer, stubPointerCapture } from './pointer.fixture';
import { velocityOf } from './velocity';

let handle: HTMLElement;
let stop: (() => void) | undefined;
let captured: number[];
const moves: Dragged[] = [];
const releases: Released[] = [];
const grabs: number[] = [];

function attach(axis: 'x' | 'y' = 'y', origin = 400): void {
  stop = drag({
    axis,
    origin: () => origin,
    onGrab: () => grabs.push(1),
    onMove: (dragged) => moves.push(dragged),
    onRelease: (released) => releases.push(released),
  })(handle) as (() => void) | undefined;
}

beforeEach(() => {
  captured = stubPointerCapture();
  handle = document.createElement('div');
  document.body.append(handle);
  moves.length = 0;
  releases.length = 0;
  grabs.length = 0;
});

afterEach(() => {
  stop?.();
  stop = undefined;
  handle.remove();
});

it('MOT-030: the item keeps the offset from the grab point while it follows the pointer 1:1', () => {
  attach('y', 400);
  handle.dispatchEvent(pointer('pointerdown', 0, 410));
  handle.dispatchEvent(pointer('pointermove', 16, 300));
  expect(moves).toEqual([{ position: 290, delta: -110 }]);
  expect(grabs).toHaveLength(1);
});

it('MOT-030: the pointer is captured at the grab', () => {
  attach();
  handle.dispatchEvent(pointer('pointerdown', 0, 410));
  expect(captured).toEqual([1]);
});

it('MOT-030: on the x axis the item follows clientX', () => {
  attach('x', 50);
  handle.dispatchEvent(pointer('pointerdown', 0, 999, 60));
  handle.dispatchEvent(pointer('pointermove', 16, 0, 90));
  expect(moves).toEqual([{ position: 80, delta: 30 }]);
});

it('MOT-031: release reports the pointer velocity in px/s from the event timestamps', () => {
  attach();
  handle.dispatchEvent(pointer('pointerdown', 0, 400));
  handle.dispatchEvent(pointer('pointermove', 50, 450));
  handle.dispatchEvent(pointer('pointerup', 100, 500));
  expect(releases).toEqual([{ position: 500, delta: 100, velocity: 1000 }]);
});

it('MOT-031: a pointer that rested before release leaves with velocity 0', () => {
  attach();
  handle.dispatchEvent(pointer('pointerdown', 0, 400));
  handle.dispatchEvent(pointer('pointermove', 20, 600));
  handle.dispatchEvent(pointer('pointerup', 400, 600));
  expect(releases[0]?.velocity).toBe(0);
});

it('MOT-031: only the last 100 ms of movement counts', () => {
  expect(
    velocityOf(
      [
        { at: 0, value: 0 },
        { at: 400, value: 1000 },
        { at: 500, value: 1010 },
      ],
      500,
    ),
  ).toBe(100);
});

it('MOT-031: a cancelled pointer ends the drag like a release, at its last position', () => {
  attach();
  handle.dispatchEvent(pointer('pointerdown', 0, 400));
  handle.dispatchEvent(pointer('pointermove', 50, 350));
  handle.dispatchEvent(pointer('pointercancel', 80, 0));
  expect(releases).toEqual([{ position: 350, delta: -50, velocity: -1000 }]);
});

it('MOT-030: moves after the release are ignored and a new grab starts fresh', () => {
  attach();
  handle.dispatchEvent(pointer('pointerdown', 0, 400));
  handle.dispatchEvent(pointer('pointerup', 10, 400));
  handle.dispatchEvent(pointer('pointermove', 20, 100));
  expect(moves).toEqual([]);
  handle.dispatchEvent(pointer('pointerdown', 30, 405));
  handle.dispatchEvent(pointer('pointermove', 40, 305));
  expect(moves).toEqual([{ position: 300, delta: -100 }]);
});

it('MOT-030: after teardown the element no longer reacts', () => {
  attach();
  stop?.();
  stop = undefined;
  handle.dispatchEvent(pointer('pointerdown', 0, 400));
  expect(captured).toEqual([]);
});
