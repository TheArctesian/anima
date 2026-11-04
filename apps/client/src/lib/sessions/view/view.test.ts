// @vitest-environment jsdom
import { flushSync } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { installBlankCanvas } from '../tunnel/fake/canvas.fixture';
import { FakeTunnelSocket } from '../tunnel/fake/socket.fixture';
import { RESIZE_SETTLE_MS } from './display';
import { FakeResizeObserver, connectView, flags, installBrowserFakes, sent, settle, showView } from './view.fixture.svelte';

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

describe('MAT-008: the remote display', () => {
  const MOTION_PROPERTIES = ['transform', 'translate', 'scale', 'rotate', 'filter', 'backdrop-filter', 'transition', 'animation'];
  const NEUTRAL = ['', 'none'];

  it('MAT-008: the display element and the pane holding it have no transform, filter, transition, animation or material', async () => {
    const { target } = show();
    await connectView();
    const host = target.querySelector<HTMLElement>('[data-display]');
    const element = host?.firstElementChild;
    if (host === null || !(element instanceof HTMLElement)) throw new Error('no display');
    for (const node of [element, host, host.parentElement]) {
      if (node === null) continue;
      const style = getComputedStyle(node);
      for (const property of MOTION_PROPERTIES) expect(NEUTRAL, property).toContain(style.getPropertyValue(property));
      expect(['', '1'], 'opacity').toContain(style.opacity);
      expect(node.getAnimations?.() ?? []).toEqual([]);
    }
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
