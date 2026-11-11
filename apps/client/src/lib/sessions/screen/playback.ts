import { Terminal } from '@xterm/xterm';
import '@xterm/xterm/css/xterm.css';
import { terminalOptions } from './options';

/** Full reset (RIS). It goes through `write` so it falls in order after the output already queued. */
const RESET = '\x1bc';

/** What a recording player needs from a terminal. The recorded text is written as it is; nothing animates it (MAT-008). */
export interface PlaybackTerminal {
  write(text: string): void;
  resize(cols: number, rows: number): void;
  /** Clear the screen and the scroll-back. */
  reset(): void;
}

export interface OpenedPlayback {
  readonly terminal: PlaybackTerminal;
  dispose(): void;
}

/**
 * An xterm.js terminal for playback in `host`: the same Nord theme and font tokens as the session terminal, the
 * recorded size (the player sets it), and no input: playback has no keyboard to a device (RA-041).
 */
export function openPlaybackTerminal(host: HTMLElement): OpenedPlayback {
  const xterm = new Terminal({
    ...terminalOptions(host),
    cursorStyle: 'underline',
    cursorInactiveStyle: 'none',
    disableStdin: true,
  });
  xterm.open(host);
  let open = true;
  // `write` is queued and parsed later but `resize` acts at once, so a resize waits in the write queue behind the
  // output written before it.
  const queuedResize = (cols: number, rows: number) => {
    xterm.write('', () => {
      if (open) xterm.resize(cols, rows);
    });
  };
  return {
    terminal: {
      write: (text) => {
        if (open) xterm.write(text);
      },
      resize: (cols, rows) => {
        if (open) queuedResize(cols, rows);
      },
      reset: () => {
        if (open) xterm.write(RESET);
      },
    },
    dispose: () => {
      open = false;
      xterm.dispose();
    },
  };
}
