// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { Hub } from '$lib/hub/hub';
import { duration } from '$lib/motion/tokens';
import { runs } from '$lib/motion/engine.fixture';
import { route } from '$lib/nav/route.fixture.svelte';
import Layout from '../../../routes/+layout.svelte';
import type { PopOutPort, SessionPort } from '../sessions.svelte';
import { grow } from './grow';
import { cleanup, openStarted, prepare, settle, show as mountLayout } from './layout.fixture';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));

vi.mock('$app/state', async () => {
  const { route } = await import('$lib/nav/route.fixture.svelte');
  return {
    page: {
      get url() {
        return new URL(route.pathname, 'http://localhost');
      },
    },
  };
});

vi.mock('$lib/hub/hub', async (importOriginal) => ({
  ...(await importOriginal<typeof import('$lib/hub/hub')>()),
  createHub: (): Hub =>
    ({
      address: { origin: 'https://hub.anima.ing', http: '', socket: '' },
      status: { current: { connection: 'connected', lastContactAt: null } },
      // The nav's live-session item asks the hub; nothing answers here.
      request: () => new Promise(() => undefined),
      subscribe: () => () => undefined,
      close: () => Promise.resolve(),
    }) as unknown as Hub,
}));

// The layout's sessions API is the fixture's fake hub; its port is spied on, and `popOut` is the test's port.
vi.mock('$lib/sessions/api', async (importOriginal) => {
  const { fake: state } = await import('./layout.fixture');
  return { ...(await importOriginal<typeof import('$lib/sessions/api')>()), sessionsApi: () => state.api };
});

vi.mock('$lib/sessions/sessions.svelte', async (importOriginal) => {
  const actual = await importOriginal<typeof import('$lib/sessions/sessions.svelte')>();
  const { spiedSessions } = await import('./layout.fixture');
  return { ...actual, createSessions: (port: SessionPort) => spiedSessions(actual.createSessions, port) };
});

const show = (popOut?: PopOutPort) => mountLayout(Layout, popOut);

beforeEach(() => {
  prepare();
  runs.length = 0;
  route.pathname = '/devices';
});

afterEach(cleanup);

const moving = { reduced: false };
const reduced = { reduced: true };

function box(left: number, top: number, width: number, height: number): DOMRect {
  return new DOMRect(left, top, width, height);
}

const tab = () => {
  const node = document.createElement('div');
  node.getBoundingClientRect = () => box(100, 0, 100, 36);
  return node;
};
const origin = { left: 400, top: 300, width: 50, height: 20 };

/** Where `css` puts the tab's box at its scale about its transform-origin. */
function drawn(css: string, tab: DOMRect): DOMRect {
  const scale = Number(/scale: ([\d.]+);/.exec(css)?.[1]);
  const [x = 0, y = 0] = (/transform-origin: ([-\d.]+)px ([-\d.]+)px;/.exec(css) ?? []).slice(1).map(Number);
  const fixedX = tab.left + x;
  const fixedY = tab.top + y;
  return box(fixedX + scale * (tab.left - fixedX), fixedY + scale * (tab.top - fixedY), scale * tab.width, scale * tab.height);
}

describe('grow (Session tab open, 5.7.3)', () => {
  it('MOT-005: the tab starts centred on the control that opened it, at its width, and grows on spring-snappy', () => {
    const config = grow(tab(), { origin, preference: moving })({ direction: 'in' });
    const start = drawn(config.css?.(0, 1) ?? '', box(100, 0, 100, 36));
    expect(start.left).toBeCloseTo(origin.left, 6);
    expect(start.width).toBeCloseTo(origin.width, 6);
    expect(start.top + start.height / 2).toBeCloseTo(origin.top + origin.height / 2, 6);
    expect(config.css?.(1, 0)).toContain('scale: 1;');
    expect(config.duration).toBeGreaterThan(0);
    expect(config.easing?.(1)).toBeCloseTo(1);
  });

  it('MOT-005: a control as wide as the tab or wider keeps the tab at full size, about the control centre', () => {
    for (const width of [100, 160]) {
      const wide = { left: 400, top: 300, width, height: 20 };
      const css = grow(tab(), { origin: wide, preference: moving })({ direction: 'in' }).css?.(0, 1) ?? '';
      expect(css).toContain('scale: 1;');
      // Control centre less the tab's corner (100, 0).
      expect(css).toContain(`transform-origin: ${String(300 + width / 2)}px 310px;`);
    }
  });

  it('MOT-040: under reduced motion the tab fades in place over duration-base', () => {
    const config = grow(tab(), { origin, preference: reduced })({ direction: 'in' });
    expect(config.css?.(0.5, 0.5)).toBe('opacity: 0.5;');
    expect(config.duration).toBe(duration.base);
  });

  it('5.7.3: a closing tab closes to its own place over duration-fast; reduced motion only fades', () => {
    const exit = grow(tab(), { origin, preference: moving })({ direction: 'out' });
    expect(exit.css?.(0.5, 0.5)).toBe('opacity: 0.5; scale: 0.5;');
    expect(exit.css?.(0.5, 0.5)).not.toContain('transform-origin');
    expect(exit.duration).toBe(duration.fast);
    expect(grow(tab(), { origin, preference: reduced })({ direction: 'out' }).css?.(0.5, 0.5)).toBe('opacity: 0.5;');
  });
});

describe('grow: reversal mid-flight (MOT-002)', () => {
  it('MOT-002: a close mid-open goes back from where the tab is, with no step in scale or place', () => {
    const role = grow(tab(), { origin, preference: moving });
    const opening = role({ direction: 'in' });
    const closing = role({ direction: 'out' });
    for (const t of [0.1, 0.4, 0.9]) expect(closing.css?.(t, 1 - t)).toBe(opening.css?.(t, 1 - t));
    expect(closing.duration).toBe(duration.fast);
  });

  it('MOT-002: a tab with no known control fades in, and a close mid-open fades out the same way', () => {
    const role = grow(tab(), { origin: null, preference: moving });
    const opening = role({ direction: 'in' });
    const closing = role({ direction: 'out' });
    for (const t of [0.2, 0.5, 0.8]) expect(closing.css?.(t, 1 - t)).toBe(opening.css?.(t, 1 - t));
    expect(closing.css?.(0.5, 0.5)).toBe('opacity: 0.5;');
  });

  it('MOT-002: a reopen mid-close grows back from where the closing tab is', () => {
    const role = grow(tab(), { origin, preference: moving });
    const closing = role({ direction: 'out' });
    const reopening = role({ direction: 'in' });
    for (const t of [0.2, 0.7]) expect(reopening.css?.(t, 1 - t)).toBe(closing.css?.(t, 1 - t));
  });
});

it('MOT-005 / MV-002: the last tab closes with the strip still shown and out of the flow, and the strip goes when the exit ends', async () => {
  const sessions = show();
  await openStarted(sessions);
  const strip = document.querySelector('[role="tablist"]');
  expect(strip?.hasAttribute('hidden')).toBe(false);
  document.querySelector<HTMLButtonElement>('button[aria-label="Close web-1 SSH session"]')?.click();
  // The exit is playing: the slot is still drawn in a laid-out strip that no longer holds the page down.
  expect(strip?.hasAttribute('hidden')).toBe(false);
  expect(strip?.classList.contains('leaving')).toBe(true);
  expect(strip?.querySelector('[data-slot]')).not.toBeNull();
  await settle();
  expect(strip?.querySelector('[data-slot]')).toBeNull();
  expect(strip?.hasAttribute('hidden')).toBe(true);
  expect(strip?.classList.contains('leaving')).toBe(false);
});

it('MOT-005: with two tabs closed in quick succession the strip stays shown until the second exit ends', async () => {
  const sessions = show();
  await openStarted(sessions);
  await openStarted(sessions);
  const strip = document.querySelector('[role="tablist"]');
  // Exits that end only when the test says so.
  const pending: { element: Element; done: () => void }[] = [];
  Object.defineProperty(Element.prototype, 'animate', {
    configurable: true,
    value(this: Element) {
      const element: Element = Object(this) as Element;
      const animation = {
        cancel: () => undefined,
        currentTime: 0,
        playState: 'running',
        effect: null,
        set onfinish(done: () => void) {
          pending.push({
            element,
            done: () => {
              animation.playState = 'finished';
              done();
            },
          });
        },
      };
      return animation;
    },
  });
  const [first, second] = [...document.querySelectorAll<HTMLButtonElement>('button[aria-label="Close web-1 SSH session"]')];
  first?.click();
  await settle();
  second?.click();
  await settle();
  const closing = [...document.querySelectorAll('[data-closing]')];
  expect(closing).toHaveLength(2);
  const finish = async (slot: Element | undefined) => {
    // Svelte plays a delay animation, then the exit itself: finish each as it appears.
    const finished = new Set<unknown>();
    for (let round = 0; round < 3; round += 1) {
      for (const exit of pending.filter((one) => one.element === slot && !finished.has(one))) {
        finished.add(exit);
        exit.done();
      }
      await settle();
    }
  };
  await finish(closing[0]);
  // One exit has ended and the other still plays: the strip is still shown.
  expect(strip?.hasAttribute('hidden')).toBe(false);
  expect(strip?.querySelectorAll('[data-slot]')).toHaveLength(1);
  await finish(closing[1]);
  expect(strip?.querySelector('[data-slot]')).toBeNull();
  expect(strip?.hasAttribute('hidden')).toBe(true);
});
