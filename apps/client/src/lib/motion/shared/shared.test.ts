// @vitest-environment jsdom
import { beforeEach, expect, it, vi } from 'vitest';
import { runs } from '../engine.fixture';
import { spring } from '../tokens';
import { sharedItem } from './shared';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('../engine.fixture')).fakeRun,
}));

const moving = { preference: { reduced: false } };
const key = 'device-7-name';

function pair(text: string, rest: number): [number, number] {
  const [x = rest, y = x] = text
    .split(' ')
    .filter((part) => part !== '')
    .map((part) => Number.parseFloat(part));
  return [x, y];
}

interface Layout {
  top: number;
  left: number;
  width: number;
  height: number;
}

/** An element whose on-screen box is its layout box moved by `translate` and scaled about its centre. */
function box(layout: Layout): HTMLElement {
  const node = document.createElement('span');
  document.body.append(node);
  node.getBoundingClientRect = () => {
    const [x, y] = pair(node.style.translate, 0);
    const [sx, sy] = pair(node.style.scale, 1);
    const width = layout.width * sx;
    const height = layout.height * sy;
    return new DOMRect(
      layout.left + x + (layout.width - width) / 2,
      layout.top + y + (layout.height - height) / 2,
      width,
      height,
    );
  };
  return node;
}

/** The outgoing page's element: a card name still in the document while its page fades out. */
function card(): HTMLElement {
  const node = box({ top: 300, left: 40, width: 100, height: 20 });
  node.dataset.shared = key;
  return node;
}

/** The new page's element: the detail header name, at twice the card's size. */
function header(): HTMLElement {
  return box({ top: 20, left: 20, width: 200, height: 40 });
}

function attach(node: HTMLElement, options = moving): () => void {
  const detach = sharedItem(key, options)(node);
  if (typeof detach !== 'function') throw new Error('sharedItem must return a cleanup');
  return detach;
}

/** Lets the playback's promise chain run; no timer is involved (MOT-042). */
async function settled(): Promise<void> {
  for (let turn = 0; turn < 10; turn += 1) await Promise.resolve();
}

beforeEach(() => {
  document.body.replaceChildren();
  runs.length = 0;
});

it('card to detail: the element is marked with its shared key', () => {
  const node = header();
  attach(node);
  expect(node.dataset.shared).toBe(key);
});

it('card to detail: with no element on the outgoing page, nothing animates', () => {
  const node = header();
  attach(node);
  expect(runs).toEqual([]);
  expect(node.style.translate).toBe('');
  expect(node.style.scale).toBe('');
});

it('card to detail: an element with another key is not a counterpart', () => {
  const other = card();
  other.dataset.shared = 'device-8-name';
  attach(header());
  expect(runs).toEqual([]);
});

it('card to detail, MOT-006: the item starts on the card rect and springs to its place by translate and scale', () => {
  card();
  const node = header();

  attach(node);

  expect(node.getBoundingClientRect()).toEqual(new DOMRect(40, 300, 100, 20));
  expect(runs.map((run) => run.keyframes)).toEqual([
    [-30, 0],
    [270, 0],
    [0.5, 1],
    [0.5, 1],
  ]);
  for (const run of runs) {
    expect(run.options).toMatchObject({ type: 'spring', stiffness: spring.move.stiffness, damping: spring.move.damping });
  }
  expect(node.getAttribute('style')).toMatch(/^translate: [^;]+; scale: [^;]+;$/);
});

it('card to detail: the card is hidden during the flight and shown again when it lands', async () => {
  const old = card();
  old.style.opacity = '0.8';
  attach(header());

  expect(old.style.opacity).toBe('0');
  for (const run of runs) run.finish();
  await settled();

  expect(old.style.opacity).toBe('0.8');
});

it('card to detail: with several outgoing pages, the last one mounted is the counterpart', () => {
  const first = card();
  const second = box({ top: 100, left: 70, width: 100, height: 20 });
  second.dataset.shared = key;
  const node = header();

  attach(node);

  expect(node.getBoundingClientRect().top).toBe(100);
  expect(second.style.opacity).toBe('0');
  expect(first.style.opacity).toBe('');
});

it('card to detail: an element that left the document is not a counterpart', () => {
  const old = card();
  old.remove();
  attach(header());
  expect(runs).toEqual([]);
});

it('MOT-040: under reduced motion nothing moves and the card is not hidden', () => {
  const old = card();
  const node = header();

  attach(node, { preference: { reduced: true } });

  expect(runs).toEqual([]);
  expect(node.getBoundingClientRect()).toEqual(new DOMRect(20, 20, 200, 40));
  expect(old.style.opacity).toBe('');
});

it('card to detail: detaching mid-flight stops the flight and clears the styles it wrote', () => {
  const old = card();
  const node = header();
  const detach = attach(node);
  runs[1]?.frame(100);

  detach();

  expect(runs.every((run) => run.stops === 1)).toBe(true);
  expect(node.style.translate).toBe('');
  expect(node.style.scale).toBe('');
  expect(node.dataset.shared).toBeUndefined();
  expect(old.style.opacity).toBe('');
});

it('card to detail: detaching after no flight leaves the styles it did not write', () => {
  const node = header();
  node.style.translate = '4px 0px';
  node.style.scale = '0.9';

  attach(node)();

  expect(node.style.translate).toBe('4px 0px');
  expect(node.style.scale).toBe('0.9');
  expect(node.dataset.shared).toBeUndefined();
});

it('MOT-002: a page that mounts mid-flight starts from where the moving item is on screen', () => {
  card();
  const middle = header();
  attach(middle);
  runs[0]?.frame(-10);
  runs[1]?.frame(100);
  runs[2]?.frame(0.75);
  runs[3]?.frame(0.75);
  const onScreen = middle.getBoundingClientRect();
  runs.length = 0;

  const last = box({ top: 400, left: 20, width: 200, height: 40 });
  attach(last);

  expect(last.getBoundingClientRect()).toEqual(onScreen);
  expect(middle.style.opacity).toBe('0');
});
