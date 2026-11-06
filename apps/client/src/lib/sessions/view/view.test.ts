// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { flushSync } from 'svelte';
import { compile } from 'svelte/compiler';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { installBlankCanvas } from '../tunnel/fake/canvas.fixture';
import { FakeTerminalSocket } from '../tunnel/terminal/fake/socket.fixture';
import { KEY_SEQUENCES } from './menu/sequences';
import {
  FakeResizeObserver,
  TOKEN_MARKER,
  connectView,
  flags,
  installBrowserFakes,
  screenText,
  settle,
  showView,
  systemClipboard,
} from './view.fixture.svelte';

let cleanups: (() => unknown)[] = [];

beforeEach(() => {
  vi.useFakeTimers();
  FakeTerminalSocket.reset();
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
    const original = window.devicePixelRatio;
    Object.defineProperty(window, 'devicePixelRatio', { configurable: true, value: 2 });
    cleanups.push(() => Object.defineProperty(window, 'devicePixelRatio', { configurable: true, value: original }));
    const { calls } = show();
    expect(calls.startSession).toEqual([]);
    FakeResizeObserver.resize(640.4, 400);
    await settle();
    const zone = Intl.DateTimeFormat().resolvedOptions().timeZone;
    expect(calls.startSession).toEqual([['device-1', { widthPx: 1281, heightPx: 800, dpi: 192, timezone: zone }]]);
  });

  it('5.7.3 connection steps: each mark changes when its step ends, and the terminal shows only when the socket is open', async () => {
    const { target } = show();
    expect(stepMarks(target)).toEqual(['request:active', 'gateway:waiting', 'device:waiting', 'ready:waiting']);
    FakeResizeObserver.resize(800, 600);
    await settle();
    expect(stepMarks(target)).toEqual(['request:done', 'gateway:active', 'device:waiting', 'ready:waiting']);
    expect(target.querySelector('.panel')).not.toBeNull();
    FakeTerminalSocket.latest().open();
    flushSync();
    expect(stepMarks(target)).toEqual([]);
    expect(target.querySelector('.panel')).toBeNull();
  });

  it('5.7.3 connection steps: a start that stops marks the step it stopped at', async () => {
    const { target } = show();
    FakeResizeObserver.resize(800, 600);
    await settle();
    FakeTerminalSocket.latest().closeWith(1006);
    await settle();
    expect(stepMarks(target)).toEqual(['request:done', 'gateway:failed', 'device:waiting', 'ready:waiting']);
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

describe('MAT-008: the terminal', () => {
  const MOTION = ['transform', 'translate', 'scale', 'rotate', 'filter', 'backdrop-filter', 'transition', 'animation', 'opacity'];
  const rules = componentRules('./SessionView.svelte');

  it('MAT-008: no rule on the pane or the stage animates, transforms, filters or blurs them', () => {
    for (const selector of ['.pane', '.stage']) {
      const declarations = [...(rules.get(selector)?.keys() ?? [])];
      expect(declarations, selector).not.toEqual([]);
      for (const property of MOTION) expect(declarations, `${selector} ${property}`).not.toContain(property);
    }
  });

  it('MOT-007: the connection step icon cell has a fixed width, so a mark glyph of another width moves no label', () => {
    const columns = componentRules('./steps/ConnectionSteps.svelte').get('.step')?.get('grid-template-columns') ?? '';
    expect(columns).toMatch(/^var\(--icon-[a-z]+\) 1fr$/);
  });

  it('UI-M2: the pane focus ring is drawn after the terminal, over it, without taking pointer input', () => {
    const ring = rules.get('.pane:focus-within::after');
    expect(ring?.get('position')).toBe('absolute');
    expect(ring?.get('pointer-events')).toBe('none');
    expect(ring?.get('box-shadow')).toContain('var(--accent)');
    expect(rules.get('.pane')?.get('position')).toMatch(/^(absolute|relative)$/);
  });

  it('MAT-008: the xterm.js element has no transform, transition, filter or animation set on it', async () => {
    const { target } = show();
    await connectView();
    const element = target.querySelector<HTMLElement>('.xterm');
    if (element === null) throw new Error('no terminal');
    for (const property of MOTION) expect(['', 'none'], property).toContain(element.style.getPropertyValue(property) || '');
    expect(element.getAnimations?.() ?? []).toEqual([]);
  });

  it('MAT-008, MOT-007: nothing lies over an open terminal: no panel, steps or modal is rendered beside it', async () => {
    const { target } = show();
    await connectView();
    expect(target.querySelector('.panel')).toBeNull();
    expect(document.querySelector('[role="alertdialog"], [role="dialog"]')).toBeNull();
  });

  it('RA-011: without WebGL the DOM renderer draws the terminal', async () => {
    const { target } = show();
    const socket = await connectView();
    socket.output('drawn');
    expect(screenText(target)).toContain('drawn');
  });
});

describe('ended panel and Reconnect (RA-007)', () => {
  it('RA-007: the panel shows the recorded reason, which wins over the end message and the close code (D-4)', async () => {
    const { target, answers, calls, ended } = show();
    const socket = await connectView();
    answers.endReason = 'REMOTE_CLOSED';
    socket.end('GATEWAY_ERROR');
    socket.closeWith(1011);
    await settle();
    expect(calls.endSession).toEqual(['session-1']);
    expect(target.querySelector('[data-reason]')?.textContent).toBe('The device closed the session.');
    expect(ended.map((ending) => ending.reason)).toEqual(['REMOTE_CLOSED']);
  });

  it('RA-007: Reconnect calls startSession once, opens a new socket with a new token and clears the old screen', async () => {
    const { target, calls, heard } = show();
    const first = await connectView();
    first.output('old session text');
    first.closeWith(1000);
    await settle();
    expect(screenText(target)).toContain('old session text');
    const reconnect = [...target.querySelectorAll('button')].find((button) => button.textContent.includes('Reconnect'));
    reconnect?.click();
    reconnect?.click();
    await settle();
    expect(calls.startSession).toHaveLength(2);
    expect(FakeTerminalSocket.made).toHaveLength(2);
    const [old, fresh] = FakeTerminalSocket.made.map((socket) => new URL(socket.url).searchParams.get('token'));
    expect(fresh).not.toBe(old);
    FakeTerminalSocket.latest().open();
    flushSync();
    expect(screenText(target)).not.toContain('old session text');
    expect(heard.titles.at(-1)).toBe('');
  });

  it('a refused start shows fixed text, not the hub message, and offers Reconnect', async () => {
    const { target } = show();
    FakeResizeObserver.resize(800, 600);
    await settle();
    FakeTerminalSocket.latest().closeWith(1006);
    await settle();
    expect(target.querySelector('[data-reason]')?.textContent).toBe('Could not open the session.');
    expect([...target.querySelectorAll('button')].some((button) => button.textContent.includes('Reconnect'))).toBe(true);
  });

  it('closing the view closes its socket and ignores the end that follows', async () => {
    const { close, calls, ended } = show();
    const socket = await connectView();
    await close();
    await settle();
    expect(socket.closedByClient).toBe(1000);
    expect(calls.endSession).toEqual([]);
    expect(ended).toEqual([]);
  });

  it('a view closed before its terminal is open starts no session', async () => {
    const { close, calls } = show();
    FakeResizeObserver.resize(800, 600);
    await close();
    await settle();
    expect(calls.startSession).toEqual([]);
    expect(FakeTerminalSocket.made).toEqual([]);
  });
});

describe('LOG-007, SEC-002', () => {
  it('LOG-007: a whole session makes no console call, and the token appears only in the socket address', async () => {
    const spies = Object.keys(console)
      .filter((name) => typeof (console as unknown as Record<string, unknown>)[name] === 'function')
      .map((name) => vi.spyOn(console as unknown as Record<string, () => void>, name).mockImplementation(() => undefined));
    const { target } = show();
    const socket = await connectView();
    socket.output('prompt$ \x1b]0;a title\x07');
    const trigger = target.querySelector<HTMLButtonElement>('[aria-haspopup="menu"]');
    const menuItems = () => [...document.querySelectorAll<HTMLButtonElement>('[role="menuitem"]')];
    for (const index of KEY_SEQUENCES.keys()) {
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
    systemClipboard.refuse = true;
    trigger?.click();
    flushSync();
    menuItems().at(-1)?.click();
    await settle();
    expect(target.textContent).toContain('Could not read your clipboard.');
    socket.end('AUTH_FAILED');
    socket.closeWith(1011);
    await settle();
    const calls = spies.flatMap((spy) => spy.mock.calls);
    for (const spy of spies) spy.mockRestore();
    expect(calls).toEqual([]);
    expect(socket.url).toContain(TOKEN_MARKER);
    expect(document.body.innerHTML).not.toContain(TOKEN_MARKER);
    expect(socket.frames.map((frame) => (typeof frame === 'string' ? frame : new TextDecoder().decode(frame))).join('')).not.toContain(TOKEN_MARKER);
  });
});
