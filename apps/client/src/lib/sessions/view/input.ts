import type { Attachment } from 'svelte/attachments';
import { forwardPointer, sendClipboard, type KeyRoute } from '../tunnel/input';
import type { SessionTunnel } from '../tunnel/tunnel';
import { PASTE_STROKES, keyFilter, keyStrokes, type KeyCombination, type KeyStroke } from './keys';

type Client = SessionTunnel['client'];

function send(client: Client, strokes: readonly KeyStroke[]): void {
  for (const [pressed, keysym] of strokes) client.sendKeyEvent(pressed, keysym);
}

/** The route of the pane's keyboard to `client`: keys the filter passes are sent as 1 or 0 (T1 note F-3). */
export function keyRoute(client: Client): KeyRoute {
  const filter = keyFilter();
  return {
    down: (keysym) => {
      if (!filter.down(keysym)) return true;
      client.sendKeyEvent(1, keysym);
      return false;
    },
    up: (keysym) => {
      if (filter.up(keysym)) client.sendKeyEvent(0, keysym);
    },
  };
}

/** Send a Keys menu combination as its press and release sequence (D-10, RA-006). */
export function sendCombination(client: Client, combination: KeyCombination): void {
  send(client, keyStrokes(combination));
}

/**
 * Paste `text` into the terminal (D-9): one `text/plain` clipboard stream, then guacd's terminal paste shortcut,
 * which types the clipboard it now holds. Empty text sends nothing.
 */
export function pasteText(client: Client, text: string): void {
  if (text === '') return;
  sendClipboard(client, text);
  send(client, PASTE_STROKES);
}

/**
 * Show the client's display in the host element (D-7): at a `devicePixelRatio` above 1 the remote draws in device
 * pixels, so the library's own static scale maps it back to CSS pixels; nothing animates it (MAT-008). Pointer
 * input is forwarded while it is shown.
 */
export function showDisplay(client: Client, devicePixelRatio: number): Attachment<HTMLElement> {
  return (host) => {
    const display = client.getDisplay();
    if (devicePixelRatio > 0 && devicePixelRatio !== 1) display.scale(1 / devicePixelRatio);
    const element = display.getElement();
    host.append(element);
    const stop = forwardPointer(client);
    return () => {
      stop();
      element.remove();
    };
  };
}
