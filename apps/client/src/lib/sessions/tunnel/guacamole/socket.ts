import type { SocketConstructor } from '../state';

/**
 * Guacamole 1.5.5's `WebSocketTunnel` keeps its socket private and turns the close code into a Guacamole status,
 * which loses it (1002, 1011, 4001 and 4002 all become 0x0200). K4 §5 needs the code, so the tunnel observes it
 * here, without changing the vendored library.
 *
 * How: the library builds its socket with `new WebSocket(url, "guacamole")`, reading the global `WebSocket` at the
 * moment `connect` runs. `withObservedSocket` swaps that global for a subclass of the real constructor for the
 * length of that one synchronous call and always puts it back in `finally`. The subclass adds one `close`
 * listener to the first socket it constructs, before the library sets `onclose`, so the code is recorded before
 * the library reacts to the close; any further socket made in the window is not observed. While the global is
 * swapped only the library's `connect` and the tunnel's own hooks run (the tunnel holds back its `onchange` until
 * `connect` returns). If `open` throws after the socket was made, the socket is closed. No reference to the
 * socket (whose URL holds the ticket token) outlives the call.
 */


/**
 * Run `open` (a synchronous call that creates one WebSocket) with the global `WebSocket` replaced by an observing
 * subclass of `base`; the global is always restored, and the socket closed when `open` throws.
 */
export function withObservedSocket(base: SocketConstructor, onClose: (code: number) => void, open: () => void): void {
  // Closes the first socket made; set by that socket's constructor, cleared when the call ends.
  let closeFirst: (() => void) | undefined;
  const Observed = class ObservedSocket extends base {
    constructor(url: string | URL, protocols?: string | string[]) {
      super(url, protocols);
      if (closeFirst !== undefined) return;
      closeFirst = () => {
        this.close();
      };
      this.addEventListener('close', (event) => {
        onClose(event.code);
      });
    }
  };
  const saved = globalThis.WebSocket;
  globalThis.WebSocket = Observed;
  try {
    open();
  } catch (error: unknown) {
    closeFirst?.();
    throw error;
  } finally {
    globalThis.WebSocket = saved;
    closeFirst = undefined;
  }
}
