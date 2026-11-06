// @vitest-environment jsdom
import { flushSync } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { installBlankCanvas } from '../../tunnel/fake/canvas.fixture';
import { FakeTunnelSocket } from '../../tunnel/fake/socket.fixture';
import { FakeResizeObserver, TOKEN_MARKER, installBrowserFakes, settle } from '../view.fixture.svelte';
import { connectRemote, keyOn, keysOf, remoteFlags, remotePaneOf, showRemote } from './remote.fixture.svelte';

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

/** Start the session and open its tunnel, without the first frame. */
async function opened(...args: Parameters<typeof showRemote>) {
  const view = show(...args);
  FakeResizeObserver.resize(800, 600);
  await settle();
  const socket = FakeTunnelSocket.latest();
  socket.open();
  flushSync();
  return { ...view, socket };
}

const prompt = () => document.querySelector<HTMLElement>('[role="dialog"]');
const fieldNames = () => [...(prompt()?.querySelectorAll<HTMLInputElement>('input[name]') ?? [])].map((input) => input.name);
const buttonNamed = (root: ParentNode | null, text: string) => [...(root?.querySelectorAll('button') ?? [])].find((button) => button.textContent.trim() === text);
const reasonOf = (target: HTMLElement) => target.querySelector('[data-reason]')?.textContent;

function type(name: string, value: string) {
  const input = prompt()?.querySelector<HTMLInputElement>(`input[name="${name}"]`);
  if (input === null || input === undefined) throw new Error(`no field ${name}`);
  input.value = value;
  input.dispatchEvent(new Event('input', { bubbles: true }));
}

/** The `argv` streams the client opened, as `[name, text]`. */
function argvOf(socket: FakeTunnelSocket): [string, string][] {
  const instructions = socket.sentInstructions();
  const opened = instructions.filter(([opcode]) => opcode === 'argv');
  return opened.map(([, stream, , name]) => {
    const blobs = instructions.filter(([opcode, index]) => opcode === 'blob' && index === stream).map(([, , data]) => data ?? '');
    return [name ?? '', Buffer.concat(blobs.map((blob) => Buffer.from(blob, 'base64'))).toString('utf8')];
  });
}

describe('GW-015: the gateway asks for a credential', () => {
  it('GW-015: a `required` after the start answered shows only the asked fields, and Sign in sends each as argv', async () => {
    const { socket } = await opened('VNC');
    socket.instruction('required', 'password');
    flushSync();
    expect(fieldNames()).toEqual(['password']);
    type('password', 'PASSWORD-MARKER');
    prompt()?.querySelector('form')?.requestSubmit();
    await settle();
    expect(argvOf(socket)).toEqual([['password', 'PASSWORD-MARKER']]);
    expect(prompt()).toBeNull();
    expect(document.body.innerHTML).not.toContain('PASSWORD-MARKER');
    socket.sync();
    flushSync();
    expect(document.querySelectorAll('[data-display]')).toHaveLength(1);
  });

  it('GW-015: a `required` that arrives before the start has answered is not lost', async () => {
    const view = show('RDP');
    let release: () => void = () => undefined;
    view.answers.policyGate = new Promise((resolve) => (release = resolve));
    FakeResizeObserver.resize(800, 600);
    await settle();
    const socket = FakeTunnelSocket.latest();
    socket.open();
    socket.instruction('required', 'password', 'username');
    flushSync();
    expect(prompt()).toBeNull();
    release();
    await settle();
    expect(fieldNames()).toEqual(['username', 'password']);
  });
});

describe('GW-015: each `required` is answered on its own', () => {
  it('GW-015: a second `required` that arrives while the password is being saved gets its own prompt', async () => {
    const view = await opened('RDP');
    let release: () => void = () => undefined;
    view.answers.credentialGate = new Promise((resolve) => (release = resolve));
    view.socket.instruction('required', 'password');
    flushSync();
    type('password', 'first');
    prompt()?.querySelector<HTMLInputElement>('input[type="checkbox"]')?.click();
    prompt()?.querySelector('form')?.requestSubmit();
    await settle();
    expect(view.calls.setCredential).toEqual([['RDP', 'device-1', 'first']]);
    view.socket.instruction('required', 'username', 'password');
    flushSync();
    release();
    await settle();
    expect(fieldNames()).toEqual(['username', 'password']);
    expect(argvOf(view.socket)).toEqual([['password', 'first']]);
  });

  it('GW-015: a `required` on the old socket after Reconnect shows no prompt', async () => {
    const { target } = show('RDP');
    const old = await connectRemote();
    old.closeWith(1000);
    await settle();
    const reconnect = [...target.querySelectorAll('button')].find((button) => button.textContent.includes('Reconnect'));
    reconnect?.click();
    await settle();
    expect(FakeTunnelSocket.made).toHaveLength(2);
    old.instruction('required', 'password');
    flushSync();
    expect(prompt()).toBeNull();
  });
});

describe('GW-015: a sign-in that ends the session', () => {
  it('GW-015: Cancel sends nothing and ends the session with fixed text', async () => {
    const { socket, target, calls } = await opened('RDP');
    socket.instruction('required', 'username', 'password');
    flushSync();
    const cancel = buttonNamed(prompt(), 'Cancel');
    expect(cancel).toBeDefined();
    cancel?.click();
    await settle();
    expect(argvOf(socket)).toEqual([]);
    expect(socket.closedByClient).toBeDefined();
    expect(calls.endSession).toEqual(['session-1']);
    expect(reasonOf(target)).toBe('You cancelled the sign-in, so the session ended.');
    expect(prompt()).toBeNull();
  });

  it('GW-015: after a cancelled sign-in, Reconnect starts afresh and a later end shows its own reason', async () => {
    const { socket, target } = await opened('RDP');
    socket.instruction('required', 'password');
    flushSync();
    buttonNamed(prompt(), 'Cancel')?.click();
    await settle();
    const reconnect = [...target.querySelectorAll('button')].find((button) => button.textContent.includes('Reconnect'));
    expect(reconnect).toBeDefined();
    reconnect?.click();
    const fresh = await connectRemote();
    fresh.closeWith(1000);
    await settle();
    expect(reasonOf(target)).toBe('The device closed the session.');
  });

  it('GW-015: a `required` naming a parameter the prompt may not ask ends the session, asking nothing', async () => {
    const { socket, target } = await opened('VNC');
    socket.instruction('required', 'password', 'hostname');
    await settle();
    expect(prompt()).toBeNull();
    expect(socket.closedByClient).toBeDefined();
    expect(reasonOf(target)).toBe('The device asked for a sign-in Anima cannot give, so the session ended.');
    expect(target.textContent).not.toContain('hostname');
  });
});

describe('GW-015: when the prompt shows and what it holds', () => {
  it('SEC-007: a hidden tab shows no prompt until it is shown again', async () => {
    remoteFlags.visible = false;
    const { socket } = await opened('RDP');
    socket.instruction('required', 'password');
    flushSync();
    expect(prompt()).toBeNull();
    remoteFlags.visible = true;
    flushSync();
    expect(fieldNames()).toEqual(['password']);
  });

  it('D-8: a prompt opened in a live session keeps the focus when the window gets it back', async () => {
    show('RDP');
    const socket = await connectRemote();
    socket.instruction('required', 'password');
    flushSync();
    prompt()?.querySelector<HTMLInputElement>('input[name="password"]')?.focus();
    remoteFlags.focused = false;
    flushSync();
    remoteFlags.focused = true;
    flushSync();
    expect(prompt()?.contains(document.activeElement)).toBe(true);
  });

  it('D-8: while the prompt is open the pane takes no key, and it does again once it was answered', async () => {
    const { target } = show('RDP');
    const socket = await connectRemote();
    socket.instruction('required', 'password');
    flushSync();
    const pane = remotePaneOf(target);
    pane.focus();
    keyOn(pane, 'keydown', { key: 'Enter', code: 'Enter', keyCode: 13 });
    expect(keysOf(socket)).toEqual([]);
    prompt()?.querySelector('form')?.requestSubmit();
    await settle();
    keyOn(pane, 'keydown', { key: 'Enter', code: 'Enter', keyCode: 13 });
    expect(keysOf(socket)).toEqual([[1, 0xff0d]]);
  });
});

describe('D-7: the RDP certificate changed', () => {
  it('D-7: close 4003 opens the warning; Accept records exactly the shown fingerprint and reconnects with a new ticket', async () => {
    const { calls } = show('RDP');
    const first = await connectRemote();
    first.gatewayError(0x0303);
    first.closeWith(4003);
    await settle();
    const modal = document.querySelector('[role="alertdialog"]');
    expect(modal?.textContent).toContain('sha256:6b:1f:9e:00:aa:bb:cc:dd');
    const accept = buttonNamed(modal, 'Accept new certificate');
    expect(accept).toBeDefined();
    accept?.click();
    await settle();
    expect(calls.acceptServerCertificate).toEqual([['device-1', 'sha256:6b:1f:9e:00:aa:bb:cc:dd']]);
    expect(calls.startSession).toHaveLength(2);
    const [old, fresh] = FakeTunnelSocket.made.map((socket) => new URL(socket.url).searchParams.get('token'));
    expect(fresh).toBe(`${TOKEN_MARKER}-2`);
    expect(fresh).not.toBe(old);
  });

  it('D-7: the warning of a hidden tab waits until the tab is shown, and closing it keeps the ended panel', async () => {
    const { target, calls } = show('RDP');
    const socket = await connectRemote();
    remoteFlags.visible = false;
    flushSync();
    socket.closeWith(4003);
    await settle();
    expect(document.querySelector('[role="alertdialog"]')).toBeNull();
    remoteFlags.visible = true;
    await settle();
    const cancel = buttonNamed(document.querySelector('[role="alertdialog"]'), 'Cancel');
    expect(cancel).toBeDefined();
    cancel?.click();
    await settle();
    expect(document.querySelector('[role="alertdialog"]')).toBeNull();
    expect(calls.acceptServerCertificate).toEqual([]);
    expect(calls.startSession).toHaveLength(1);
    expect(reasonOf(target)).toBe("The device's RDP certificate has changed, so Anima did not connect.");
  });
});
