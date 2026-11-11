// Hand-written types for the vendored Apache Guacamole 1.5.5 client (`guacamole.js`, R-045, R-046). They cover
// only what Anima calls (the tunnel, the session view and their tests); every signature is taken from the 1.5.5
// source. Add a member only when a caller needs it.
//
// 1.5.5 against 1.5.0, for what Anima calls: element lengths are counted in code points both ways
// (`Guacamole.Parser`, `Parser.toInstruction`), so a non-BMP character no longer shifts the parse; the tunnel
// pings and the client's `nop` keep-alive are rescheduled timeouts instead of fixed intervals; a first internal
// instruction is the tunnel UUID only when it has exactly one argument. The client states are still private
// numbers (there is no `Client.State` in 1.5.5), and `Guacamole.API_VERSION` still reads "1.5.4".
//
// K4 §5 note: 1.5.5 still hides the WebSocket close code. `WebSocketTunnel` keeps its socket private and, on
// close, reports only a Guacamole status that `Status.Code.fromWebSocketCode` derives from the code (1002, 1011,
// 4001 and 4002 all become 0x0200 SERVER_ERROR). `tunnel/socket.ts` observes the code instead
// (`withObservedSocket`). The `Client` does not watch its tunnel: when the socket closes without a `disconnect`
// or `error` instruction, the client stays WAITING or CONNECTED until `disconnect()` is called.

declare namespace Guacamole {
  /** A Guacamole protocol status: a numeric code (0x0000 success, 0x0200.. errors) and an optional text. */
  class Status {
    readonly code: number;
    readonly message: string | undefined;
  }

  /** The base of every tunnel; `state` is one of `Tunnel.State`. */
  class Tunnel {
    state: number;
    /** Milliseconds of silence, from `connect` on, after which the tunnel closes itself (default 15000). */
    receiveTimeout: number;
    /** Sets `state` and calls `onstatechange` when it changed. */
    setState(state: number): void;
    onstatechange: ((state: number) => void) | null;
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

  /**
   * Writes bytes to an output stream as base64 `blob`s of at most 6048 bytes; `sendEnd` closes the stream. 1.5.5
   * copies `data` with `new Uint8Array(data)`, so pass bytes, not another typed array. Unlike `StringWriter`, whose
   * 1.5.5 encoder writes each UTF-16 unit on its own (a character outside the BMP becomes two invalid 3-byte
   * sequences), what it sends is exactly the bytes given.
   */
  class ArrayBufferWriter {
    constructor(stream: OutputStream);
    sendData(data: ArrayBuffer | Uint8Array): void;
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
    /** Draws the queued operations as one frame and calls `callback` once every earlier frame is drawn. */
    flush(callback?: () => void): void;
    scale(scale: number): void;
    getScale(): number;
    onresize: ((width: number, height: number) => void) | null;
  }

  /**
   * Plays back a recording (the instructions the hub tees, K4 §11) from a `Blob`; parsing starts at once. Frames
   * end at each `sync`; positions and the duration are milliseconds from the first `sync` timestamp. `seek`
   * picks the frame `findFrame` finds for `position` (a binary search; a time just after a frame gives that frame) (past the end it lands on the last frame) and calls `callback` when done.
   */
  class SessionRecording {
    constructor(source: Blob);
    getDisplay(): Display;
    getDuration(): number;
    getPosition(): number;
    isPlaying(): boolean;
    play(): void;
    pause(): void;
    seek(position: number, callback?: () => void): void;
    cancel(): void;
    abort(): void;
    onload: (() => void) | null;
    onerror: ((message: string) => void) | null;
    onabort: (() => void) | null;
    onprogress: ((duration: number, parsedSize: number) => void) | null;
    onplay: (() => void) | null;
    onpause: (() => void) | null;
    /** Called while a seek replays frames, with the position reached, frames done and frames to do. */
    onseek: ((position: number, current: number, total: number) => void) | null;
  }

  /**
   * The Guacamole client over one tunnel. `onstatechange` hears the private 1.5.5 states 0 IDLE, 1 CONNECTING,
   * 2 WAITING, 3 CONNECTED (first `sync`), 4 DISCONNECTING, 5 DISCONNECTED. A gateway `error` instruction calls
   * `onerror`, then `disconnect()`. The send methods do nothing unless the state is WAITING or CONNECTED.
   */
  class Client {
    constructor(tunnel: Tunnel);
    connect(data?: string): void;
    disconnect(): void;
    getDisplay(): Display;
    sendSize(width: number, height: number): void;
    /** 1.5.5 writes `pressed` as `'' + pressed`, so pass 1 or 0, never a boolean (guacd reads an integer). */
    sendKeyEvent(pressed: 0 | 1, keysym: number): void;
    sendMouseState(state: Mouse.State, applyDisplayScale?: boolean): void;
    /** Opens a `clipboard` stream with `mimetype`; write it with a `StringWriter`. */
    createClipboardStream(mimetype: string): OutputStream;
    /**
     * Opens an `argv` stream that asks the gateway to set connection parameter `name` to the stream's contents,
     * sending `argv` with the stream index, `mimetype` and `name`; write it with an `ArrayBufferWriter`.
     */
    createArgumentValueStream(mimetype: string, name: string): OutputStream;
    onstatechange: ((state: number) => void) | null;
    onerror: ((status: Status) => void) | null;
    onclipboard: ((stream: InputStream, mimetype: string) => void) | null;
    /** The gateway's `name` instruction (the connection name). */
    onname: ((name: string) => void) | null;
    /** The gateway's `required` instruction: the names of the connection parameters it needs to go on. */
    onrequired: ((parameters: string[]) => void) | null;
  }

  /**
   * A recorded session played on a client and display of its own (RA-041). 1.5.5 also takes a `Tunnel`; Anima
   * passes a Blob of the whole received text. Parsing starts in the constructor: the Blob is read as text in 256 KiB
   * blocks, a frame ends at each `sync`, and `onprogress` hears each frame. A frame is found again by the sum of its
   * elements' UTF-16 lengths and sliced from the Blob by bytes, so the text must be ASCII. Times are milliseconds
   * from the first frame. `seek` jumps to the frame at `position`, or to one next to it (a binary search, not
   * always the one before it), and calls `callback` when it is done or `cancel` ends it; `play` and `pause` drop a
   * seek in progress without calling it, and with no frame at all `seek` returns at once and never calls it. A
   * parse error calls `onerror` with the parser's message and stops reading.
   */
  class SessionRecording {
    constructor(source: Blob);
    getDisplay(): Display;
    isPlaying(): boolean;
    getPosition(): number;
    getDuration(): number;
    play(): void;
    pause(): void;
    seek(position: number, callback?: () => void): void;
    /** Ends a seek in progress where it is and calls its callback (which restarts play if it was playing). */
    cancel(): void;
    /** Stops reading the Blob; frames already read still play. */
    abort(): void;
    onload: (() => void) | null;
    onerror: ((message: string) => void) | null;
    onprogress: ((duration: number, parsedSize: number) => void) | null;
    onplay: (() => void) | null;
    onpause: (() => void) | null;
    /** Hears each frame a seek or play replays: its position, and how far the seek is, in frames. */
    onseek: ((position: number, current: number, total: number) => void) | null;
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

    /** Touch on `element` as a mouse: a tap is a click, a drag moves; it fires the same events as `Mouse`. */
    class Touchscreen {
      constructor(element: Element);
      onEach(types: readonly EventType[], listener: (event: Event) => void): void;
      offEach(types: readonly EventType[], listener: (event: Event) => void): boolean;
    }

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
