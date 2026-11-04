// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { runs } from '$lib/motion/engine.fixture';
import { rubberband } from '$lib/motion/gesture';
import { spring } from '$lib/motion/tokens';
import { dragTab, type DragOptions } from './drag';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));

const moving = { reduced: false };
const reduced = { reduced: true };

function box(left: number, top: number, width: number, height: number): DOMRect {
  return new DOMRect(left, top, width, height);
}

/** Strip-local translate of `node`, as the engine draws it. */
function shown(node: HTMLElement): [number, number] {
  const [x = 0, y = 0] = node.style.translate
    .split(' ')
    .filter((part) => part !== '')
    .map((part) => Number.parseFloat(part));
  return [x, y];
}

const WIDTH = 100;
const PITCH = 110;

interface Rig {
  readonly tabs: HTMLElement[];
  readonly options: DragOptions;
  readonly clock: { now: number };
  readonly order: () => string[];
  readonly dragOut: ReturnType<typeof vi.fn<() => Promise<boolean>>>;
}

/** Three 100 px tabs, 10 px apart, in a strip 0..400 × 0..40; a tab's box follows its DOM place and translate. */
function rig(overrides: Partial<DragOptions> = {}): Rig {
  const strip = document.createElement('div');
  document.body.append(strip);
  const tabs = ['a', 'b', 'c'].map((name) => {
    const node = document.createElement('div');
    node.dataset.name = name;
    node.getBoundingClientRect = () => {
      const [x, y] = shown(node);
      return box([...strip.children].indexOf(node) * PITCH + x, y, WIDTH, 36);
    };
    strip.append(node);
    return node;
  });
  const clock = { now: 0 };
  const dragOut = vi.fn(() => Promise.resolve(true));
  const options: DragOptions = {
    tabs: () => [...strip.children] as HTMLElement[],
    bounds: () => ({ left: 0, top: 0, right: 400, bottom: 40 }),
    canDragOut: () => false,
    drop: (from, to) => {
      const node = strip.children[from];
      if (node === undefined) return;
      const rest = [...strip.children].filter((child) => child !== node);
      strip.replaceChildren(...rest.slice(0, to), node, ...rest.slice(to));
    },
    dragOut,
    preference: moving,
    now: () => clock.now,
    ...overrides,
  };
  for (const node of tabs) dragTab(options)(node);
  const order = () => [...strip.children].map((child) => (child as HTMLElement).dataset.name ?? '');
  return { tabs, options, clock, order, dragOut };
}

function pointer(node: HTMLElement, type: string, x: number, y = 18): void {
  node.dispatchEvent(new PointerEvent(type, { bubbles: true, button: 0, pointerId: 7, clientX: x, clientY: y }));
}

/** Drags from `fromX` through `points` (x at time t, and y) and releases at the last one. */
function drag(node: HTMLElement, clock: { now: number }, fromX: number, points: [number, number, number?][]): void {
  clock.now = 0;
  pointer(node, 'pointerdown', fromX);
  for (const [x, t, y] of points) {
    clock.now = t;
    pointer(node, 'pointermove', x, y);
  }
  const [lastX = fromX, , lastY] = points.at(-1) ?? [];
  pointer(node, 'pointerup', lastX, lastY);
}

// jsdom has no pointer capture; these stand in for the element's capture methods.
const capture = {
  setPointerCapture: vi.fn<(id: number) => void>(),
  hasPointerCapture: vi.fn<(id: number) => boolean>(() => true),
  releasePointerCapture: vi.fn<(id: number) => void>(),
};

beforeEach(() => {
  runs.length = 0;
  document.body.innerHTML = '';
  for (const [name, stub] of Object.entries(capture)) {
    stub.mockClear();
    Object.defineProperty(Element.prototype, name, { value: stub, configurable: true });
  }
});

afterEach(() => {
  for (const name of Object.keys(capture)) Reflect.deleteProperty(Element.prototype, name);
});

it('MOT-030: the pointer is captured on press; the tab follows only after threshold-drag, with the grab offset kept', () => {
  const { tabs } = rig();
  const [a] = tabs as [HTMLElement];
  pointer(a, 'pointerdown', 50);
  expect(capture.setPointerCapture).toHaveBeenCalledExactlyOnceWith(7);
  pointer(a, 'pointermove', 55);
  expect(a.style.translate).toBe('');
  pointer(a, 'pointermove', 80, 23);
  expect(shown(a)).toEqual([30, rubberband(5, 36)]);
});

it('MOT-030 / RA-002: a fast first move that leaves the tab still starts the drag', () => {
  const { tabs } = rig({ canDragOut: () => true });
  const [a] = tabs as [HTMLElement];
  pointer(a, 'pointerdown', 50);
  // Captured, so the browser sends the move to the tab even though the pointer is 60 px below it.
  pointer(a, 'pointermove', 52, 78);
  expect(a.dataset.dragging).toBe('');
  expect(shown(a)).toEqual([2, 60]);
});

it('a press with no drag lets the capture go, so the click is the tab\'s', () => {
  const { tabs } = rig();
  const [a] = tabs as [HTMLElement];
  pointer(a, 'pointerdown', 50);
  pointer(a, 'pointerup', 51);
  expect(capture.releasePointerCapture).toHaveBeenCalledExactlyOnceWith(7);
  expect(a.dataset.dragging).toBeUndefined();
});

it('a lost capture mid-drag puts the tab back in its place', () => {
  const { tabs, order } = rig();
  const [a] = tabs as [HTMLElement];
  pointer(a, 'pointerdown', 50);
  pointer(a, 'pointermove', 170);
  pointer(a, 'lostpointercapture', 170);
  expect(order()).toEqual(['a', 'b', 'c']);
  expect(a.style.position).toBe('');
});

it('the dragged tab is drawn in a fixed layer at its exact place, and a neighbour holds the place', () => {
  const { tabs } = rig();
  const [a, b] = tabs as [HTMLElement, HTMLElement];
  pointer(a, 'pointerdown', 50);
  pointer(a, 'pointermove', 70);
  expect([a.style.position, a.style.left, a.style.top, a.style.width]).toEqual(['fixed', '0px', '0px', '100px']);
  expect(b.style.marginInlineStart).toBe(`${String(PITCH)}px`);
  expect(a.getBoundingClientRect().left).toBe(20);
  pointer(a, 'pointerup', 70);
  expect([a.style.position, b.style.marginInlineStart]).toEqual(['', '']);

});

it('the last tab dragged is held by the margin of the tab before it', () => {
  const { tabs } = rig();
  const [, b, c] = tabs as [HTMLElement, HTMLElement, HTMLElement];
  pointer(c, 'pointerdown', 250);
  pointer(c, 'pointermove', 230);
  expect(b.style.marginInlineEnd).toBe(`${String(PITCH)}px`);
  expect(c.style.left).toBe(`${String(2 * PITCH)}px`);
});

it('MOT-030: a tab grabbed mid-motion keeps the offset from where it is on screen', () => {
  const { tabs } = rig();
  const [a] = tabs as [HTMLElement];
  a.style.translate = '12px 0px';
  pointer(a, 'pointerdown', 62);
  pointer(a, 'pointermove', 92);
  expect(shown(a)[0]).toBe(42);
});

it('5.7.4: the tabs it passes move aside with spring-move', () => {
  const { tabs } = rig();
  const [a] = tabs as [HTMLElement];
  pointer(a, 'pointerdown', 50);
  pointer(a, 'pointermove', 170);
  const aside = runs.filter((run) => run.keyframes.at(-1) === -PITCH);
  expect(aside).toHaveLength(1);
  expect(aside[0]?.options).toMatchObject({ type: 'spring', stiffness: spring.move.stiffness });
});

it('MOT-031 / MOT-035: the release spring starts with the pointer velocity, one spring per axis', () => {
  const { tabs, clock, order } = rig();
  drag(tabs[0] as HTMLElement, clock, 50, [
    [80, 50],
    [110, 100],
  ]);
  // 60 px in 100 ms = 600 px/s; centre 110 projects to 110 + 0.6 × 499 ≈ 409, the last place.
  expect(order()).toEqual(['b', 'c', 'a']);
  const momentum = runs.filter((run) => run.options.stiffness === spring.momentum.stiffness);
  expect(momentum.map((run) => run.options.velocity)).toEqual([600, 0]);
});

it('MOT-031 / MOT-035: a release moving down starts the Y spring with the pointer\'s Y velocity', () => {
  const { tabs, clock } = rig();
  drag(tabs[0] as HTMLElement, clock, 50, [
    [80, 50, 28],
    [110, 100, 38],
  ]);
  // 20 px down in 100 ms = 200 px/s, while the tab itself follows with resistance.
  const momentum = runs.filter((run) => run.options.stiffness === spring.momentum.stiffness);
  expect(momentum.map((run) => run.options.velocity)).toEqual([600, 200]);
});

it('MOT-032: the new place comes from the projected position, not the release position', () => {
  const slow = rig();
  // Released 20 px right of its place after a pause: no velocity, it stays.
  drag(slow.tabs[0] as HTMLElement, slow.clock, 50, [
    [70, 0],
    [70, 500],
  ]);
  expect(slow.order()).toEqual(['a', 'b', 'c']);

  runs.length = 0;
  const flick = rig();
  // Released at the same place while moving right at about 222 px/s: the projection passes place 1.
  drag(flick.tabs[0] as HTMLElement, flick.clock, 50, [[70, 90]]);
  expect(flick.order()).toEqual(['b', 'a', 'c']);
});

it('MOT-040: under reduced motion the order changes at once and nothing springs', () => {
  const { tabs, clock, order } = rig({ preference: reduced });
  drag(tabs[0] as HTMLElement, clock, 50, [
    [160, 100],
    [270, 600],
  ]);
  expect(order()).toEqual(['b', 'c', 'a']);
  expect(runs).toEqual([]);
  expect(tabs.map((tab) => shown(tab))).toEqual([
    [0, 0],
    [0, 0],
    [0, 0],
  ]);
});

it('a press without a drag is still a click; the click that ends a drag is swallowed', () => {
  const { tabs, clock } = rig();
  const a = tabs[0] as HTMLElement;
  const clicked = vi.fn();
  a.addEventListener('click', clicked);
  pointer(a, 'pointerdown', 50);
  pointer(a, 'pointerup', 50);
  a.click();
  expect(clicked).toHaveBeenCalledOnce();
  drag(a, clock, 50, [[150, 50]]);
  a.click();
  expect(clicked).toHaveBeenCalledOnce();
});

it('a drag that starts on a control marked data-no-drag does not move the tab', () => {
  const { tabs } = rig();
  const a = tabs[0] as HTMLElement;
  const close = document.createElement('button');
  close.dataset.noDrag = '';
  a.append(close);
  pointer(close, 'pointerdown', 90);
  pointer(close, 'pointermove', 200);
  expect(a.style.translate).toBe('');
});

describe('drag out (RA-002, D-13)', () => {
  function dragOutOf(r: Rig): void {
    const a = r.tabs[0] as HTMLElement;
    pointer(a, 'pointerdown', 50);
    pointer(a, 'pointermove', 60, 120);
    pointer(a, 'pointerup', 60, 200);
  }

  it('with a port, a release outside the strip pops the session out once and does not reorder', () => {
    const r = rig({ canDragOut: () => true });
    dragOutOf(r);
    expect(r.dragOut).toHaveBeenCalledOnce();
    expect(r.order()).toEqual(['a', 'b', 'c']);
  });

  it('without a port, the same drag does not pop out and the tab follows only with resistance', () => {
    const r = rig();
    const a = r.tabs[0] as HTMLElement;
    pointer(a, 'pointerdown', 50);
    pointer(a, 'pointermove', 60, 120);
    expect(shown(a)[1]).toBeCloseTo(rubberband(102, 36));
    pointer(a, 'pointerup', 60, 200);
    expect(r.dragOut).not.toHaveBeenCalled();
  });

  it('a pop out that fails puts the tab back in its place', async () => {
    const r = rig({ canDragOut: () => true });
    r.dragOut.mockResolvedValueOnce(false);
    dragOutOf(r);
    await Promise.resolve();
    await Promise.resolve();
    const back = runs.filter((run) => run.options.stiffness === spring.momentum.stiffness);
    expect(back.map((run) => run.keyframes.at(-1))).toEqual([0, 0]);
    expect(r.tabs[0]?.style.position).toBe('');
  });

  it('a tab that left fades where it was let go, and the others close its place', async () => {
    const r = rig({ canDragOut: () => true });
    dragOutOf(r);
    const [a, b] = r.tabs as [HTMLElement, HTMLElement];
    expect(a.style.position).toBe('fixed');
    await Promise.resolve();
    await Promise.resolve();
    expect(a.dataset.closing).toBe('');
    expect(a.style.position).toBe('fixed');
    expect(b.style.marginInlineStart).toBe('');
  });
});
