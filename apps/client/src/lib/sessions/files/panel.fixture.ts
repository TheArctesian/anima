// Test support: the file panel mounted on a fake ticket and a fake socket, and the queries tests make of it.
import { flushSync, mount, unmount } from 'svelte';
import type { HubResult } from '$lib/hub/hub';
import type { LoadingClock } from '$lib/motion/loading/loading';
import type { FilesApi, FilesTicket } from './api';
import FilesPanel from './FilesPanel.svelte';
import type { FileEntry } from './link/codec';
import { settle } from './link/fake/link.fixture';
import { FAKE_FILES_SOCKET, FakeFilesSocket } from './link/fake/socket.fixture';

export const HOME = '/home/ada';
export const entry = (name: string, kind: FileEntry['kind'], size: number | null = null, modifiedAt: string | null = null): FileEntry => ({ name, kind, size, modifiedAt });

let cleanups: (() => unknown)[] = [];

/** Stands in for the socket of a panel that never made one. */
let none: FakeFilesSocket;

/** Call before each test. */
export function resetPanels(): void {
  FakeFilesSocket.reset();
  none = new FakeFilesSocket('none');
  FakeFilesSocket.reset();
}

/** Call after each test: unmounts what the test mounted. */
export async function removePanels(): Promise<void> {
  for (const cleanup of cleanups.reverse()) await cleanup();
  cleanups = [];
  document.body.innerHTML = '';
}

export interface Shown {
  target: HTMLElement;
  socket: FakeFilesSocket;
  saved: { name: string; blob: Blob }[];
  tickets: string[];
  close: () => Promise<void>;
}

/** Mount the panel on a fake ticket and socket; `ticket` can be a failure. */
export async function show(ticket: HubResult<FilesTicket> | Promise<HubResult<FilesTicket>> | undefined = undefined, clock?: LoadingClock): Promise<Shown> {
  const tickets: string[] = [];
  const api: FilesApi = {
    openFiles: (sessionId) => {
      tickets.push(sessionId);
      return Promise.resolve(ticket ?? { ok: true, value: { sessionId, token: 'TOKEN-MARKER', tunnelPath: '/files', expiresAt: '2026-11-05T10:00:30Z' } });
    },
  };
  const saved: Shown['saved'] = [];
  const target = document.createElement('div');
  document.body.append(target);
  const component = mount(FilesPanel, {
    target,
    props: { sessionId: 'session-1', api, origin: 'https://hub.anima.ing', WebSocket: FAKE_FILES_SOCKET, clock, save: (name, blob) => saved.push({ name, blob }) },
  });
  let closed = false;
  const close = async () => {
    if (closed) return;
    closed = true;
    await unmount(component);
  };
  cleanups.push(close);
  await settle();
  const socket = FakeFilesSocket.made.at(-1) ?? none;
  return { target, socket, saved, tickets, close };
}

/** Open the link and answer the home listing. */
export async function listed(entries: FileEntry[], path = HOME): Promise<Shown> {
  const shown = await show();
  shown.socket.open();
  await settle();
  shown.socket.text({ type: 'entries', path, entries });
  await settle();
  flushSync();
  return shown;
}

export const button = (target: HTMLElement, label: string): HTMLButtonElement => {
  const found = [...target.querySelectorAll('button')].find((each) => (each.getAttribute('aria-label') ?? each.textContent).trim() === label);
  if (found === undefined) throw new Error(`no button ${label}`);
  return found;
};
export const names = (target: HTMLElement) => [...target.querySelectorAll('.entry .name')].map((each) => each.textContent);
/** Whether the button is off: it stays focusable, so it says so with `aria-disabled`. */
export const off = (element: HTMLElement): boolean => element.getAttribute('aria-disabled') === 'true';
export const status = (target: HTMLElement) => target.querySelector('.status')?.textContent;
export const bar = (target: HTMLElement) => target.querySelector('[role="progressbar"]');

export async function click(element: HTMLElement) {
  element.click();
  await settle();
  flushSync();
}

export const file = (name: string, bytes = 5) => new File([new Uint8Array(bytes)], name);

export function pick(target: HTMLElement, chosen: File) {
  const input = target.querySelector('input[type="file"]');
  if (!(input instanceof HTMLInputElement)) throw new Error('no picker');
  Object.defineProperty(input, 'files', { configurable: true, value: [chosen] });
  input.dispatchEvent(new Event('change', { bubbles: true }));
}
