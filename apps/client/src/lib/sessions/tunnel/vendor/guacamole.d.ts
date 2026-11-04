// Hand-written types for the vendored Apache Guacamole 1.5.0 client (`guacamole.js`, R-045). They cover only
// what Anima calls; every signature is taken from the 1.5.0 source. Add a member only when a caller needs it.
//
// K4 §5 note: 1.5.0 hides the WebSocket close code. `WebSocketTunnel` keeps its socket private and, on close,
// reports only a Guacamole status that `Status.Code.fromWebSocketCode` derives from the code (1002, 1011, 4001
// and 4002 all become 0x0200 SERVER_ERROR). `tunnel/socket.ts` observes the code instead (`withObservedSocket`).
// The 1.5.0 `Client` does not watch its tunnel: when the socket closes without a `disconnect` or `error`
// instruction, the client stays WAITING or CONNECTED until `disconnect()` is called.

declare namespace Guacamole {
  /** A Guacamole protocol status: a numeric code (0x0000 success, 0x0200.. errors) and an optional text. */
  class Status {
    constructor(code: number, message?: string);
    readonly code: number;
    readonly message: string | undefined;
    isError(): boolean;
  }

  /** The base of every tunnel; `state` is one of `Tunnel.State`. */
  class Tunnel {
    state: number;
    /** Sets `state` and calls `onstatechange` when it changed. */
    setState(state: number): void;
    disconnect(): void;
    onstatechange: ((state: number) => void) | null;
    onerror: ((status: Status) => void) | null;
  }

  namespace Tunnel {
    const State: { readonly CONNECTING: 0; readonly OPEN: 1; readonly CLOSED: 2; readonly UNSTABLE: 3 };
  }

  /**
   * A tunnel over one WebSocket with subprotocol `guacamole`. `connect(data)` opens `tunnelURL + "?" + data`.
   * The state turns OPEN when the first instruction arrives (the hub's internal `0.,36.<uuid>;`, K4 §4).
   */
  class WebSocketTunnel extends Tunnel {
    constructor(tunnelURL: string);
  }

  /** A stream the client writes, such as a clipboard stream. */
  class OutputStream {
    readonly index: number;
  }

  /** A stream the gateway writes, such as the `clipboard` it sends. */
  class InputStream {
    readonly index: number;
  }

  /** Writes text to an output stream as base64 `blob`s; `sendEnd` closes the stream. */
  class StringWriter {
    constructor(stream: OutputStream);
    sendText(text: string): void;
    sendEnd(): void;
  }

  /** Reads text from an input stream; `ontext` hears each decoded part, `onend` the end. */
  class StringReader {
    constructor(stream: InputStream);
    ontext: ((text: string) => void) | null;
    onend: (() => void) | null;
  }

  /** The remote display. Its element holds the layers; Anima never transforms or animates it (MAT-008). */
  class Display {
    getElement(): HTMLDivElement;
    getWidth(): number;
    getHeight(): number;
    scale(scale: number): void;
    getScale(): number;
    onresize: ((width: number, height: number) => void) | null;
  }

  /**
   * The Guacamole client over one tunnel. `onstatechange` hears the private 1.5.0 states 0 IDLE, 1 CONNECTING,
   * 2 WAITING, 3 CONNECTED (first `sync`), 4 DISCONNECTING, 5 DISCONNECTED. A gateway `error` instruction calls
   * `onerror`, then `disconnect()`. The send methods do nothing unless the state is WAITING or CONNECTED.
   */
  class Client {
    constructor(tunnel: Tunnel);
    connect(data?: string): void;
    disconnect(): void;
    getDisplay(): Display;
    sendSize(width: number, height: number): void;
    /** 1.5.0 writes `pressed` with `String()`, so pass 1 or 0, never a boolean (guacd reads an integer). */
    sendKeyEvent(pressed: 0 | 1, keysym: number): void;
    sendMouseState(state: Mouse.State, applyDisplayScale?: boolean): void;
    /** Opens a `clipboard` stream with `mimetype`; write it with a `StringWriter`. */
    createClipboardStream(mimetype: string): OutputStream;
    onstatechange: ((state: number) => void) | null;
    onerror: ((status: Status) => void) | null;
    onclipboard: ((stream: InputStream, mimetype: string) => void) | null;
  }

  /**
   * Keyboard input from `element` (or `listenTo` targets). 1.5.0 has no way to remove its DOM listeners:
   * unbinding means setting `onkeydown` and `onkeyup` to null, which makes the listeners ignore every event.
   */
  class Keyboard {
    constructor(element?: Element | Document);
    /** Return false to stop the browser handling the key. */
    onkeydown: ((keysym: number) => boolean | undefined) | null;
    onkeyup: ((keysym: number) => void) | null;
    listenTo(element: Element | Document): void;
    press(keysym: number): boolean | undefined;
    release(keysym: number): void;
    /** Releases every pressed key, firing `onkeyup` for each. */
    reset(): void;
  }

  /** Mouse input on `element`; the wheel becomes presses of the `up` and `down` buttons. */
  class Mouse {
    constructor(element: Element);
    on(type: Mouse.EventType, listener: (event: Mouse.Event) => void): void;
    onEach(types: readonly Mouse.EventType[], listener: (event: Mouse.Event) => void): void;
    off(type: Mouse.EventType, listener: (event: Mouse.Event) => void): boolean;
    offEach(types: readonly Mouse.EventType[], listener: (event: Mouse.Event) => void): boolean;
  }

  namespace Mouse {
    type EventType = 'mousedown' | 'mouseup' | 'mousemove' | 'mouseout';

    interface StateTemplate {
      readonly x?: number;
      readonly y?: number;
      readonly left?: boolean;
      readonly middle?: boolean;
      readonly right?: boolean;
      readonly up?: boolean;
      readonly down?: boolean;
    }

    class State {
      constructor(template?: StateTemplate);
      x: number;
      y: number;
      left: boolean;
      middle: boolean;
      right: boolean;
      up: boolean;
      down: boolean;
    }

    class Event {
      readonly type: EventType;
      readonly state: State;
    }
  }
}

export default Guacamole;
