// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { runs } from '$lib/motion/engine.fixture';
import { rubberband } from '$lib/motion/gesture';
import { duration, spring } from '$lib/motion/tokens';
import { dragTab, type DragOptions } from './drag';
import { grow } from './grow';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));

const moving = { reduced: false };
const reduced = { reduced: true };

function box(left: number, top: number, width: number, height: number): DOMRect {
  return new DOMRect(left, top, width, height);
}

describe('grow (Session tab open, 5.7.3)', () => {
  const tab = () => {
    const node = document.createElement('div');
    node.getBoundingClientRect = () => box(100, 0, 100, 36);
    return node;
  };
  const origin = { left: 400, top: 300, width: 50, height: 20 };

  it('MOT-005: the tab grows on spring-snappy with transform-origin at the control that opened it', () => {
    const config = grow(tab(), { origin, preference: moving })({ direction: 'in' });
    // Control centre (425, 310) less the tab's corner (100, 0).
    expect(config.css?.(0, 1)).toContain('transform-origin: 325px 310px;');
    expect(config.css?.(0, 1)).toContain('scale: 0.5;');
    expect(config.css?.(1, 0)).toContain('scale: 1;');
    expect(config.duration).toBeGreaterThan(0);
    expect(config.easing?.(1)).toBeCloseTo(1);
  });

  it('MOT-040: under reduced motion the tab fades in place over duration-base', () => {
    const config = grow(tab(), { origin, preference: reduced })({ direction: 'in' });
    expect(config.css?.(0.5, 0.5)).toBe('opacity: 0.5;');
    expect(config.duration).toBe(duration.base);
  });

  it('a tab with no known control fades in', () => {
    const config = grow(tab(), { origin: null, preference: moving })({ direction: 'in' });
    expect(config.css?.(0.5, 0.5)).toBe('opacity: 0.5;');
  });

  it('5.7.3: a closing tab closes to its own place over duration-fast; reduced motion only fades', () => {
    const exit = grow(tab(), { origin, preference: moving })({ direction: 'out' });
    expect(exit.css?.(0.5, 0.5)).toBe('opacity: 0.5; scale: 0.5;');
    expect(exit.css?.(0.5, 0.5)).not.toContain('transform-origin');
    expect(exit.duration).toBe(duration.fast);
    expect(grow(tab(), { origin, preference: reduced })({ direction: 'out' }).css?.(0.5, 0.5)).toBe('opacity: 0.5;');
  });
});

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
    node.setPointerCapture = vi.fn();
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

/** Drags from `fromX` through `points` (x at time t) and releases at the last one. */
function drag(node: HTMLElement, clock: { now: number }, fromX: number, points: [number, number][]): void {
  clock.now = 0;
  pointer(node, 'pointerdown', fromX);
  for (const [x, t] of points) {
    clock.now = t;
    pointer(node, 'pointermove', x);
  }
  const [lastX = fromX] = points.at(-1) ?? [];
  pointer(node, 'pointerup', lastX);
}

beforeEach(() => {
  runs.length = 0;
  document.body.innerHTML = '';
});

it('MOT-030: the tab follows the pointer only after threshold-drag, with pointer capture and the grab offset kept', () => {
  const { tabs } = rig();
  const [a] = tabs as [HTMLElement];
  pointer(a, 'pointerdown', 50);
  pointer(a, 'pointermove', 55);
  expect(a.style.translate).toBe('');
  expect(a.setPointerCapture).not.toHaveBeenCalled();
  pointer(a, 'pointermove', 80, 23);
  expect(a.setPointerCapture).toHaveBeenCalledWith(7);
  expect(shown(a)).toEqual([30, rubberband(5, 36)]);
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
  });
});
