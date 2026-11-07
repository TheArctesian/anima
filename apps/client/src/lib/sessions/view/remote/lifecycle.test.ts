// @vitest-environment jsdom
import { flushSync } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { installBlankCanvas } from '../../tunnel/fake/canvas.fixture';
import { FakeTunnelSocket } from '../../tunnel/fake/socket.fixture';
import { FakeResizeObserver, TOKEN_MARKER, installBrowserFakes, settle } from '../view.fixture.svelte';
import { connectRemote, remoteFlags, showRemote } from './remote.fixture.svelte';

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
  vi.restoreAllMocks();
  vi.useRealTimers();
});

function show(...args: Parameters<typeof showRemote>) {
  const view = showRemote(...args);
  cleanups.push(view.close);
  return view;
}

const stepMarks = (target: HTMLElement) =>
  [...target.querySelectorAll<HTMLElement>('[data-step]')].map((step) => `${step.dataset.step ?? ''}:${step.dataset.mark ?? ''}`);
const wakeOf = (target: HTMLElement) => {
  const line = target.querySelector<HTMLElement>('.wake');
  return line === null ? undefined : { mark: line.dataset.wake, text: [...line.querySelectorAll('.label, .sr')].map((span) => span.textContent).join(' ') };
};
const reasonOf = (target: HTMLElement) => target.querySelector('[data-reason]')?.textContent;
const certificateModals = () => document.querySelectorAll('[role="alertdialog"]').length;
const reconnectOf = (target: HTMLElement) => [...target.querySelectorAll('button')].find((button) => button.textContent.includes('Reconnect'));

describe('start and connection steps (Section 5.7.3)', () => {
  it.each(['RDP', 'VNC'] as const)('%s: the first pane measure starts one session for the protocol, labelled by it', async (protocol) => {
    const { calls, target } = show(protocol);
    expect(calls.startSession).toEqual([]);
    expect(target.querySelector('section')?.getAttribute('aria-label')).toBe(`${protocol} session on desk`);
    FakeResizeObserver.resize(800, 600);
    FakeResizeObserver.resize(810, 600);
    await settle();
    expect(calls.startSession.map(([device, asked]) => [device, asked])).toEqual([['device-1', protocol]]);
  });

  it('each mark changes when its step ends, and the display shows only after the first frame', async () => {
    const { target, heard } = show();
    expect(stepMarks(target)).toEqual(['request:active', 'gateway:waiting', 'device:waiting', 'ready:waiting']);
    FakeResizeObserver.resize(800, 600);
    await settle();
    expect(stepMarks(target)).toEqual(['request:done', 'gateway:active', 'device:waiting', 'ready:waiting']);
    const socket = FakeTunnelSocket.latest();
    socket.open();
    flushSync();
    expect(stepMarks(target)).toEqual(['request:done', 'gateway:done', 'device:active', 'ready:waiting']);
    expect(target.querySelector('[data-display]')).toBeNull();
    expect(heard.clients).toHaveLength(1);
    socket.sync();
    flushSync();
    expect(stepMarks(target)).toEqual([]);
    expect(target.querySelector('[data-display]')?.firstElementChild).toBe(heard.clients[0]?.getDisplay().getElement());
  });

  it('a start that stops marks the step it stopped at', async () => {
    const { target } = show();
    FakeResizeObserver.resize(800, 600);
    await settle();
    FakeTunnelSocket.latest().closeWith(1006);
    await settle();
    expect(stepMarks(target)).toEqual(['request:done', 'gateway:failed', 'device:waiting', 'ready:waiting']);
    expect(reasonOf(target)).toBe('Could not open the session.');
  });
});

describe('wake line (RA-031)', () => {
  it('RA-031: a wake packet shows "Waking desk" under the four steps, done when the socket opens', async () => {
    const { target, answers } = show();
    answers.wakeSent = true;
    FakeResizeObserver.resize(800, 600);
    await settle();
    expect(stepMarks(target)).toEqual(['request:done', 'gateway:active', 'device:waiting', 'ready:waiting']);
    expect(wakeOf(target)).toEqual({ mark: 'active', text: 'Waking desk in progress' });
    FakeTunnelSocket.latest().open();
    flushSync();
    expect(wakeOf(target)).toEqual({ mark: 'done', text: 'Waking desk done' });
  });

  it('RA-031: a tunnel that is refused after a waking ticket still shows the wake line as stopped', async () => {
    const { target, answers } = show();
    answers.wakeSent = true;
    FakeResizeObserver.resize(800, 600);
    await settle();
    FakeTunnelSocket.latest().closeWith(1006);
    await settle();
    expect(wakeOf(target)).toEqual({ mark: 'failed', text: 'Waking desk stopped' });
  });

  it('RA-031: Reconnect after a waking attempt: the line clears and the new tunnel has the 15 s limit', async () => {
    const { target, answers } = show();
    answers.wakeSent = true;
    FakeResizeObserver.resize(800, 600);
    await settle();
    const first = FakeTunnelSocket.latest();
    vi.advanceTimersByTime(100_000);
    expect(first.closedByClient).toBeUndefined();
    first.closeWith(1006);
    await settle();
    answers.wakeSent = false;
    reconnectOf(target)?.click();
    await settle();
    const second = FakeTunnelSocket.latest();
    expect(second).not.toBe(first);
    expect(wakeOf(target)).toEqual({ mark: undefined, text: '' });
    vi.advanceTimersByTime(14_000);
    expect(second.closedByClient).toBeUndefined();
    vi.advanceTimersByTime(1_000);
    expect(second.closedByClient).toBeDefined();
  });

  it('RA-031: with no wake packet the slot stays and is empty', async () => {
    const { target } = show();
    FakeResizeObserver.resize(800, 600);
    await settle();
    expect(wakeOf(target)).toEqual({ mark: undefined, text: '' });
  });
});

describe('ended panel and Reconnect (RA-007)', () => {
  it('RA-007: the panel shows the recorded reason, which wins over the gateway error (D-4)', async () => {
    const { target, answers, calls, heard } = show();
    const socket = await connectRemote();
    answers.endReason = 'REMOTE_CLOSED';
    socket.gatewayError(0x0301);
    socket.closeWith(1011);
    await settle();
    expect(calls.endSession).toEqual(['session-1']);
    expect(reasonOf(target)).toBe('The device closed the session.');
    expect(heard.ended.map((ending) => ending.reason)).toEqual(['REMOTE_CLOSED']);
    expect(certificateModals()).toBe(0);
    expect(heard.gone).toBe(1);
    expect(target.querySelector('[data-display]')).toBeNull();
  });

  it('RA-007: a refused sign-in with no recorded reason reads as AUTH_FAILED from the gateway status', async () => {
    const { target } = show();
    const socket = await connectRemote();
    socket.gatewayError(0x0301);
    socket.closeWith(1000);
    await settle();
    expect(reasonOf(target)).toBe('The device refused the sign-in, or no credential is stored for it.');
  });

  it('RA-007: Reconnect asks for one new ticket and opens a new tunnel with a new token in the same pane', async () => {
    const { target, calls } = show('VNC');
    const first = await connectRemote();
    first.disconnect();
    first.closeWith(1000);
    await settle();
    const reconnect = reconnectOf(target);
    reconnect?.click();
    reconnect?.click();
    await settle();
    expect(calls.startSession.map(([, protocol]) => protocol)).toEqual(['VNC', 'VNC']);
    expect(FakeTunnelSocket.made).toHaveLength(2);
    const [old, fresh] = FakeTunnelSocket.made.map((socket) => new URL(socket.url).searchParams.get('token'));
    expect(fresh).not.toBe(old);
    FakeTunnelSocket.latest().open();
    FakeTunnelSocket.latest().sync();
    flushSync();
    expect(target.querySelectorAll('[data-display]')).toHaveLength(1);
  });

});

describe('certificate changes and refusals (K4 §1, §3, §5)', () => {
  it.each([
    ['close 4003', null],
    ['the recorded reason', 'SERVER_CERTIFICATE_MISMATCH'],
  ] as const)('K4 §3, §5: %s says the RDP certificate changed and opens the certificate warning once', async (_name, recorded) => {
    const { target, answers } = show();
    const socket = await connectRemote();
    answers.endReason = recorded;
    socket.gatewayError(0x0303);
    socket.closeWith(recorded === null ? 4003 : 1011);
    await settle();
    expect(reasonOf(target)).toBe("The device's RDP certificate has changed, so Anima did not connect.");
    expect(certificateModals()).toBe(1);
  });

  it('K4 §5: close 4003 opens the certificate warning even when the hub recorded another reason', async () => {
    const { answers } = show();
    const socket = await connectRemote();
    answers.endReason = 'GATEWAY_ERROR';
    socket.closeWith(4003);
    await settle();
    expect(certificateModals()).toBe(1);
  });

  it('K4 §1: a CONFLICT for RDP points to the connection test, in fixed text, not the hub message', async () => {
    const { target, answers } = show();
    answers.refuse = 'CONFLICT';
    FakeResizeObserver.resize(800, 600);
    await settle();
    expect(reasonOf(target)).toBe("Could not open the session. The device's RDP certificate is not accepted yet: run the connection test first.");
    expect(target.textContent).not.toContain('HUB-TEXT-MARKER');
    expect(certificateModals()).toBe(0);
    expect(FakeTunnelSocket.made).toEqual([]);
  });

  it('K4 §1: a CONFLICT for VNC has its own fixed text, with no host key or certificate in it', async () => {
    const { target, answers } = show('VNC');
    answers.refuse = 'CONFLICT';
    FakeResizeObserver.resize(800, 600);
    await settle();
    const text = reasonOf(target) ?? '';
    expect(text).toBe('Could not open the session. The hub refused it for this device: check its VNC settings, then run the connection test.');
    expect(text).not.toMatch(/host key|certificate|HUB-TEXT-MARKER/);
  });

  it('closing the view closes its tunnel and ignores the end that follows', async () => {
    const { close, calls, heard } = show();
    const socket = await connectRemote();
    await close();
    await settle();
    expect(socket.closedByClient).toBeDefined();
    expect(calls.endSession).toEqual([]);
    expect(heard.ended).toEqual([]);
  });

  it('a view closed while the hub answers closes the tunnel it then gets', async () => {
    const { close } = show();
    FakeResizeObserver.resize(800, 600);
    await close();
    await settle();
    expect(FakeTunnelSocket.latest().closedByClient).toBeDefined();
  });
});

describe('D-6: only the display, keys, pointer and clipboard', () => {
  it('D-6: a `filesystem` from the gateway is ignored, and no audio input stream is opened', async () => {
    const { target } = show();
    const socket = await connectRemote();
    socket.instruction('filesystem', 0, 'Shared Drive');
    flushSync();
    expect(sentOf(socket).filter(([opcode]) => ['audio', 'get', 'put', 'file'].includes(opcode ?? ''))).toEqual([]);
    expect(target.querySelectorAll('[data-display]')).toHaveLength(1);
  });

  it('LOG-007: a whole session makes no console call and never shows the token', async () => {
    const spies = Object.keys(console)
      .filter((name) => typeof (console as unknown as Record<string, unknown>)[name] === 'function')
      .map((name) => vi.spyOn(console as unknown as Record<string, () => void>, name).mockImplementation(() => undefined));
    const { target } = show();
    const socket = await connectRemote();
    socket.clipboard('CLIP-MARKER-91c2');
    socket.gatewayError(0x0301);
    socket.closeWith(1011);
    await settle();
    const calls = spies.flatMap((spy) => spy.mock.calls);
    for (const spy of spies) spy.mockRestore();
    expect(calls).toEqual([]);
    expect(document.body.innerHTML).not.toContain(TOKEN_MARKER);
    expect(target.textContent).not.toContain('CLIP-MARKER');
  });
});

function sentOf(socket: FakeTunnelSocket): string[][] {
  return socket.sentInstructions();
}
