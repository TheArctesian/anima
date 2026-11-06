// @vitest-environment jsdom
import { Terminal } from '@xterm/xterm';
import { describe, expect, it, vi } from 'vitest';
import { TITLE_MAX, cleanTitle, guardEscapes } from './escapes';

const RANGE = { start: { x: 1, y: 1 }, end: { x: 2, y: 1 } };

/** A terminal with the guards on, and what they reported once `output` was written. */
async function after(output: string) {
  const terminal = new Terminal({ allowProposedApi: true });
  const hooks = { ontitle: vi.fn<(title: string) => void>(), onlink: vi.fn<(address: string) => void>() };
  guardEscapes(terminal, hooks);
  await new Promise<void>((done) => {
    terminal.write(output, done);
  });
  return { terminal, hooks };
}

describe('window title (K4 §8.7)', () => {
  it.each([
    ['plain text stays', 'user@host: ~/src', 'user@host: ~/src'],
    ['C0 controls go', 'a\u0000b\u0007c\u001bd\u007fe', 'abcde'],
    ['C1 controls go', 'a\u0085b\u009bc', 'abc'],
    ['bidi overrides, embeddings and isolates go', 'a\u202eb\u202ac\u2066d\u2069e\u200ff\u061cg', 'abcdefg'],
    ['line and paragraph separators go', 'a\u2028b\u2029c', 'abc'],
    ['zero-width characters go', 'a\u200bb\ufeffc', 'abc'],
    ['surrounding space goes', '  tmux  ', 'tmux'],
    ['an emoji stays whole', '\u{1f680} deploy', '\u{1f680} deploy'],
  ])('K4 §8.7: %s', (_name, raw, expected) => {
    expect(cleanTitle(raw)).toBe(expected);
  });

  it('K4 §8.7: a title is at most 80 characters, cut by code points so a surrogate pair is never split', () => {
    const cut = cleanTitle('\u{1f680}'.repeat(200));
    expect([...cut]).toHaveLength(TITLE_MAX);
    expect(cut).toBe('\u{1f680}'.repeat(TITLE_MAX));
    expect(cleanTitle('x'.repeat(81))).toBe('x'.repeat(80));
  });

  it('K4 §8.7: a title the remote sets with OSC 0 or OSC 2 reaches the tab cleaned', async () => {
    const { hooks } = await after('\u001b]0;one\u202e\u0007\u001b]2;two\u0007');
    expect(hooks.ontitle.mock.calls).toEqual([['one'], ['two']]);
  });
});

describe('what the remote may do (K4 §8.7)', () => {
  it('K4 §8.7: OSC 52 is swallowed: it neither fails the parser nor reaches a hook, and text after it still draws', async () => {
    const { terminal, hooks } = await after('\u001b]52;c;U0VDUkVU\u0007\u001b]52;c;?\u0007visible');
    expect(terminal.buffer.active.getLine(0)?.translateToString(true)).toBe('visible');
    expect(hooks.ontitle).not.toHaveBeenCalled();
    expect(hooks.onlink).not.toHaveBeenCalled();
  });

  it('K4 §8.7: activating an OSC 8 link passes its address to the owner, which decides; nothing opens here', async () => {
    const open = vi.spyOn(window, 'open').mockReturnValue(null);
    const { terminal, hooks } = await after('\u001b]8;;javascript:alert(1)\u0007click\u001b]8;;\u0007');
    terminal.options.linkHandler?.activate(new MouseEvent('click'), 'javascript:alert(1)', RANGE);
    expect(hooks.onlink.mock.calls).toEqual([['javascript:alert(1)']]);
    expect(open).not.toHaveBeenCalled();
    open.mockRestore();
  });
});
