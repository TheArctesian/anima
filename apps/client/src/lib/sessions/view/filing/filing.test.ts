// @vitest-environment jsdom
import { flushSync } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { Hub } from '$lib/hub/hub';
import { FAKE_FILES_SOCKET, FakeFilesSocket } from '../../files/link/fake/socket.fixture';
import { installBlankCanvas } from '../../tunnel/fake/canvas.fixture';
import { FakeTerminalSocket } from '../../tunnel/terminal/fake/socket.fixture';
import { FakeResizeObserver, connectView, flags, installBrowserFakes, settle, showView } from '../view.fixture.svelte';

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

describe('RA-032: the Files toggle', () => {
  it('RA-032: the Files button is disabled until the socket is open, like Find, and then reports it is collapsed', async () => {
    const { target } = show();
    const files = () => [...target.querySelectorAll('button')].find((each) => each.textContent.includes('Files'));
    expect(files()?.disabled).toBe(true);
    FakeResizeObserver.resize(800, 600);
    await settle();
    FakeTerminalSocket.latest().open();
    flushSync();
    expect(files()?.disabled).toBe(false);
    expect(files()?.getAttribute('aria-expanded')).toBe('false');
  });
});


/** A hub that answers every `openFiles` and keeps the session ids it was asked for. */
function filesHub() {
  const asked: unknown[] = [];
  const hub = {
    address: { origin: 'https://hub.anima.ing', http: 'https://hub.anima.ing/graphql', socket: 'wss://hub.anima.ing/graphql' },
    request: (_document: unknown, variables: { sessionId: string }) => {
      asked.push(variables.sessionId);
      const openFiles = { sessionId: variables.sessionId, token: 'files-token', tunnelPath: '/files', expiresAt: '2026-11-06T10:00:30Z' };
      return Promise.resolve({ ok: true, value: { openFiles } });
    },
  } as unknown as Hub;
  return { hub, asked };
}

describe('RA-032: the file panel follows its session', () => {
  const filesButton = (target: HTMLElement) => [...target.querySelectorAll('button')].find((each) => each.textContent.includes('Files'));

  it('RA-032: the panel comes before the pane for Tab; ending the session closes its socket; after Reconnect the toggle is off and the new session id is used', async () => {
    const original = globalThis.WebSocket;
    globalThis.WebSocket = FAKE_FILES_SOCKET;
    cleanups.push(() => {
      globalThis.WebSocket = original;
    });
    FakeFilesSocket.reset();
    const { hub, asked } = filesHub();
    const view = showView((open) => open, hub);
    cleanups.push(view.close);
    const first = await connectView();
    filesButton(view.target)?.click();
    await settle();
    expect(asked).toEqual(['session-1']);
    const panel = view.target.querySelector('.files');
    const pane = view.target.querySelector('.pane');
    expect(panel?.compareDocumentPosition(pane ?? view.target)).toBe(Node.DOCUMENT_POSITION_FOLLOWING);
    const files = FakeFilesSocket.latest();
    files.open();
    first.closeWith(1000);
    await settle();
    expect(files.closedByClient).toBe(1000);
    expect(view.target.querySelector('.files')).toBeNull();
    [...view.target.querySelectorAll('button')].find((each) => each.textContent.includes('Reconnect'))?.click();
    await settle();
    FakeTerminalSocket.latest().open();
    flushSync();
    expect(filesButton(view.target)?.getAttribute('aria-expanded')).toBe('false');
    filesButton(view.target)?.click();
    await settle();
    expect(asked).toEqual(['session-1', 'session-2']);
  });
});
