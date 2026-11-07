import type { Attachment } from 'svelte/attachments';
import { forwardPointer, sendClipboard, type KeyRoute } from '../../tunnel/guacamole/input';
import type { SessionTunnel } from '../../tunnel/guacamole/tunnel';
import { pasteStrokes, keyStrokes, type KeyCombination, type KeyFilter, type KeyStroke } from './keys';

type Client = SessionTunnel['client'];

function send(client: Client, strokes: readonly KeyStroke[]): void {
  for (const [pressed, keysym] of strokes) client.sendKeyEvent(pressed, keysym);
}

/** The route of the pane's keyboard to `client`: keys `filter` passes are sent as 1 or 0 (T1 note F-3). */
export function keyRoute(client: Client, filter: KeyFilter): KeyRoute {
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

/** Send a Keys menu combination as its press and release sequence (RA-006). */
export function sendCombination(client: Client, combination: KeyCombination): void {
  send(client, keyStrokes(combination));
}

/**
 * Paste `text` on the device (RA-005): one `text/plain` clipboard stream, which sets the device's clipboard, then
 * Ctrl+V, which pastes it, leaving down any Ctrl in `held` (the keys the device holds). Empty text sends nothing.
 */
export function pasteText(client: Client, text: string, held: ReadonlySet<number>): void {
  if (text === '') return;
  sendClipboard(client, text);
  send(client, pasteStrokes(held));
}

/**
 * Follow whether the keyboard focus is inside `pane` (D-8): `change(true)` when focus enters it, `change(false)`
 * when focus moves out of it, the window loses focus, or the page is hidden. In those three cases no `keyup` will
 * reach the pane for a key still held, so the caller must release it at once. Returns the function that stops.
 */
export function followPaneFocus(pane: HTMLElement, change: (inside: boolean) => void): () => void {
  const inside = () => {
    change(pane.contains(document.activeElement));
  };
  const focusout = (event: FocusEvent) => {
    if (!(event.relatedTarget instanceof Node && pane.contains(event.relatedTarget))) change(false);
  };
  const lost = () => {
    change(false);
  };
  const visibility = () => {
    if (document.visibilityState === 'hidden') change(false);
    else inside();
  };
  pane.addEventListener('focusin', inside);
  pane.addEventListener('focusout', focusout);
  window.addEventListener('blur', lost);
  window.addEventListener('focus', inside);
  document.addEventListener('visibilitychange', visibility);
  inside();
  return () => {
    pane.removeEventListener('focusin', inside);
    pane.removeEventListener('focusout', focusout);
    window.removeEventListener('blur', lost);
    window.removeEventListener('focus', inside);
    document.removeEventListener('visibilitychange', visibility);
  };
}

/**
 * Show the client's display in the host element; nothing animates it (MAT-008), and its scale is the caller's.
 * Pointer and touch input are forwarded while it is shown.
 */
export function showDisplay(client: Client): Attachment<HTMLElement> {
  return (host) => {
    const element = client.getDisplay().getElement();
    host.append(element);
    const stop = forwardPointer(client);
    return () => {
      stop();
      element.remove();
    };
  };
}
