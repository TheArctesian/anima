/**
 * Guacamole 1.5.0's `WebSocketTunnel` keeps its socket private and turns the close code into a Guacamole status,
 * which loses it (1002, 1011, 4001 and 4002 all become 0x0200). K4 §5 needs the code, so the tunnel observes it
 * here, without changing the vendored library.
 *
 * How: the library builds its socket with `new WebSocket(url, "guacamole")`, reading the global `WebSocket` at the
 * moment `connect` runs. `withObservedSocket` swaps that global for a subclass of the real constructor for the
 * length of that one synchronous call, then puts it back in `finally`. The subclass adds one `close` listener in
 * its constructor, before the library sets `onclose`, so the code is recorded before the library reacts to the
 * close. No other code runs while the global is swapped, and no reference to the socket (whose URL holds the
 * ticket token) is kept.
 */

/** The WebSocket constructor the library reads; tests pass a fake one (CQ-021). */
export type SocketConstructor = typeof WebSocket;

/** A constructor that behaves like `base` and reports each socket's close code to `onClose`. */
function observing(base: SocketConstructor, onClose: (code: number) => void): SocketConstructor {
  return class ObservedSocket extends base {
    constructor(url: string | URL, protocols?: string | string[]) {
      super(url, protocols);
      this.addEventListener('close', (event) => {
        onClose(event.code);
      });
    }
  };
}

/**
 * Run `open` (a synchronous call that creates one WebSocket) with the global `WebSocket` replaced by an observing
 * subclass of `base`; the global is always restored, also when `open` throws.
 */
export function withObservedSocket(base: SocketConstructor, onClose: (code: number) => void, open: () => void): void {
  const saved = globalThis.WebSocket;
  globalThis.WebSocket = observing(base, onClose);
  try {
    open();
  } finally {
    globalThis.WebSocket = saved;
  }
}
