// @vitest-environment jsdom
import { flushSync } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { installBlankCanvas } from '../tunnel/fake/canvas.fixture';
import { FakeTerminalSocket } from '../tunnel/terminal/fake/socket.fixture';
import { connectView, flags, installBrowserFakes, key, screenText, settle, showView, systemClipboard, textareaOf } from './view.fixture.svelte';

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
  vi.restoreAllMocks();
  vi.useRealTimers();
});

async function connected() {
  const view = showView();
  cleanups.push(view.close);
  const socket = await connectView();
  return { ...view, socket, field: textareaOf(view.target) };
}

const F = { key: 'f', code: 'KeyF', keyCode: 70 };

describe('the remote host controls the bytes (K4 §8.7)', () => {
  it('K4 §8.7: OSC 52, a write or a read of the clipboard, never reaches the system clipboard', async () => {
    const { socket, target } = await connected();
    const write = vi.spyOn(systemClipboard, 'writeText');
    const read = vi.spyOn(systemClipboard, 'readText');
    socket.output('\x1b]52;c;U0VDUkVU\x07\x1b]52;c;?\x07after');
    expect(screenText(target)).toContain('after');
    expect(write).not.toHaveBeenCalled();
    expect(read).not.toHaveBeenCalled();
    expect(systemClipboard.written).toEqual([]);
    expect(socket.typed()).toEqual([]);
  });

  it('K4 §8.7: the window title reaches the tab without control or bidi characters, at most 80 characters', async () => {
    const { socket, heard } = await connected();
    const override = String.fromCodePoint(0x202e);
    socket.output(`\x1b]0;user@host: ${'long '.repeat(30)}${override}\x07`);
    vi.advanceTimersByTime(100);
    const title = heard.titles.at(-1) ?? '';
    expect(title.startsWith('user@host: long')).toBe(true);
    expect([...title].length).toBeLessThanOrEqual(80);
    expect(title).not.toContain(override);
  });

  it.each(['javascript:alert(1)', 'data:text/html,hi', 'file:///etc/passwd', 'ftp://example.com/x', 'not a link', 'https://accounts.google.com:443@evil.example/login'])(
    'K4 §8.7: the link %s is refused, with no dialog and nothing opened',
    async (address) => {
      const open = vi.spyOn(window, 'open').mockReturnValue(null);
      const { target, heard } = await connected();
      heard.hooks?.onlink(address);
      await settle();
      expect(document.querySelector('[role="dialog"]')).toBeNull();
      expect(target.textContent).toContain('not http or https');
      expect(open).not.toHaveBeenCalled();
    },
  );

  it('K4 §8.7: an https link shows its full address, opens only after "Open link", and without access to this window', async () => {
    const open = vi.spyOn(window, 'open').mockReturnValue(null);
    const { heard } = await connected();
    heard.hooks?.onlink('https://example.com/a/very/long/path?with=query&and=more#frag');
    await settle();
    expect(document.querySelector('[data-address]')?.textContent).toBe('https://example.com/a/very/long/path?with=query&and=more#frag');
    expect(document.querySelector('[data-host]')?.textContent).toBe('example.com');
    expect(open).not.toHaveBeenCalled();
    [...document.querySelectorAll('button')].find((button) => button.textContent === 'Open link')?.click();
    await settle();
    expect(open).toHaveBeenCalledExactlyOnceWith('https://example.com/a/very/long/path?with=query&and=more#frag', '_blank', 'noopener,noreferrer');
  });

  it('K4 §8.7: Cancel opens nothing', async () => {
    const open = vi.spyOn(window, 'open').mockReturnValue(null);
    const { heard } = await connected();
    heard.hooks?.onlink('http://example.com/');
    await settle();
    [...document.querySelectorAll('button')].find((button) => button.textContent === 'Cancel')?.click();
    await settle();
    expect(open).not.toHaveBeenCalled();
  });
});

describe('find (RA-003)', () => {
  it('RA-003: Cmd+F opens a labelled find bar; Enter finds the next match, "No match" says when there is none, Escape closes', async () => {
    const { socket, target, field } = await connected();
    socket.output('needle in a haystack');
    vi.advanceTimersByTime(100);
    key(field, { ...F, metaKey: true });
    await settle();
    const input = target.querySelector<HTMLInputElement>('input[aria-label="Find in terminal"]');
    if (input === null) throw new Error('no find bar');
    expect(document.activeElement).toBe(input);
    // UI-009: the find bar has its own row, so the toolbar cannot overflow a narrow pane.
    expect(target.querySelector('.toolbar')?.contains(input)).toBe(false);
    input.value = 'needle';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true, cancelable: true }));
    flushSync();
    expect(target.querySelector('.find [role="status"]')?.textContent).toBe('');
    input.value = 'absent';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true, cancelable: true }));
    flushSync();
    expect(target.querySelector('.find [role="status"]')?.textContent).toBe('No match');
    input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true }));
    flushSync();
    expect(target.querySelector('input[aria-label="Find in terminal"]')).toBeNull();
    expect(document.activeElement).toBe(field);
  });

  it('RA-003: Ctrl+F alone stays with the shell; the Find button opens the bar', async () => {
    const { socket, target, field } = await connected();
    key(field, { ...F, ctrlKey: true });
    expect(socket.typed()).toEqual([6]);
    expect(target.querySelector('input[aria-label="Find in terminal"]')).toBeNull();
    [...target.querySelectorAll('button')].find((button) => button.textContent.includes('Find'))?.click();
    flushSync();
    expect(target.querySelector('input[aria-label="Find in terminal"]')).not.toBeNull();
  });
});
