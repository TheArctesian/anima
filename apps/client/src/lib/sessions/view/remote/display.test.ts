// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { flushSync } from 'svelte';
import { compile } from 'svelte/compiler';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { installBlankCanvas } from '../../tunnel/fake/canvas.fixture';
import { FakeTunnelSocket } from '../../tunnel/fake/socket.fixture';
import { RESIZE_SETTLE_MS } from '../display';
import { FakeResizeObserver, installBrowserFakes } from '../view.fixture.svelte';
import { displayScale } from './fit';
import { connectRemote, remoteFlags, sentOf, showRemote } from './remote.fixture.svelte';

let cleanups: (() => unknown)[] = [];

beforeEach(() => {
  vi.useFakeTimers();
  FakeTunnelSocket.reset();
  remoteFlags.visible = true;
  remoteFlags.focused = true;
  cleanups = [installBlankCanvas(), installBrowserFakes()];
});

afterEach(async () => {
  for (const cleanup of cleanups.reverse()) await cleanup();
  document.body.innerHTML = '';
  vi.useRealTimers();
});

function show(...args: Parameters<typeof showRemote>) {
  const view = showRemote(...args);
  cleanups.push(view.close);
  return view;
}

function withRatio(ratio: number) {
  Object.defineProperty(window, 'devicePixelRatio', { configurable: true, value: ratio });
  cleanups.push(() => Reflect.deleteProperty(window, 'devicePixelRatio'));
}

describe('D-5: RDP follows the pane (RA-004)', () => {
  it('RA-004: the session starts at the pane in device pixels, its DPI and the IANA zone', async () => {
    withRatio(2);
    const { calls } = show('RDP');
    FakeResizeObserver.resize(640.4, 400);
    await Promise.resolve();
    const zone = Intl.DateTimeFormat().resolvedOptions().timeZone;
    expect(calls.startSession).toEqual([['device-1', 'RDP', { widthPx: 1281, heightPx: 800, dpi: 192, timezone: zone }]]);
  });

  it('RA-004: a pane resize sends one size once the pane has settled for 150 ms, clamped to 1..8192', async () => {
    show('RDP');
    const socket = await connectRemote(800, 600);
    FakeResizeObserver.resize(900, 600);
    vi.advanceTimersByTime(RESIZE_SETTLE_MS - 1);
    FakeResizeObserver.resize(20_000, 0.2);
    vi.advanceTimersByTime(RESIZE_SETTLE_MS - 1);
    expect(sentOf(socket, 'size')).toEqual([]);
    vi.advanceTimersByTime(1);
    expect(sentOf(socket, 'size')).toEqual([['8192', '1']]);
    vi.advanceTimersByTime(RESIZE_SETTLE_MS * 4);
    expect(sentOf(socket, 'size')).toHaveLength(1);
  });

  it('D-5: the size the session started with, and a hidden pane (0 × 0), send no size', async () => {
    show('RDP');
    const socket = await connectRemote(800, 600);
    FakeResizeObserver.resize(800, 600);
    FakeResizeObserver.resize(0, 0);
    vi.advanceTimersByTime(RESIZE_SETTLE_MS * 2);
    expect(sentOf(socket, 'size')).toEqual([]);
  });

  it('D-5: the RDP display is drawn one remote pixel per device pixel', async () => {
    withRatio(2);
    const { heard } = show('RDP');
    await connectRemote(800, 600);
    expect(heard.clients[0]?.getDisplay().getScale()).toBe(0.5);
  });
});

/** A `matchMedia` whose queries a test can fire; `devicePixelRatio` is set by the test. */
function fakeResolutionQueries() {
  const listeners: (() => void)[] = [];
  Object.defineProperty(window, 'matchMedia', {
    configurable: true,
    value: () => ({ addEventListener: (_type: string, listener: () => void) => listeners.push(listener), removeEventListener: () => undefined }),
  });
  cleanups.push(() => Reflect.deleteProperty(window, 'matchMedia'));
  return {
    moveTo: (ratio: number) => {
      Object.defineProperty(window, 'devicePixelRatio', { configurable: true, value: ratio });
      for (const listener of listeners.splice(0)) listener();
      flushSync();
    },
  };
}

describe('D-5: a monitor change (devicePixelRatio)', () => {
  it('RA-004: the RDP display rescales, and the pane is sent again in the new device pixels once it settles', async () => {
    withRatio(1);
    const monitor = fakeResolutionQueries();
    const { heard } = show('RDP');
    const socket = await connectRemote(800, 600);
    const display = heard.clients[0]?.getDisplay();
    expect(display?.getScale()).toBe(1);
    monitor.moveTo(2);
    expect(display?.getScale()).toBe(0.5);
    vi.advanceTimersByTime(RESIZE_SETTLE_MS);
    expect(sentOf(socket, 'size')).toEqual([['1600', '1200']]);
    monitor.moveTo(1);
    expect(display?.getScale()).toBe(1);
  });
});

describe('D-5: VNC fits the display in the pane', () => {
  it('D-5: a pane resize sends no size, and the display scales to fit, recomputed on each change', async () => {
    const { heard } = show('VNC');
    const socket = await connectRemote(800, 600);
    socket.instruction('size', 0, 1600, 1000);
    socket.sync(1);
    flushSync();
    const display = heard.clients[0]?.getDisplay();
    expect(display?.getScale()).toBe(0.5);
    FakeResizeObserver.resize(400, 600);
    flushSync();
    expect(display?.getScale()).toBe(0.25);
    FakeResizeObserver.resize(3200, 2000);
    flushSync();
    expect(display?.getScale()).toBe(1);
    vi.advanceTimersByTime(RESIZE_SETTLE_MS * 4);
    expect(sentOf(socket, 'size')).toEqual([]);
  });
});

describe('displayScale (D-5)', () => {
  it.each([
    ['RDP at ratio 1', 'RDP', 1, 1],
    ['RDP at ratio 2', 'RDP', 2, 0.5],
    ['RDP at a broken ratio', 'RDP', Number.NaN, 1],
    ['RDP at ratio 0', 'RDP', 0, 1],
  ] as const)('%s ignores both sizes', (_name, protocol, ratio, expected) => {
    expect(displayScale(protocol, { width: 4000, height: 3000 }, { width: 100, height: 100 }, ratio)).toBe(expected);
  });

  it.each([
    ['a larger remote shrinks to the tighter side', { width: 1600, height: 1200 }, { width: 800, height: 900 }, 0.5],
    ['a taller remote shrinks by its height', { width: 1000, height: 2000 }, { width: 1000, height: 500 }, 0.25],
    ['a smaller remote is never upscaled', { width: 640, height: 480 }, { width: 1920, height: 1080 }, 1],
    ['an unknown remote size', { width: 0, height: 0 }, { width: 800, height: 600 }, 1],
    ['a hidden pane', { width: 1600, height: 1200 }, { width: 0, height: 0 }, 1],
    ['a broken pane', { width: 1600, height: 1200 }, { width: Number.NaN, height: 600 }, 1],
  ])('VNC: %s', (_name, remote, pane, expected) => {
    expect(displayScale('VNC', remote, pane, 2)).toBe(expected);
  });
});

/** The rules of a component's stylesheet as Svelte compiles it, by selector without the scoping class. */
function componentRules(file: string): Map<string, Map<string, string>> {
  const source = readFileSync(new URL(file, import.meta.url), 'utf8');
  const code = (compile(source, { filename: file, css: 'external' }).css?.code ?? '').replaceAll(/\/\*[\s\S]*?\*\//g, '');
  const rules = new Map<string, Map<string, string>>();
  for (const [, selector = '', body = ''] of code.matchAll(/([^{}]+)\{([^{}]*)\}/g)) {
    const declarations = body
      .split(';')
      .map((declaration) => declaration.split(':').map((part) => part.trim()))
      .filter(([property]) => property !== '');
    rules.set(selector.replaceAll(/\.svelte-[\w-]+/g, '').trim(), new Map(declarations.map(([property = '', ...value]) => [property, value.join(':')])));
  }
  return rules;
}

describe('MAT-008: the remote display', () => {
  const MOTION = ['transform', 'translate', 'scale', 'rotate', 'filter', 'backdrop-filter', 'transition', 'animation', 'opacity'];
  const rules = componentRules('./RemoteView.svelte');

  it('MAT-008: no rule on the display, the pane or the stage animates, transforms, filters or blurs them', () => {
    for (const selector of ['.display', '.pane', '.stage']) {
      const declarations = [...(rules.get(selector)?.keys() ?? [])];
      expect(declarations, selector).not.toEqual([]);
      for (const property of MOTION) expect(declarations, `${selector} ${property}`).not.toContain(property);
    }
  });

  it('MAT-008: a scaled VNC display has no transition, filter or animation, and nothing lies over it', async () => {
    const { target, heard } = show('VNC');
    const socket = await connectRemote(800, 600);
    socket.instruction('size', 0, 1600, 1200);
    socket.sync(1);
    flushSync();
    expect(heard.clients[0]?.getDisplay().getScale()).toBe(0.5);
    const element = heard.clients[0]?.getDisplay().getElement();
    if (element === undefined) throw new Error('no display');
    for (const node of [element, ...element.querySelectorAll<HTMLElement>('*')]) {
      for (const property of ['transition', 'filter', 'backdrop-filter', 'animation']) expect(node.style.getPropertyValue(property), property).toBe('');
    }
    const stage = target.querySelector('[data-display]')?.parentElement?.parentElement;
    expect(stage?.children).toHaveLength(1);
  });
});
