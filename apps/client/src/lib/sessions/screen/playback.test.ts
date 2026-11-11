// @vitest-environment jsdom
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import type { Terminal } from '@xterm/xterm';
import { openPlaybackTerminal } from './playback';

const made = vi.hoisted(() => ({ terminals: [] as Terminal[] }));
vi.mock('@xterm/xterm', async (importOriginal) => {
  const original = await importOriginal<typeof import('@xterm/xterm')>();
  class Watched extends original.Terminal {
    constructor(options?: ConstructorParameters<typeof original.Terminal>[0]) {
      super(options);
      made.terminals.push(this);
    }
  }
  return { ...original, Terminal: Watched };
});

let saved: typeof window.matchMedia;

beforeEach(() => {
  saved = window.matchMedia;
  window.matchMedia = (() => ({ matches: false, addEventListener: () => undefined, removeEventListener: () => undefined, addListener: () => undefined, removeListener: () => undefined })) as unknown as typeof window.matchMedia;
});

afterEach(() => {
  window.matchMedia = saved;
  document.body.innerHTML = '';
});

function opened() {
  const host = document.createElement('div');
  document.body.append(host);
  return { host, playback: openPlaybackTerminal(host) };
}

it('RA-041: the playback terminal opens without a keyboard to the device', () => {
  const { host, playback } = opened();
  expect(host.querySelector('.xterm')).not.toBeNull();
  const sent = vi.fn();
  made.terminals.at(-1)?.onData(sent);
  const input = host.querySelector('.xterm-helper-textarea');
  input?.dispatchEvent(new KeyboardEvent('keydown', { key: 'a', code: 'KeyA', keyCode: 65, bubbles: true }));
  input?.dispatchEvent(new KeyboardEvent('keypress', { key: 'a', charCode: 97, keyCode: 97, bubbles: true }));
  input?.dispatchEvent(new InputEvent('input', { data: 'a', inputType: 'insertText', bubbles: true }));
  expect(sent).not.toHaveBeenCalled();
  playback.dispose();
});

it('RA-041: the playback terminal survives use after dispose', () => {
  const { playback } = opened();
  playback.terminal.resize(100, 30);
  playback.terminal.write('hello');
  playback.terminal.reset();
  playback.dispose();
  expect(() => {
    playback.terminal.write('late');
    playback.terminal.resize(10, 10);
    playback.terminal.reset();
  }).not.toThrow();
});

it('RA-041: output written before a resize is parsed at the old size, as recorded', async () => {
  const { host, playback } = opened();
  playback.terminal.resize(20, 6);
  // A cursor move to column 15, then X: parsed at 20 columns it lands at column 15; at 5 columns it would be clamped to column 5.
  playback.terminal.write('\x1b[1;15HX');
  playback.terminal.resize(5, 6);
  await new Promise((resolve) => setTimeout(resolve, 300));
  const lines = [...host.querySelectorAll('.xterm-rows > div')].map((row) => row.textContent?.trimEnd() ?? '');
  // Parsed in order, X sat at column 15 and the narrowing cut it off; parsed after the resize it would show.
  expect(lines.some((line) => line.includes('X'))).toBe(false);
  playback.dispose();
});
