import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { ANSI_NORD, sizeInPx, terminalTheme } from './theme';

const palette = new Map(Array.from({ length: 16 }, (_, index) => [`--nord-${String(index)}`, `value-of-nord-${String(index)}`]));
const read = (name: string) => palette.get(name) ?? '';

describe('the Nord theme (RA-010, TOK-003)', () => {
  const tokens = JSON.parse(readFileSync(new URL('../../../../../../design/tokens/terminal.tokens.json', import.meta.url), 'utf8')) as {
    terminal: { background: { $value: string }; foreground: { $value: string }; ansi: Record<string, { $value: string }> };
  };
  const nord = (reference: string) => Number(/^\{color\.nord\.(\d+)\}$/.exec(reference)?.[1]);

  it('RA-010: the 16 ANSI colours use the palette numbers of design/tokens/terminal.tokens.json', () => {
    expect([...ANSI_NORD]).toEqual(Array.from({ length: 16 }, (_, index) => nord(tokens.terminal.ansi[String(index)]?.$value ?? '')));
  });

  it('RA-010: background and foreground are the terminal tokens', () => {
    const theme = terminalTheme(read);
    expect(theme.background).toBe(read(`--nord-${String(nord(tokens.terminal.background.$value))}`));
    expect(theme.foreground).toBe(read(`--nord-${String(nord(tokens.terminal.foreground.$value))}`));
  });

  it('TOK-003: every colour is read from a --nord-N variable, none is written in code', () => {
    const theme = terminalTheme(read);
    const colours = Object.values(theme);
    expect(colours).toHaveLength(22);
    for (const colour of colours) expect(colour).toMatch(/^value-of-nord-\d+$/);
  });

  it('RA-010: the cursor and the selection have their own colours, the cursor text sits on the background', () => {
    const theme = terminalTheme(read);
    expect(theme.cursor).toBe('value-of-nord-4');
    expect(theme.cursorAccent).toBe('value-of-nord-0');
    expect(theme.selectionBackground).toBe('value-of-nord-3');
    expect(theme.selectionBackground).not.toBe(theme.background);
  });
});

describe('the font size token (spec 6.3.1)', () => {
  it.each([
    ['0.75rem', 16, 12],
    ['0.75rem', 20, 15],
    ['12px', 16, 12],
    ['1.5rem', 16, 24],
  ])('%s at a root of %i px is %i px', (token, root, expected) => {
    expect(sizeInPx(token, root, 99)).toBe(expected);
  });

  it.each(['', 'calc(1rem)', 'large', '12'])('%j is not a size: the fallback is used', (token) => {
    expect(sizeInPx(token, 16, 12)).toBe(12);
  });
});
