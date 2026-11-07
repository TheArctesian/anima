// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { HubResult } from '$lib/hub/hub';
import { UNREACHABLE } from '$lib/hub/error';
import type { FilesApi, FilesTicket } from './api';
import FilesPanel from './FilesPanel.svelte';
import { FAKE_FILES_SOCKET, FakeFilesSocket } from './link/fake/socket.fixture';
import { settle } from './link/fake/link.fixture';
import type { FileEntry } from './link/codec';

const HOME = '/home/ada';
const entry = (name: string, kind: FileEntry['kind'], size: number | null = null, modifiedAt: string | null = null): FileEntry => ({ name, kind, size, modifiedAt });

let cleanups: (() => unknown)[] = [];

/** Stands in for the socket of a panel that never made one. */
let none: FakeFilesSocket;

beforeEach(() => {
  FakeFilesSocket.reset();
  none = new FakeFilesSocket('none');
  FakeFilesSocket.reset();
});

afterEach(async () => {
  for (const cleanup of cleanups.reverse()) await cleanup();
  cleanups = [];
  document.body.innerHTML = '';
});

interface Shown {
  target: HTMLElement;
  socket: FakeFilesSocket;
  saved: { name: string; blob: Blob }[];
  tickets: string[];
  close: () => Promise<void>;
}

/** Mount the panel on a fake ticket and socket; `ticket` can be a failure. */
async function show(ticket: HubResult<FilesTicket> | undefined = undefined): Promise<Shown> {
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
    props: { sessionId: 'session-1', api, origin: 'https://hub.anima.ing', WebSocket: FAKE_FILES_SOCKET, save: (name, blob) => saved.push({ name, blob }) },
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
async function listed(entries: FileEntry[], path = HOME): Promise<Shown> {
  const shown = await show();
  shown.socket.open();
  await settle();
  shown.socket.text({ type: 'entries', path, entries });
  await settle();
  flushSync();
  return shown;
}

const button = (target: HTMLElement, label: string): HTMLButtonElement => {
  const found = [...target.querySelectorAll('button')].find((each) => (each.getAttribute('aria-label') ?? each.textContent).trim() === label);
  if (found === undefined) throw new Error(`no button ${label}`);
  return found;
};
const names = (target: HTMLElement) => [...target.querySelectorAll('.entry .name')].map((each) => each.textContent);
const status = (target: HTMLElement) => target.querySelector('.status')?.textContent;
const bar = (target: HTMLElement) => target.querySelector('[role="progressbar"]');

async function click(element: HTMLElement) {
  element.click();
  await settle();
  flushSync();
}

describe('RA-032: opening and listing', () => {
  it('RA-032: opens a ticket for the session, then lists the home directory and shows the resolved path', async () => {
    const { target, socket, tickets } = await show();
    expect(tickets).toEqual(['session-1']);
    expect(socket.url).toContain('wss://hub.anima.ing/files?token=');
    socket.open();
    await settle();
    expect(socket.messages()).toEqual([{ type: 'list', path: '.' }]);
    socket.text({ type: 'entries', path: HOME, entries: [] });
    await settle();
    flushSync();
    expect(target.querySelector('.where')?.textContent).toBe(HOME);
  });

  it('RA-032: directories come first, each group by name, with size and time, and a dash for what the device left out', async () => {
    const { target } = await listed([
      entry('b.txt', 'file', 1536, '2026-11-05T10:00:00Z'),
      entry('zeta', 'dir'),
      entry('a.txt', 'file'),
      entry('alpha', 'dir'),
    ]);
    expect(names(target)).toEqual(['alpha', 'zeta', 'a.txt', 'b.txt']);
    const rows = [...target.querySelectorAll('.entry')];
    const meta = (row: Element | undefined) => [...(row?.querySelectorAll('.meta') ?? [])].map((each) => each.textContent);
    expect(meta(rows[0])).toEqual(['—', '—']);
    expect(meta(rows[3])[0]).toBe('1.5 KiB');
    expect(meta(rows[3])[1]).not.toBe('—');
    expect(meta(rows[2])).toEqual(['—', '—']);
  });

  it('RA-032: names from the device are only text, never markup', async () => {
    const hostile = '<img src=x onerror=alert(1)>\u0007';
    const { target } = await listed([entry(hostile, 'file')]);
    expect(target.querySelector('img')).toBeNull();
    expect(names(target)).toEqual([hostile]);
  });

  it('RA-032: a ticket the hub refuses shows fixed text and never the hub message or the token', async () => {
    const { target, socket } = await show({ ok: false, error: UNREACHABLE });
    expect(status(target)).toBe('Could not open the file connection. Check that the session is still running.');
    expect(FakeFilesSocket.made).toHaveLength(0);
    expect(socket.frames).toEqual([]);
    expect(target.innerHTML).not.toContain('TOKEN-MARKER');
  });

  it('RA-032: a refused upgrade and a lost link show the fixed end text and disable the actions', async () => {
    const { target, socket } = await listed([entry('f', 'file')]);
    socket.closeWith(1011);
    await settle();
    flushSync();
    expect(status(target)).toBe('The connection to the device failed.');
    expect(button(target, 'Refresh the list').disabled).toBe(true);
    expect(button(target, 'Download f').disabled).toBe(true);
    expect(button(target, 'Upload a file').disabled).toBe(true);
  });

  it('RA-032: a refused upgrade says the connection could not be opened', async () => {
    const { target, socket } = await show();
    socket.closeWith(1006);
    await settle();
    flushSync();
    expect(status(target)).toBe('The file connection could not be opened.');
  });

  it('RA-032: the link is closed when the panel unmounts', async () => {
    const { socket, close } = await listed([]);
    expect(socket.closedByClient).toBeUndefined();
    await close();
    expect(socket.closedByClient).toBe(1000);
  });

  it('RA-032: unmounting before the ticket arrives opens no link', async () => {
    const { close } = await show({ ok: true, value: { sessionId: 's', token: 't', tunnelPath: '/files', expiresAt: 'x' } });
    await close();
    expect(FakeFilesSocket.made.every((each) => each.closedByClient === 1000 || each.readyState === 0)).toBe(true);
  });
});

describe('RA-032: navigation', () => {
  it('RA-032: a directory opens by its joined path and Up goes to the parent of the resolved path', async () => {
    const { target, socket } = await listed([entry('docs', 'dir')]);
    await click(button(target, 'docs'));
    expect(socket.messages().at(-1)).toEqual({ type: 'list', path: `${HOME}/docs` });
    socket.text({ type: 'entries', path: `${HOME}/docs`, entries: [] });
    await settle();
    flushSync();
    expect(target.querySelector('.where')?.textContent).toBe(`${HOME}/docs`);
    await click(button(target, 'Up one folder'));
    expect(socket.messages().at(-1)).toEqual({ type: 'list', path: HOME });
  });

  it('RA-032: a name that is "..", holds "/" or is empty is never joined and nothing is sent', async () => {
    const { target, socket } = await listed([entry('..', 'dir'), entry('a/b', 'dir'), entry('', 'dir')]);
    const before = socket.frames.length;
    for (const each of [...target.querySelectorAll<HTMLButtonElement>('.open')]) await click(each);
    expect(socket.frames).toHaveLength(before);
    expect(status(target)).toBe('That name cannot be used as a file name on the device.');
  });

  it('RA-032: the device refusing a folder shows its fixed reason and keeps the list', async () => {
    const { target, socket } = await listed([entry('secret', 'dir')]);
    await click(button(target, 'secret'));
    socket.text({ type: 'refused', reason: 'DENIED' });
    await settle();
    flushSync();
    expect(status(target)).toBe('The device did not allow that.');
    expect(names(target)).toEqual(['secret']);
  });
});

describe('RA-032, MOT-009: download', () => {
  it('MOT-009: progress is received over size, exact and linear, with no transition on the value', async () => {
    const { target, socket } = await listed([entry('big.bin', 'file', 200)]);
    await click(button(target, 'Download big.bin'));
    expect(socket.messages().at(-1)).toEqual({ type: 'download', path: `${HOME}/big.bin` });
    socket.text({ type: 'download', size: 200 });
    flushSync();
    expect(bar(target)?.getAttribute('aria-valuenow')).toBe('0');
    const seen: string[] = [];
    for (const length of [50, 50, 100]) {
      socket.bytes(length);
      flushSync();
      seen.push(bar(target)?.getAttribute('aria-valuenow') ?? '');
    }
    expect(seen).toEqual(['0.25', '0.5', '1']);
    expect(bar(target)?.getAttribute('style')).toContain('100%');
    expect(bar(target)?.getAttribute('aria-valuemin')).toBe('0');
    expect(bar(target)?.getAttribute('aria-valuemax')).toBe('1');
  });

  it('RA-032: a finished download is saved under a name reduced to its last clean segment, with the bytes received', async () => {
    const { target, socket, saved } = await listed([entry('rep\u0007ort.txt', 'file', 4)]);
    await click(button(target, 'Download rep\u0007ort.txt'));
    socket.text({ type: 'download', size: 4 });
    socket.bytes(4, 9);
    socket.text({ type: 'done' });
    await settle();
    flushSync();
    expect(saved.map((each) => each.name)).toEqual(['report.txt']);
    expect([...new Uint8Array(await saved[0]?.blob.arrayBuffer())]).toEqual([9, 9, 9, 9]);
    expect(status(target)).toBe('Download finished.');
    expect(bar(target)?.getAttribute('aria-valuenow')).toBe('0');
  });

  it('RA-032: Cancel stops the download, saves nothing and says so', async () => {
    const { target, socket, saved } = await listed([entry('f.bin', 'file', 10)]);
    await click(button(target, 'Download f.bin'));
    socket.text({ type: 'download', size: 10 });
    socket.bytes(3);
    flushSync();
    await click(button(target, 'Cancel'));
    expect(socket.types().at(-1)).toBe('cancel');
    socket.text({ type: 'done' });
    await settle();
    flushSync();
    expect(saved).toEqual([]);
    expect(status(target)).toBe('The transfer was cancelled.');
  });

  it.each([
    ['DOWNLOAD_OFF', 'Downloads are off for this device.'],
    ['NOT_FOUND', 'The device has no file or folder at that place.'],
    ['DENIED', 'The device did not allow that.'],
  ] as const)('RA-032: a download refused with %s shows its fixed text', async (reason, text) => {
    const { target, socket, saved } = await listed([entry('f', 'file')]);
    await click(button(target, 'Download f'));
    socket.text({ type: 'refused', reason });
    await settle();
    flushSync();
    expect(status(target)).toBe(text);
    expect(saved).toEqual([]);
  });
});

describe('RA-032: upload', () => {
  const file = (name: string, bytes = 5) => new File([new Uint8Array(bytes)], name);

  function pick(target: HTMLElement, chosen: File) {
    const input = target.querySelector('input[type="file"]');
    if (!(input instanceof HTMLInputElement)) throw new Error('no picker');
    Object.defineProperty(input, 'files', { configurable: true, value: [chosen] });
    input.dispatchEvent(new Event('change', { bubbles: true }));
  }

  function drop(target: HTMLElement, chosen: File) {
    const event = new Event('drop', { bubbles: true, cancelable: true });
    Object.defineProperty(event, 'dataTransfer', { value: { files: [chosen] } });
    target.querySelector('.files')?.dispatchEvent(event);
  }

  it('RA-032: a picked file is sent to the current directory under its own name, then the folder is listed again', async () => {
    const { target, socket } = await listed([]);
    pick(target, file('notes.txt', 5));
    await settle();
    flushSync();
    expect(socket.messages().at(-1)).toEqual({ type: 'upload', path: `${HOME}/notes.txt`, size: 5 });
    expect(socket.binarySizes()).toEqual([5]);
    socket.text({ type: 'done' });
    await settle();
    flushSync();
    expect(status(target)).toBe('Upload finished.');
    expect(socket.messages().at(-1)).toEqual({ type: 'list', path: HOME });
  });

  it('RA-032: a dropped file is uploaded the same way', async () => {
    const { target, socket } = await listed([]);
    drop(target, file('dropped.bin', 3));
    await settle();
    expect(socket.messages().at(-1)).toEqual({ type: 'upload', path: `${HOME}/dropped.bin`, size: 3 });
  });

  it('RA-032: a local name that is not one plain name is refused before anything is sent', async () => {
    const { target, socket } = await listed([]);
    const before = socket.frames.length;
    for (const name of ['..', 'a/b', '.']) {
      pick(target, file(name));
      await settle();
      flushSync();
    }
    expect(socket.frames).toHaveLength(before);
    expect(status(target)).toBe('That name cannot be used as a file name on the device.');
  });

  it('MOT-009: upload progress is the bytes the socket took over the file size', async () => {
    const { target, socket } = await listed([]);
    socket.bufferedAmount = 0;
    pick(target, file('p.bin', 100));
    await settle();
    flushSync();
    expect(bar(target)?.getAttribute('aria-valuenow')).toBe('1');
  });

  it.each([
    ['UPLOAD_OFF', 'Uploads are off for this device.'],
    ['EXISTS', 'A file with this name is already there; Anima never replaces a file.'],
    ['DENIED', 'The device did not allow that.'],
    ['NOT_FOUND', 'The device has no file or folder at that place.'],
  ] as const)('RA-032: an upload refused with %s shows its fixed text and the folder is listed again', async (reason, text) => {
    const { target, socket } = await listed([]);
    pick(target, file('x.txt'));
    await settle();
    socket.text({ type: 'refused', reason });
    await settle();
    flushSync();
    expect(status(target)).toBe(text);
    expect(socket.types().slice(-2)).toEqual(['cancel', 'list']);
  });
});

describe('D-5: one transfer at a time', () => {
  it('D-5: while a download runs every other action is disabled and only Cancel is on', async () => {
    const { target, socket } = await listed([entry('docs', 'dir'), entry('a', 'file'), entry('b', 'file')]);
    await click(button(target, 'Download a'));
    const sent = socket.frames.length;
    expect(button(target, 'Download b').disabled).toBe(true);
    expect(button(target, 'docs').disabled).toBe(true);
    expect(button(target, 'Up one folder').disabled).toBe(true);
    expect(button(target, 'Refresh the list').disabled).toBe(true);
    expect(button(target, 'Upload a file').disabled).toBe(true);
    expect(button(target, 'Cancel').disabled).toBe(false);
    await click(button(target, 'Download b'));
    expect(socket.frames).toHaveLength(sent);
    socket.text({ type: 'refused', reason: 'DENIED' });
    await settle();
    flushSync();
    expect(button(target, 'Download b').disabled).toBe(false);
    expect(button(target, 'Cancel').disabled).toBe(true);
  });
});

describe('MAT-008, MOT-009: the panel is solid and its bar is not eased', () => {
  const source = readFileSync(`${import.meta.dirname}/FilesPanel.svelte`, 'utf8');
  const style = source.slice(source.indexOf('<style>'));

  it('MAT-008: no blur, translucency, filter or overlay position in the panel stylesheet', () => {
    for (const forbidden of ['backdrop-filter', 'filter:', 'opacity', 'position: absolute', 'position: fixed', 'transparent', 'color-mix']) {
      expect(style, forbidden).not.toContain(forbidden);
    }
    expect(style).toContain('background: var(--surface)');
  });

  it('MOT-009: nothing in the panel stylesheet transitions or animates', () => {
    expect(style).not.toMatch(/transition|animation|@keyframes/);
  });
});
