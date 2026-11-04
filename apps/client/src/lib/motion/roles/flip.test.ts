// @vitest-environment jsdom
import { beforeEach, expect, it, vi } from 'vitest';
import { runs } from '../engine.fixture';
import { spring } from '../tokens';
import { flip, measure } from './flip';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('../engine.fixture')).fakeRun,
}));

/** An element whose on-screen box is its layout box moved by its `translate`, as in a browser. */
function row(top: number) {
  const node = document.createElement('li');
  document.body.append(node);
  const layout = { top };
  node.getBoundingClientRect = () => {
    const [x = 0, y = 0] = node.style.translate.split(' ').map((part) => Number.parseFloat(part) || 0);
    return new DOMRect(x, layout.top + y, 200, 40);
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

  flip(first, { preference: { reduced: false } });

  expect(node.getBoundingClientRect().top).toBe(0);
  expect(runs.map((run) => run.keyframes)).toEqual([
    [0, 0],
    [-120, 0],
  ]);
  expect(runs[1]?.options).toMatchObject({ stiffness: spring.move.stiffness, damping: spring.move.damping });
});

it('MOT-002: a second change mid-flight starts from where the item is on screen', () => {
  const { node, layout } = row(0);
  const initial = measure([node]);
  layout.top = 120;
  flip(initial, { preference: { reduced: false } });
  runs.at(-1)?.frame(-50);
  expect(node.getBoundingClientRect().top).toBe(70);

  const midFlight = measure([node]);
  layout.top = 240;
  runs.length = 0;
  flip(midFlight, { preference: { reduced: false } });

  expect(node.getBoundingClientRect().top).toBe(70);
  expect(runs.at(-1)?.keyframes).toEqual([-170, 0]);
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
  const first = new Map([[node, new DOMRect(0, 0, 200, 20)]]);

  flip(first, { scale: true, preference: { reduced: false } });

  expect(runs.filter((run) => run.keyframes.at(-1) === 1).map((run) => run.keyframes[0])).toEqual([1, 0.5]);
});

it('an item removed from the document is skipped', () => {
  const { node } = row(0);
  const first = measure([node]);
  node.remove();
  flip(first, { preference: { reduced: false } });
  expect(runs).toEqual([]);
});
