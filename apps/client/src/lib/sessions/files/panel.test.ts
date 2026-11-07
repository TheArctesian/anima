// @vitest-environment jsdom
import { flushSync } from 'svelte';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { UNREACHABLE } from '$lib/hub/error';
import { FakeFilesSocket } from './link/fake/socket.fixture';
import { settle } from './link/fake/link.fixture';
import { HOME, button, click, entry, listed, names, removePanels, resetPanels, show, status } from './panel.fixture';

beforeEach(resetPanels);
afterEach(removePanels);

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

});

describe('RA-032: when the link or ticket fails', () => {
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
