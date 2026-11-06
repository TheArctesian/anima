// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { SessionEndReason } from '../ending';
import { installBlankCanvas } from '../tunnel/fake/canvas.fixture';
import { FakeTerminalSocket } from '../tunnel/terminal/fake/socket.fixture';
import { FakeResizeObserver, connectView, flags, installBrowserFakes, settle, showView } from './view.fixture.svelte';

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

/** A session whose hub (the fake) ends it as `end` and `code` say, with `recorded` as the hub's row (K4 §8.6). */
async function endedBy(end: SessionEndReason | undefined, code: number, recorded: SessionEndReason | null = null) {
  const view = showView();
  cleanups.push(view.close);
  view.answers.endReason = recorded;
  const socket = await connectView();
  if (end !== undefined) socket.end(end);
  socket.closeWith(code);
  await settle();
  return { ...view, text: view.target.querySelector('[data-reason]')?.textContent, hostKeyModal: document.querySelector('[role="alertdialog"]') };
}

describe('each end of a session (K4 §8.6, RA-007)', () => {
  it.each<[string, SessionEndReason | undefined, number, string]>([
    ['the shell exits (1000)', 'REMOTE_CLOSED', 1000, 'The device closed the session.'],
    ['a close with no end message (1000)', undefined, 1000, 'The device closed the session.'],
    ['the hub loses its SSH connection (1011)', 'GATEWAY_ERROR', 1011, 'The gateway lost the connection to the device.'],
    ['the device refuses the credential (1011)', 'AUTH_FAILED', 1011, 'The device refused the sign-in.'],
    ['a protocol error (1002)', 'PROTOCOL_ERROR', 1002, 'The hub ended the session because Anima sent an instruction it could not read.'],
    ['client silence (4002)', 'TIMEOUT', 4002, 'The session timed out because the hub heard nothing from Anima.'],
    ['a close of 4002 with no end message', undefined, 4002, 'The session timed out because the hub heard nothing from Anima.'],
    ['the hub shuts down (1001)', 'HUB_SHUTDOWN', 1001, 'The hub shut down.'],
  ])('RA-007: %s gives its ended panel', async (_name, end, code, expected) => {
    const { text, hostKeyModal } = await endedBy(end, code);
    expect(text).toBe(expected);
    expect(hostKeyModal).toBeNull();
  });

  it('SEC-007: a host key mismatch (end message and 4001) opens the host key warning, not a plain ended panel only', async () => {
    const { text, hostKeyModal } = await endedBy('HOST_KEY_MISMATCH', 4001);
    expect(text).toBe("The device's host key has changed, so Anima did not connect.");
    expect(hostKeyModal?.textContent).toContain('host key');
  });

  it('SEC-007: the close code 4001 alone opens the host key warning', async () => {
    const { hostKeyModal } = await endedBy(undefined, 4001);
    expect(hostKeyModal).not.toBeNull();
  });

  it('SEC-007: a recorded HOST_KEY_MISMATCH opens the warning although the socket closed with another code', async () => {
    const { hostKeyModal } = await endedBy(undefined, 1000, 'HOST_KEY_MISMATCH');
    expect(hostKeyModal).not.toBeNull();
  });

  it('D-4: the hub row from endSession wins over the end message', async () => {
    const { text } = await endedBy('GATEWAY_ERROR', 1011, 'AUTH_FAILED');
    expect(text).toBe('The device refused the sign-in.');
  });

  it('RA-007: a refusal before the upgrade (503, seen as 1006) says the session could not be opened, with the hub reason', async () => {
    const view = showView();
    cleanups.push(view.close);
    view.answers.endReason = 'GATEWAY_ERROR';
    FakeResizeObserver.resize(800, 600);
    await settle();
    FakeTerminalSocket.latest().closeWith(1006);
    await settle();
    expect(view.target.querySelector('[data-reason]')?.textContent).toBe(
      'Could not open the session. The gateway lost the connection to the device.',
    );
  });

  it('RA-007: an end message with a reason this client does not know is ignored; the close code decides', async () => {
    const view = showView();
    cleanups.push(view.close);
    const socket = await connectView();
    socket.end('SOMETHING_NEWER');
    socket.closeWith(1011);
    await settle();
    expect(view.target.querySelector('[data-reason]')?.textContent).toBe('The gateway lost the connection to the device.');
  });
});
