// @vitest-environment jsdom
import { beforeEach, expect, it, vi } from 'vitest';
import { runs } from '../engine.fixture';
import { spring } from '../tokens';
import { flip, measure } from './flip';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('../engine.fixture')).fakeRun,
}));

const moving = { preference: { reduced: false } };

function pair(text: string, rest: number): [number, number] {
  const [x = rest, y = x] = text
    .split(' ')
    .filter((part) => part !== '')
    .map((part) => Number.parseFloat(part));
  return [x, y];
}

/** An element whose on-screen box is its layout box moved by `translate` and sized by `scale` about its centre. */
function row(top: number, left = 0) {
  const node = document.createElement('li');
  document.body.append(node);
  const layout = { top, left, width: 200, height: 40 };
  node.getBoundingClientRect = () => {
    const [x, y] = pair(node.style.translate, 0);
    const [, sy] = pair(node.style.scale, 1);
    const height = layout.height * sy;
    return new DOMRect(layout.left + x, layout.top + y + (layout.height - height) / 2, layout.width, height);
  };
  return { node, layout };
}

beforeEach(() => {
  runs.length = 0;
});

it('5.7.3 reorder: an item moves from its old place to its new one with spring-move', () => {
  const { node, layout } = row(0);
  const first = measure([node]);
  layout.top = 120;

  flip(first, moving);

  expect(node.getBoundingClientRect().top).toBe(0);
  expect(runs.map((run) => run.keyframes)).toEqual([
    [0, 0],
    [-120, 0],
  ]);
  expect(runs[1]?.options).toMatchObject({ stiffness: spring.move.stiffness, damping: spring.move.damping });
});

it('5.7.3 reorder: a sideways move is animated on the x axis', () => {
  const { node, layout } = row(0, 0);
  const first = measure([node]);
  layout.left = 300;

  flip(first, moving);

  expect(node.getBoundingClientRect().left).toBe(0);
  expect(runs[0]?.keyframes).toEqual([-300, 0]);
});

it('MOT-002: a second change mid-flight starts from where the item is on screen', () => {
  const { node, layout } = row(0);
  const initial = measure([node]);
  layout.top = 120;
  flip(initial, moving);
  runs.at(-1)?.frame(-50);
  expect(node.getBoundingClientRect().top).toBe(70);

  const midFlight = measure([node]);
  layout.top = 240;
  runs.length = 0;
  flip(midFlight, moving);

  expect(node.getBoundingClientRect().top).toBe(70);
  expect(runs.at(-1)?.keyframes).toEqual([-170, 0]);
});

it('MOT-002: after a stopped flip, a flip with no layout change keeps the item in place', () => {
  const { node, layout } = row(0);
  const initial = measure([node]);
  layout.top = 120;
  const playback = flip(initial, moving);
  runs.at(-1)?.frame(-50);
  playback.stop();

  runs.length = 0;
  flip(measure([node]), moving);

  expect(node.getBoundingClientRect().top).toBe(70);
  expect(runs.at(-1)?.keyframes).toEqual([-50, 0]);
});

it('MOT-040: under reduced motion the item is in its new place at once', () => {
  const { node, layout } = row(0);
  const first = measure([node]);
  layout.top = 120;

  flip(first, { preference: { reduced: true } });

  expect(runs).toEqual([]);
  expect(node.getBoundingClientRect().top).toBe(120);
});

it('disclosure: with scale the size change is animated from the old size', () => {
  const { node } = row(0);
  const first = new Map([[node, new DOMRect(0, 10, 200, 20)]]);

  flip(first, { ...moving, scale: true });

  expect(node.getBoundingClientRect().height).toBe(20);
  expect(runs.filter((run) => run.keyframes.at(-1) === 1).map((run) => run.keyframes[0])).toEqual([1, 0.5]);
});

it('disclosure: a scale left by an earlier flip is not counted as the new size', () => {
  const { node } = row(0);
  node.style.scale = '1 0.5';
  const first = measure([node]);

  flip(first, { ...moving, scale: true });

  expect(node.getBoundingClientRect().height).toBe(20);
  expect(runs.find((run) => run.keyframes[0] !== 1 && run.keyframes.at(-1) === 1)?.keyframes).toEqual([0.5, 1]);
});

it('an item removed from the document is skipped', () => {
  const { node } = row(0);
  const first = measure([node]);
  node.remove();
  flip(first, moving);
  expect(runs).toEqual([]);
});
