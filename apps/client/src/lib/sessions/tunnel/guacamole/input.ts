import Guacamole from '../vendor/guacamole.js';
import type { SessionTunnel } from './tunnel';

type Client = SessionTunnel['client'];

/** Where key events go while the keyboard is bound; `down` returns true to let the browser handle the key. */
export interface KeyRoute {
  down(keysym: number): boolean;
  up(keysym: number): void;
}

/**
 * The keyboard of one session pane (D-8): a `Guacamole.Keyboard` that listens to `pane` only, never the document.
 * 1.5.5 cannot remove its listeners, so one keyboard serves the pane for its whole life and is switched by its
 * handlers: with none set, every listener ignores its event.
 */
export interface PaneKeyboard {
  bind(route: KeyRoute): void;
  /** Release every key still held (through the bound route, so the remote has no stuck key), then stop listening. */
  unbind(): void;
}

export function paneKeyboard(pane: HTMLElement): PaneKeyboard {
  const keyboard = new Guacamole.Keyboard(pane);
  return {
    bind: (route) => {
      keyboard.onkeydown = (keysym) => route.down(keysym);
      keyboard.onkeyup = (keysym) => {
        route.up(keysym);
      };
    },
    unbind: () => {
      // `reset` also stops the key-repeat timer, which would otherwise call the handlers after they are null.
      if (keyboard.onkeyup !== null) keyboard.reset();
      keyboard.onkeydown = null;
      keyboard.onkeyup = null;
    },
  };
}

const MOUSE_EVENTS: readonly Guacamole.Mouse.EventType[] = ['mousedown', 'mouseup', 'mousemove'];

/**
 * Send mouse and touch input on the client's display element to the gateway (D-8). The wheel arrives as presses
 * of the `up` and `down` buttons, which scroll guacd's scroll-back (RA-003). Returns the function that stops it.
 */
export function forwardPointer(client: Client): () => void {
  const element = client.getDisplay().getElement();
  const pointers = [new Guacamole.Mouse(element), new Guacamole.Mouse.Touchscreen(element)];
  const send = (event: Guacamole.Mouse.Event) => {
    client.sendMouseState(event.state, true);
  };
  for (const pointer of pointers) pointer.onEach(MOUSE_EVENTS, send);
  return () => {
    for (const pointer of pointers) pointer.offEach(MOUSE_EVENTS, send);
  };
}

/** Clipboard text longer than this (UTF-16 units) from the gateway is dropped whole, not cut. */
export const CLIPBOARD_LIMIT = 1_048_576;

const PLAIN_TEXT = 'text/plain';

/**
 * Hear the gateway's clipboard (D-9): each `text/plain` stream, read with a `StringReader`, is given to `deliver`
 * once it ends. Other types and text over `CLIPBOARD_LIMIT` are ignored. The text is held only while its stream
 * is open; nothing here logs or keeps it.
 */
export function receiveClipboard(client: Client, deliver: (text: string) => void): void {
  client.onclipboard = (stream, mimetype) => {
    if (mimetype.split(';')[0]?.trim().toLowerCase() !== PLAIN_TEXT) return;
    const reader = new Guacamole.StringReader(stream);
    let text: string | undefined = '';
    reader.ontext = (part) => {
      if (text === undefined) return;
      text = text.length + part.length > CLIPBOARD_LIMIT ? undefined : text + part;
    };
    reader.onend = () => {
      if (text !== undefined) deliver(text);
      text = undefined;
    };
  };
}

/** Send `text` to the gateway as one `text/plain` clipboard stream (D-9). */
export function sendClipboard(client: Client, text: string): void {
  const writer = new Guacamole.StringWriter(client.createClipboardStream(PLAIN_TEXT));
  writer.sendText(text);
  writer.sendEnd();
}
