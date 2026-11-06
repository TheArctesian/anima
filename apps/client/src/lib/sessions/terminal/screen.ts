import { FitAddon } from '@xterm/addon-fit';
import { SearchAddon } from '@xterm/addon-search';
import { Unicode11Addon } from '@xterm/addon-unicode11';
import { WebLinksAddon } from '@xterm/addon-web-links';
import { WebglAddon } from '@xterm/addon-webgl';
import { Terminal } from '@xterm/xterm';
import '@xterm/xterm/css/xterm.css';
import type { CellSize } from '../tunnel/terminal/codec';
import { guardEscapes, type EscapeHooks } from './escapes';
import { safePaste } from './paste';
import { shortcutOf, type ShortcutAction } from './shortcuts';
import { sizeInPx, terminalTheme, tokensOf } from './theme';

/** Lines kept above the visible screen (RA-003). */
const SCROLLBACK_LINES = 5000;
/** The size used when the token cannot be read: spec Section 6.3.1 says 12. */
const FALLBACK_FONT_PX = 12;
const ROOT_FALLBACK_PX = 16;

/** What the screen tells the session view. */
export interface ScreenHooks extends EscapeHooks {
  /** What the operator typed or pasted, as bytes for the shell. */
  readonly oninput: (bytes: Uint8Array<ArrayBuffer>) => void;
  /** The terminal changed size in cells (after a fit). */
  readonly onresize: (size: CellSize) => void;
  /** The operator asked to copy the selection (Ctrl+Shift+C, Cmd+C). */
  readonly oncopy: (text: string) => void;
  /** The operator asked to find (Cmd+F, Ctrl+Shift+F). */
  readonly onfind: () => void;
}

/** The xterm.js terminal of one session pane. It is the only thing that touches xterm.js outside this folder's files. */
export interface TerminalScreen {
  size(): CellSize;
  /** Draw terminal output from the hub. */
  write(bytes: Uint8Array): void;
  /** Clear the screen and the scroll-back (a new session in the same pane). */
  reset(): void;
  /** Fit the terminal to its host; a changed size reaches `onresize`. */
  fit(): void;
  focus(): void;
  blur(): void;
  /** Paste `text` as the operator's own action; bracketed when the shell asked for it. */
  paste(text: string): void;
  /** Select the next (or previous) match of `text`; false when there is none. */
  find(text: string, backwards: boolean): boolean;
  clearFind(): void;
  dispose(): void;
}

/** Opens a screen in a host element; the session pane passes it to the view. */
export type ScreenOpener = (host: HTMLElement, hooks: ScreenHooks) => Promise<TerminalScreen>;

/** Wait for the terminal font so the first cell measure is right; an engine without font loading goes on at once. */
async function fontReady(family: string, sizePx: number): Promise<void> {
  if (!('fonts' in document)) return;
  try {
    await document.fonts.load(`${String(sizePx)}px ${family}`);
  } catch {
    // A font that cannot load leaves the fallback family in use; the terminal still works.
  }
}

/**
 * WebGL when the engine offers it; otherwise, or when its context is lost, xterm.js's DOM renderer draws. The addon
 * only reports a lost context after a 3 s wait for a restore, which leaves the screen blank; the canvas's own
 * `webglcontextlost` event (which does not bubble, so it is heard in the capture phase) disposes the addon at once,
 * and `onContextLoss` stays as the backstop.
 */
function enableWebgl(terminal: Terminal): void {
  try {
    const webgl = new WebglAddon();
    let disposed = false;
    const fallBack = () => {
      if (disposed) return;
      disposed = true;
      webgl.dispose();
    };
    webgl.onContextLoss(fallBack);
    terminal.loadAddon(webgl);
    terminal.element?.addEventListener('webglcontextlost', fallBack, { capture: true, once: true });
  } catch {
    // No WebGL 2: the DOM renderer stays (xterm.js 6 has no canvas renderer).
  }
}

function acts(terminal: Terminal, action: ShortcutAction, hooks: ScreenHooks): void {
  if (action === 'copy' && terminal.hasSelection()) hooks.oncopy(terminal.getSelection());
  if (action === 'find') hooks.onfind();
}

/** Input, size and shortcuts from the terminal to the hooks. */
function wireEvents(terminal: Terminal, hooks: ScreenHooks): void {
  const encoder = new TextEncoder();
  terminal.onData((text) => {
    hooks.oninput(encoder.encode(text));
  });
  // `onBinary` carries a binary string: one byte per character (mouse reports above column 223).
  terminal.onBinary((text) => {
    hooks.oninput(Uint8Array.from(text, (character) => character.charCodeAt(0)));
  });
  terminal.onResize(({ cols, rows }) => {
    hooks.onresize({ cols, rows });
  });
  terminal.attachCustomKeyEventHandler((event) => {
    const action = shortcutOf(event);
    if (action === undefined) return true;
    if (event.type === 'keydown') {
      // Paste is left to the browser's `paste` event, which xterm.js handles; the others are ours.
      if (action !== 'paste') event.preventDefault();
      acts(terminal, action, hooks);
    }
    return false;
  });
}

/**
 * Open the session terminal in `host` (RA-003, RA-011): the shipped Nord-themed OpenDyslexicM Nerd Font Mono at
 * the token size, 5000 lines of scroll-back, Unicode 11 widths, search, detected links (confirmed by the view),
 * WebGL with a DOM fallback, and the escapes guarded as K4 §8.7 says. Resolves once the font is ready and the
 * terminal is open and fitted.
 */
export const openScreen: ScreenOpener = async (host, hooks) => {
  const tokens = tokensOf(host);
  const rootPx = parseFloat(getComputedStyle(document.documentElement).fontSize) || ROOT_FALLBACK_PX;
  const fontSize = sizeInPx(tokens('--text-xs'), rootPx, FALLBACK_FONT_PX);
  const fontFamily = tokens('--font-mono-opendyslexic');
  await fontReady(fontFamily, fontSize);
  const terminal = new Terminal({
    scrollback: SCROLLBACK_LINES,
    fontFamily,
    fontSize,
    theme: terminalTheme(tokens),
    cursorBlink: false,
    // Unicode 11 widths are set through the proposed `unicode` API.
    allowProposedApi: true,
  });
  const fit = new FitAddon();
  const search = new SearchAddon();
  terminal.loadAddon(fit);
  terminal.loadAddon(search);
  terminal.loadAddon(new Unicode11Addon());
  terminal.unicode.activeVersion = '11';
  terminal.loadAddon(
    new WebLinksAddon((_event, address) => {
      hooks.onlink(address);
    }),
  );
  guardEscapes(terminal, hooks);
  wireEvents(terminal, hooks);
  terminal.open(host);
  // The paste event is taken before xterm.js sees it, so the text is cleaned first (RA-003).
  host.addEventListener(
    'paste',
    (event) => {
      event.preventDefault();
      event.stopPropagation();
      terminal.paste(safePaste(event.clipboardData?.getData('text/plain') ?? ''));
    },
    true,
  );
  enableWebgl(terminal);
  fit.fit();
  return screenOf(terminal, fit, search);
};

/** The screen over an open `terminal`. */
function screenOf(terminal: Terminal, fit: FitAddon, search: SearchAddon): TerminalScreen {
  return {
    size: () => ({ cols: terminal.cols, rows: terminal.rows }),
    write: (bytes) => {
      terminal.write(bytes);
    },
    reset: () => {
      terminal.reset();
    },
    fit: () => {
      fit.fit();
    },
    focus: () => {
      terminal.focus();
    },
    blur: () => {
      terminal.blur();
    },
    paste: (text) => {
      terminal.paste(safePaste(text));
    },
    find: (text, backwards) => (backwards ? search.findPrevious(text) : search.findNext(text)),
    clearFind: () => {
      search.clearDecorations();
      terminal.clearSelection();
    },
    dispose: () => {
      terminal.dispose();
    },
  };
}
