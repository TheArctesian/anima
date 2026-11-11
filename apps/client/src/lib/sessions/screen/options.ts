import type { ITerminalOptions } from '@xterm/xterm';
import { MINIMUM_CONTRAST_RATIO, sizeInPx, terminalTheme, tokensOf } from './theme';

/** Lines kept above the visible screen (RA-003). */
const SCROLLBACK_LINES = 5000;
/** The size used when the token cannot be read: spec Section 6.3.1 says 12. */
const FALLBACK_FONT_PX = 12;
const ROOT_FALLBACK_PX = 16;

/** The font the terminals of a `host` draw with: token family and size in pixels. */
export function terminalFont(host: HTMLElement): { readonly fontFamily: string; readonly fontSize: number } {
  const tokens = tokensOf(host);
  const rootPx = parseFloat(getComputedStyle(document.documentElement).fontSize) || ROOT_FALLBACK_PX;
  return { fontFamily: tokens('--font-mono-opendyslexic'), fontSize: sizeInPx(tokens('--text-xs'), rootPx, FALLBACK_FONT_PX) };
}

/** The xterm.js options the live and the playback terminal share: Nord theme, font tokens, scroll-back, contrast. */
export function terminalOptions(host: HTMLElement, font = terminalFont(host)): ITerminalOptions {
  return {
    scrollback: SCROLLBACK_LINES,
    ...font,
    theme: terminalTheme(tokensOf(host)),
    cursorBlink: false,
    minimumContrastRatio: MINIMUM_CONTRAST_RATIO,
  };
}
