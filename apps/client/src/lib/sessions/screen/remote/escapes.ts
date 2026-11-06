import type { Terminal } from '@xterm/xterm';

/** The longest title the tab shows, in characters (K4 §8.7). */
export const TITLE_MAX = 80;

/** OSC 52: write to the clipboard. */
const OSC_CLIPBOARD = 52;

/**
 * Control characters (C0, DEL, C1), format characters (which include the bidirectional marks, embeddings,
 * overrides and isolates, and the zero-width ones) and the line and paragraph separators.
 */
const UNSAFE_IN_TITLE = /[\p{Cc}\p{Cf}\u2028\u2029]/gu;

/**
 * A window title the remote host set (OSC 0 and 2) made safe for the tab (K4 §8.7): controls and bidi overrides
 * removed, surrounding space trimmed, at most `TITLE_MAX` characters (code points, so an emoji is never cut).
 */
export function cleanTitle(raw: string): string {
  return [...raw.replace(UNSAFE_IN_TITLE, '').trim()].slice(0, TITLE_MAX).join('').trim();
}

/** What the escapes the remote host may send reach. */
export interface EscapeHooks {
  /** A clean title for the tab. */
  readonly ontitle: (title: string) => void;
  /** An OSC 8 link was activated; the address is the remote's, unchecked. */
  readonly onlink: (address: string) => void;
}

/**
 * Set what the remote host's escape sequences may do (K4 §8.7). OSC 52 is swallowed, so a remote cannot fill the
 * clipboard (no clipboard add-on is loaded either). A title goes through `cleanTitle`. An OSC 8 hyperlink goes to
 * `onlink` and nowhere else.
 */
export function guardEscapes(terminal: Terminal, hooks: EscapeHooks): void {
  terminal.parser.registerOscHandler(OSC_CLIPBOARD, () => true);
  terminal.onTitleChange((title) => {
    hooks.ontitle(cleanTitle(title));
  });
  terminal.options.linkHandler = {
    activate: (_event, address) => {
      hooks.onlink(address);
    },
  };
}
