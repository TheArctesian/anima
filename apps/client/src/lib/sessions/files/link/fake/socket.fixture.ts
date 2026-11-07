// Test support: the hub's side of one files WebSocket (K4 §9), driven by hand. Pass `FAKE_FILES_SOCKET` as
// `WebSocket` to `openFilesLink`. `bufferedAmount` is set by the test to play a slow network. No vitest import here.
import type { SocketConstructor } from '../../../tunnel/state';

interface CloseEvent {
  code: number;
  reason: string;
  wasClean: boolean;
}

/** One frame the client sent: text, or binary bytes. */
export type SentFrame = string | ArrayBuffer;

export class FakeFilesSocket {
  static readonly CONNECTING = 0;
  static readonly OPEN = 1;
  static readonly CLOSING = 2;
  static readonly CLOSED = 3;
  /** Every socket made since the last `reset`, oldest first. */
  static made: FakeFilesSocket[] = [];

  static reset(): void {
    FakeFilesSocket.made = [];
  }

  /** The latest socket; throws when none was made. */
  static latest(): FakeFilesSocket {
    const socket = FakeFilesSocket.made.at(-1);
    if (socket === undefined) throw new Error('no files socket was made');
    return socket;
  }

  readyState = FakeFilesSocket.CONNECTING;
  binaryType: BinaryType = 'blob';
  /** Unsent bytes the client sees; the test sets it. */
  bufferedAmount = 0;
  /** Every frame the client sent, in order. */
  readonly frames: SentFrame[] = [];
  /** The code the client closed with, when it closed first. */
  closedByClient: number | undefined;
  onopen: ((event: unknown) => void) | null = null;
  onclose: ((event: CloseEvent) => void) | null = null;
  onerror: ((event: unknown) => void) | null = null;
  onmessage: ((event: { data: unknown }) => void) | null = null;

  constructor(
    readonly url: string,
    readonly protocol?: string | string[],
  ) {
    FakeFilesSocket.made.push(this);
  }

  send(frame: SentFrame): void {
    this.frames.push(frame);
  }

  /** The text messages the client sent, parsed. */
  messages(): Record<string, unknown>[] {
    return this.frames.filter((frame): frame is string => typeof frame === 'string').map((text) => JSON.parse(text) as Record<string, unknown>);
  }

  /** The `type` of each text message the client sent, in order. */
  types(): unknown[] {
    return this.messages().map((message) => message.type);
  }

  /** The byte length of each binary frame the client sent, in order. */
  binarySizes(): number[] {
    return this.frames.filter((frame): frame is ArrayBuffer => typeof frame !== 'string').map((frame) => frame.byteLength);
  }

  /** The client closes; the hub answers the close handshake with the same code on the next microtask. */
  close(code = 1000): void {
    if (this.readyState >= FakeFilesSocket.CLOSING) return;
    this.readyState = FakeFilesSocket.CLOSING;
    this.closedByClient = code;
    queueMicrotask(() => {
      this.finish(code);
    });
  }

  /** The upgrade succeeds. */
  open(): void {
    this.readyState = FakeFilesSocket.OPEN;
    this.onopen?.({});
  }

  /** The hub sends one text message: an object is sent as JSON, a string as it is. */
  text(message: unknown): void {
    this.onmessage?.({ data: typeof message === 'string' ? message : JSON.stringify(message) });
  }

  /** The hub sends `length` bytes of download data, each `fill`. */
  bytes(length: number, fill = 7): void {
    // Built here, not taken from a typed array of another realm: the link tests `instanceof ArrayBuffer`.
    const buffer = new ArrayBuffer(length);
    new Uint8Array(buffer).fill(fill);
    this.onmessage?.({ data: buffer });
  }

  /** The hub closes the socket with `code`; 1006 before `open` is a refused upgrade. */
  closeWith(code: number): void {
    this.finish(code);
  }

  private finish(code: number): void {
    if (this.readyState === FakeFilesSocket.CLOSED) return;
    this.readyState = FakeFilesSocket.CLOSED;
    this.onclose?.({ code, reason: '', wasClean: code === 1000 });
  }
}

/** `FakeFilesSocket` typed as the WebSocket constructor the link takes. */
export const FAKE_FILES_SOCKET = FakeFilesSocket as unknown as SocketConstructor;
