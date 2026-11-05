// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { flushSync } from 'svelte';
import { compile } from 'svelte/compiler';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { installBlankCanvas } from '../tunnel/fake/canvas.fixture';
import { FakeTunnelSocket } from '../tunnel/fake/socket.fixture';
import { RESIZE_SETTLE_MS } from './display';
import { KEY_COMBINATIONS } from './keys';
import {
  FakeResizeObserver,
  TOKEN_MARKER,
  connectView,
  flags,
  installBrowserFakes,
  paneOf,
  sent,
  settle,
  showView,
  systemClipboard,
} from './view.fixture.svelte';

let cleanups: (() => unknown)[] = [];

beforeEach(() => {
  vi.useFakeTimers();
  FakeTunnelSocket.reset();
  flags.visible = true;
  flags.focused = true;
  cleanups = [installBlankCanvas(), installBrowserFakes()];
});

afterEach(async () => {
  for (const cleanup of cleanups.reverse()) await cleanup();
  document.body.innerHTML = '';
  vi.useRealTimers();
});

function show() {
  const view = showView();
  cleanups.push(view.close);
  return view;
}

const stepMarks = (target: HTMLElement) =>
  [...target.querySelectorAll<HTMLElement>('[data-step]')].map((step) => `${step.dataset.step ?? ''}:${step.dataset.mark ?? ''}`);

describe('start and display (D-7)', () => {
  it('D-7: the first pane measure starts the session with the pane in device pixels, its DPI and the IANA zone', async () => {
    Object.defineProperty(window, 'devicePixelRatio', { configurable: true, value: 2 });
    cleanups.push(() => Reflect.deleteProperty(window, 'devicePixelRatio'));
    const { calls } = show();
    expect(calls.startSession).toEqual([]);
    FakeResizeObserver.resize(640.4, 400);
    await settle();
    const zone = Intl.DateTimeFormat().resolvedOptions().timeZone;
    expect(calls.startSession).toEqual([['device-1', { widthPx: 1281, heightPx: 800, dpi: 192, timezone: zone }]]);
  });

  it('5.7.3 connection steps: each mark changes when its step ends, and the display shows only when ready', async () => {
    const { target } = show();
    expect(stepMarks(target)).toEqual(['request:active', 'gateway:waiting', 'device:waiting', 'ready:waiting']);
    FakeResizeObserver.resize(800, 600);
    await settle();
    expect(stepMarks(target)).toEqual(['request:done', 'gateway:active', 'device:waiting', 'ready:waiting']);
    const socket = FakeTunnelSocket.latest();
    socket.open();
    flushSync();
    expect(stepMarks(target)).toEqual(['request:done', 'gateway:done', 'device:active', 'ready:waiting']);
    expect(target.querySelector('[data-display]')).toBeNull();
    socket.sync();
    flushSync();
    expect(stepMarks(target)).toEqual([]);
    expect(target.querySelector('[data-display] canvas, [data-display] div')).not.toBeNull();
  });

  it('5.7.3 connection steps: a start that stops marks the step it stopped at', async () => {
    const { target } = show();
    FakeResizeObserver.resize(800, 600);
    await settle();
    FakeTunnelSocket.latest().closeWith(1006);
    await settle();
    expect(stepMarks(target)).toEqual(['request:done', 'gateway:failed', 'device:waiting', 'ready:waiting']);
  });

  it('RA-003: a pane resize sends size once it settles, in device pixels clamped to 1..8192', async () => {
    show();
    const socket = await connectView(800, 600);
    FakeResizeObserver.resize(900, 600);
    vi.advanceTimersByTime(RESIZE_SETTLE_MS - 1);
    FakeResizeObserver.resize(20_000, 0.2);
    vi.advanceTimersByTime(RESIZE_SETTLE_MS - 1);
    expect(sent(socket, 'size')).toEqual([]);
    vi.advanceTimersByTime(1);
    expect(sent(socket, 'size')).toEqual([['8192', '1']]);
  });

  it('D-7: the size the session started with, and a hidden pane (0 × 0), send no size', async () => {
    show();
    const socket = await connectView(800, 600);
    FakeResizeObserver.resize(800, 600);
    FakeResizeObserver.resize(0, 0);
    vi.advanceTimersByTime(RESIZE_SETTLE_MS * 2);
    expect(sent(socket, 'size')).toEqual([]);
  });
});

/**
 * The rules of a component's stylesheet as Svelte compiles it, by selector without the scoping class. jsdom applies
 * no component CSS, so the structural style checks read the compiled rules.
 */
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
  const rules = componentRules('./SessionView.svelte');

  it('MAT-008: no rule on the display, the pane or the stage animates, transforms, filters or blurs them', () => {
    for (const selector of ['.display', '.pane', '.stage']) {
      const declarations = [...(rules.get(selector)?.keys() ?? [])];
      expect(declarations, selector).not.toEqual([]);
      for (const property of MOTION) expect(declarations, `${selector} ${property}`).not.toContain(property);
    }
  });

  it('MOT-007: the connection step icon cell has a fixed width, so a mark glyph of another width moves no label', () => {
    const columns = componentRules('./steps/ConnectionSteps.svelte').get('.step')?.get('grid-template-columns') ?? '';
    expect(columns).toMatch(/^var\(--icon-[a-z]+\) 1fr$/);
  });

  it('UI-M1, D-7: the display makes its own stacking context, so the library layers (z-index -1) paint in front of the session', () => {
    expect(rules.get('.display')?.get('isolation')).toBe('isolate');
  });

  it('UI-M2: the pane focus ring is drawn after the display, over it, without taking pointer input', () => {
    const ring = rules.get('.pane:focus-visible::after');
    expect(ring?.get('position')).toBe('absolute');
    expect(ring?.get('pointer-events')).toBe('none');
    expect(ring?.get('box-shadow')).toContain('var(--accent)');
    expect(rules.get('.pane')?.get('position')).toMatch(/^(absolute|relative)$/);
  });

  it('MAT-008: the library display element has no transform, transition, filter or animation set on it', async () => {
    const { target } = show();
    await connectView();
    const element = target.querySelector<HTMLElement>('[data-display]')?.firstElementChild;
    if (!(element instanceof HTMLElement)) throw new Error('no display');
    for (const property of MOTION) expect(['', 'none'], property).toContain(element.style.getPropertyValue(property) || '');
    expect(element.getAnimations?.() ?? []).toEqual([]);
  });

  it('MAT-008: nothing lies over a live display: no panel, steps or modal is rendered beside it', async () => {
    const { target } = show();
    await connectView();
    const stage = target.querySelector('[data-display]')?.parentElement?.parentElement;
    expect(stage?.children).toHaveLength(1);
    expect(document.querySelector('[role="alertdialog"]')).toBeNull();
  });
});

describe('ended panel and Reconnect (RA-007)', () => {
  it('RA-007: the panel shows the recorded reason, which wins over the close code (D-4)', async () => {
    const { target, answers, calls, ended } = show();
    const socket = await connectView();
    answers.endReason = 'REMOTE_CLOSED';
    socket.closeWith(1011);
    await settle();
    expect(calls.endSession).toEqual(['session-1']);
    expect(target.querySelector('[data-reason]')?.textContent).toBe('The device closed the session.');
    expect(ended.map((ending) => ending.reason)).toEqual(['REMOTE_CLOSED']);
    expect(target.querySelector('[data-display]')).toBeNull();
  });

  it('RA-007: Reconnect calls startSession once and opens a new tunnel with a new token in the same pane', async () => {
    const { target, calls } = show();
    const first = await connectView();
    first.disconnect();
    first.closeWith(1000);
    await settle();
    const reconnect = [...target.querySelectorAll('button')].find((button) => button.textContent.includes('Reconnect'));
    reconnect?.click();
    reconnect?.click();
    await settle();
    expect(calls.startSession).toHaveLength(2);
    expect(FakeTunnelSocket.made).toHaveLength(2);
    const [old, fresh] = FakeTunnelSocket.made.map((socket) => new URL(socket.url).searchParams.get('token'));
    expect(fresh).not.toBe(old);
    FakeTunnelSocket.latest().open();
    FakeTunnelSocket.latest().sync();
    flushSync();
    expect(target.querySelectorAll('[data-display]')).toHaveLength(1);
  });

  it('a refused start shows fixed text, not the hub message, and offers Reconnect', async () => {
    const { target } = show();
    FakeResizeObserver.resize(800, 600);
    await settle();
    FakeTunnelSocket.latest().closeWith(1006);
    await settle();
    expect(target.querySelector('[data-reason]')?.textContent).toBe('Could not open the session.');
  });

  it('closing the view closes its tunnel and ignores the end that follows', async () => {
    const { close, calls, ended } = show();
    const socket = await connectView();
    await close();
    await settle();
    expect(socket.closedByClient).toBeDefined();
    expect(calls.endSession).toEqual([]);
    expect(ended).toEqual([]);
  });

  it('a view closed while the hub answers closes the tunnel it then gets', async () => {
    const { close } = show();
    FakeResizeObserver.resize(800, 600);
    await close();
    await settle();
    expect(FakeTunnelSocket.latest().closedByClient).toBeDefined();
  });
});

describe('LOG-007', () => {
  it('LOG-007: a whole session makes no console call at all', async () => {
    const spies = Object.keys(console)
      .filter((name) => typeof (console as unknown as Record<string, unknown>)[name] === 'function')
      .map((name) => vi.spyOn(console as unknown as Record<string, () => void>, name).mockImplementation(() => undefined));
    const { target } = show();
    const socket = await connectView();
    const pane = paneOf(target);
    socket.clipboard('CLIP-MARKER-91c2');
    pane.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', code: 'Enter', bubbles: true, cancelable: true }));
    pane.dispatchEvent(new KeyboardEvent('keyup', { key: 'Enter', code: 'Enter', bubbles: true, cancelable: true }));
    const trigger = target.querySelector<HTMLButtonElement>('[aria-haspopup="menu"]');
    const menuItems = () => [...document.querySelectorAll<HTMLButtonElement>('[role="menuitem"]')];
    for (const index of KEY_COMBINATIONS.keys()) {
      trigger?.click();
      flushSync();
      menuItems()[index]?.click();
      flushSync();
    }
    systemClipboard.text = 'MENU-PASTE-MARKER-2d7';
    trigger?.click();
    flushSync();
    menuItems().at(-1)?.click();
    await settle();
    const paste = new Event('paste', { bubbles: true, cancelable: true });
    Object.defineProperty(paste, 'clipboardData', { value: { getData: () => 'PASTE-MARKER-4b1e' } });
    pane.dispatchEvent(paste);
    systemClipboard.refuse = true;
    pane.focus();
    flushSync();
    socket.clipboard('REFUSED-CLIP-MARKER');
    await settle();
    expect(target.textContent).toContain('Could not copy the text to your clipboard.');
    trigger?.click();
    flushSync();
    menuItems().at(-1)?.click();
    await settle();
    expect(target.textContent).toContain('Could not read your clipboard.');
    socket.gatewayError(0x0301);
    socket.closeWith(1011);
    await settle();
    const calls = spies.flatMap((spy) => spy.mock.calls);
    for (const spy of spies) spy.mockRestore();
    expect(sent(socket, 'clipboard').length).toBeGreaterThan(1);
    expect(calls).toEqual([]);
    expect(document.body.innerHTML).not.toContain(TOKEN_MARKER);
    expect(target.textContent).not.toContain('CLIP-MARKER');
  });
});
