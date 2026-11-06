import type { ITheme } from '@xterm/xterm';

/** The Nord palette number behind each of the 16 ANSI colours: the mapping in `design/tokens/terminal.tokens.json` (RA-010). */
export const ANSI_NORD = [1, 11, 14, 13, 9, 15, 8, 5, 3, 11, 14, 13, 9, 15, 7, 6] as const;
const BACKGROUND_NORD = 0;
const FOREGROUND_NORD = 4;
/** The cursor is nord4 like the text; the selection is nord3 under it, which keeps the text readable. */
const CURSOR_NORD = 4;
const SELECTION_NORD = 3;
const SELECTION_INACTIVE_NORD = 2;

/** Reads a CSS custom property (`--nord-0`) as the page resolves it; an unset one gives an empty string. */
export type TokenReader = (name: string) => string;

/**
 * The xterm.js theme from the Nord tokens (RA-010, TOK-003): every colour is a CSS variable read at the time of
 * the call, so no colour value is written in code.
 */
export function terminalTheme(read: TokenReader): ITheme {
  const nord = (index: number) => read(`--nord-${String(index)}`);
  return {
    background: nord(BACKGROUND_NORD),
    foreground: nord(FOREGROUND_NORD),
    cursor: nord(CURSOR_NORD),
    cursorAccent: nord(BACKGROUND_NORD),
    selectionBackground: nord(SELECTION_NORD),
    selectionInactiveBackground: nord(SELECTION_INACTIVE_NORD),
    black: nord(ANSI_NORD[0]),
    red: nord(ANSI_NORD[1]),
    green: nord(ANSI_NORD[2]),
    yellow: nord(ANSI_NORD[3]),
    blue: nord(ANSI_NORD[4]),
    magenta: nord(ANSI_NORD[5]),
    cyan: nord(ANSI_NORD[6]),
    white: nord(ANSI_NORD[7]),
    brightBlack: nord(ANSI_NORD[8]),
    brightRed: nord(ANSI_NORD[9]),
    brightGreen: nord(ANSI_NORD[10]),
    brightYellow: nord(ANSI_NORD[11]),
    brightBlue: nord(ANSI_NORD[12]),
    brightMagenta: nord(ANSI_NORD[13]),
    brightCyan: nord(ANSI_NORD[14]),
    brightWhite: nord(ANSI_NORD[15]),
  };
}

/** The reader for the tokens `element` sees. */
export function tokensOf(element: Element): TokenReader {
  const style = getComputedStyle(element);
  return (name) => style.getPropertyValue(name).trim();
}

/**
 * A size token (`0.75rem`, `12px`) in CSS px; `rootPx` is the root font size. A value that is neither falls back
 * to `fallback`, so a missing token still gives a readable terminal.
 */
export function sizeInPx(token: string, rootPx: number, fallback: number): number {
  const match = /^(\d*\.?\d+)(rem|px)$/.exec(token);
  if (match === null) return fallback;
  const value = Number(match[1]);
  return match[2] === 'rem' ? value * rootPx : value;
}
