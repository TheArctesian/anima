// @vitest-environment jsdom
import { createRawSnippet, flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { Hub } from '$lib/hub/hub';
import type { PopOutPort, SessionPort, Sessions } from '../sessions.svelte';
import Layout from '../../../routes/+layout.svelte';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));

const fake = vi.hoisted(() => ({
  port: undefined as unknown as SessionPort,
  popOut: undefined as PopOutPort | undefined,
  sessions: undefined as unknown as Sessions,
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
      close: () => Promise.resolve(),
    }) as unknown as Hub,
}));

// The layout's temporary port is replaced by a fake hub, and `popOut` by the test's port.
vi.mock('$lib/sessions/sessions.svelte', async (importOriginal) => {
  const actual = await importOriginal<typeof import('$lib/sessions/sessions.svelte')>();
  return {
    ...actual,
    createSessions: () => {
      fake.sessions = actual.createSessions(fake.port, fake.popOut);
      return fake.sessions;
    },
  };
});

let app: ReturnType<typeof mount> | undefined;
const keyframes: Keyframe[][] = [];

/** jsdom has no Web Animations; this one finishes at once so outros complete. */
function finishingAnimate(this: Element, frames: Keyframe[]) {
  keyframes.push(frames);
  return {
    cancel: () => undefined,
    currentTime: 0,
    playState: 'finished',
    effect: null,
    set onfinish(done: () => void) {
      queueMicrotask(done);
    },
  };
}

async function settle(): Promise<void> {
  for (let tick = 0; tick < 6; tick += 1) {
    await Promise.resolve();
    flushSync();
  }
}

function show(popOut?: PopOutPort): Sessions {
  fake.popOut = popOut;
  const children = createRawSnippet(() => ({ render: () => '<p id="page">page</p>' }));
  app = mount(Layout, { target: document.body, props: { children } });
  flushSync();
  return fake.sessions;
}

const request = { deviceId: 'web-1', deviceName: 'web-1', protocol: 'SSH', origin: null } as const;
const tabs = () => [...document.querySelectorAll<HTMLElement>('[role="tab"]')];
const visiblePanels = () => [...document.querySelectorAll('[role="tabpanel"]')].filter((panel) => !panel.closest('[hidden]'));

async function openStarted(sessions: Sessions, origin: DOMRect | null = null): Promise<string> {
  const id = sessions.openSession({ ...request, origin });
  await sessions.start(id, { widthPx: 800, heightPx: 600 });
  await settle();
  return id;
}

beforeEach(() => {
  let count = 0;
  fake.port = {
    start: vi.fn(() => {
      count += 1;
      return Promise.resolve({ sessionId: `session-${String(count)}` });
    }),
    end: vi.fn(() => Promise.resolve()),
  };
  keyframes.length = 0;
  Object.defineProperty(Element.prototype, 'animate', { value: finishingAnimate, configurable: true });
});

afterEach(async () => {
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
  Reflect.deleteProperty(Element.prototype, 'animate');
});

it('no tabs: the strip is hidden and the pages show', () => {
  show();
  expect(document.querySelector('[role="tablist"]')?.hasAttribute('hidden')).toBe(true);
  expect(visiblePanels()).toEqual([]);
});

it('RA-001: two opens of one device give two tabs with name and protocol; the newest is selected', async () => {
  const sessions = show();
  await openStarted(sessions);
  await openStarted(sessions);
  const label = (tab: HTMLElement) => [tab.querySelector('.name'), tab.querySelector('.protocol')].map((part) => part?.textContent);
  expect(tabs().map(label)).toEqual([
    ['web-1', 'SSH'],
    ['web-1', 'SSH'],
  ]);
  expect(tabs().map((tab) => tab.getAttribute('aria-selected'))).toEqual(['false', 'true']);
  expect(visiblePanels().map((panel) => panel.getAttribute('aria-labelledby'))).toEqual([tabs()[1]?.id]);
});

it('RA-001: switching tabs keeps both sessions mounted and shows only the selected one', async () => {
  const sessions = show();
  await openStarted(sessions);
  await openStarted(sessions);
  tabs()[0]?.click();
  flushSync();
  expect(tabs().map((tab) => tab.getAttribute('aria-selected'))).toEqual(['true', 'false']);
  expect(document.querySelectorAll('[data-placeholder="session-view"]')).toHaveLength(2);
  expect(visiblePanels()).toHaveLength(1);
  expect(fake.port.end).not.toHaveBeenCalled();
});

it('RA-001 / UI-015: the labelled close button ends that session once and keeps the other tab', async () => {
  const sessions = show();
  await openStarted(sessions);
  await openStarted(sessions);
  const close = document.querySelector<HTMLButtonElement>('button[aria-label="Close web-1 SSH session"]');
  expect(close?.getAttribute('title')).toBe('Close web-1 SSH session');
  close?.click();
  await settle();
  expect(fake.port.end).toHaveBeenCalledExactlyOnceWith('session-1');
  expect(tabs()).toHaveLength(1);
  expect(tabs()[0]?.getAttribute('aria-selected')).toBe('true');
});

it('keyboard: the selected tab is the one Tab stop, the arrows move focus, Delete closes', async () => {
  const sessions = show();
  await openStarted(sessions);
  await openStarted(sessions);
  expect(tabs().map((tab) => tab.tabIndex)).toEqual([-1, 0]);
  tabs()[1]?.focus();
  tabs()[1]?.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowRight', bubbles: true }));
  expect(document.activeElement).toBe(tabs()[0]);
  tabs()[0]?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Delete', bubbles: true }));
  await settle();
  expect(fake.port.end).toHaveBeenCalledExactlyOnceWith('session-1');
  expect(tabs()).toHaveLength(1);
});

it('MOT-005: a new tab grows with transform-origin at the control that opened it', async () => {
  const sessions = show();
  await openStarted(sessions, new DOMRect(400, 300, 50, 20));
  const growing = keyframes.find((frames) => frames.some((frame) => 'transformOrigin' in frame));
  expect(growing?.[0]?.transformOrigin).toBe('425px 310px');
});

describe('layout mount (WP-14 E-2)', () => {
  it('the pages stay mounted, covered and inert, while a session tab is selected', async () => {
    const sessions = show();
    const page = document.querySelector('#page');
    await openStarted(sessions);
    expect(document.querySelector('#page')).toBe(page);
    // Svelte sets the `inert` property, which a browser reflects to the attribute (jsdom does not).
    const pages = page?.closest<HTMLElement>('.pages');
    expect(pages?.inert).toBe(true);
    expect(pages?.classList.contains('covered')).toBe(true);
    expect(visiblePanels()).toHaveLength(1);
  });

  it('D-11: the nav Devices item leaves the session tab open and shows the pages', async () => {
    const sessions = show();
    await openStarted(sessions);
    const devices = document.querySelector<HTMLElement>('nav a[href="/devices"]');
    // jsdom cannot follow the link; the router is not under test here.
    document.addEventListener(
      'click',
      (event) => {
        event.preventDefault();
      },
      { once: true },
    );
    devices?.click();
    flushSync();
    expect(sessions.active).toBeNull();
    expect(tabs()).toHaveLength(1);
    expect(visiblePanels()).toEqual([]);
    expect(document.querySelector<HTMLElement>('.pages')?.inert).toBe(false);
    expect(fake.port.end).not.toHaveBeenCalled();
  });
});

describe('pop out (RA-002, D-13)', () => {
  it('with no port, no menu and no "Pop out" are offered', async () => {
    const sessions = show();
    await openStarted(sessions);
    expect(document.querySelector('[aria-haspopup="menu"]')).toBeNull();
    expect(document.body.textContent).not.toContain('Pop out');
  });

  it('with a port, "Pop out" in the tab menu hands the session over once and the tab leaves', async () => {
    const popOut = vi.fn(() => Promise.resolve());
    const sessions = show(popOut);
    await openStarted(sessions);
    const more = document.querySelector<HTMLButtonElement>('[aria-haspopup="menu"]');
    expect(more?.getAttribute('aria-label')).toBe('More for web-1 SSH session');
    more?.click();
    await settle();
    document.querySelector<HTMLButtonElement>('[role="menuitem"]')?.click();
    await settle();
    expect(popOut).toHaveBeenCalledExactlyOnceWith('session-1');
    expect(tabs()).toEqual([]);
    expect(fake.port.end).not.toHaveBeenCalled();
  });

  it('with a port, dragging a tab out of the strip calls it once with the session id', async () => {
    const popOut = vi.fn(() => Promise.resolve());
    const sessions = show(popOut);
    await openStarted(sessions);
    const slot = document.querySelector<HTMLElement>('[data-slot]');
    if (slot === null) throw new Error('no tab');
    slot.setPointerCapture = vi.fn();
    const at = (type: string, y: number) =>
      new PointerEvent(type, { bubbles: true, button: 0, pointerId: 3, clientX: 20, clientY: y });
    slot.querySelector('[role="tab"]')?.dispatchEvent(at('pointerdown', 10));
    slot.dispatchEvent(at('pointermove', 120));
    slot.dispatchEvent(at('pointerup', 200));
    await settle();
    expect(popOut).toHaveBeenCalledExactlyOnceWith('session-1');
  });

  it('"Pop out" waits for the session to start', () => {
    const sessions = show(vi.fn(() => Promise.resolve()));
    sessions.openSession(request);
    flushSync();
    document.querySelector<HTMLButtonElement>('[aria-haspopup="menu"]')?.click();
    flushSync();
    expect(document.querySelector<HTMLButtonElement>('[role="menuitem"]')?.disabled).toBe(true);
  });
});
