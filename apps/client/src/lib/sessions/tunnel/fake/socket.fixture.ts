// Test support: the hub's side of one session WebSocket, driven by hand under the real Guacamole client
// (K4 §4, §5). Pass `FAKE_TUNNEL_SOCKET` as `WebSocket` to `openTunnel` or `startTunnel`. No vitest import here.
import type { SocketConstructor } from '../state';

type Listener = (event: { code: number; reason: string; wasClean: boolean }) => void;

/** How many Unicode code points `text` holds: the unit of every Guacamole element length (guacd, the hub). */
export function codePoints(text: string): number {
  return [...text].length;
}

/**
 * One Guacamole instruction as the hub relays it: `<len>.<opcode>,<len>.<arg>,…;`, each length in code points
 * (K4 §4). A character outside the BMP counts once here but twice in a JavaScript string's `length`.
 */
export function encodeInstruction(opcode: string, ...args: readonly (string | number)[]): string {
  return `${[opcode, ...args].map((element) => `${String(codePoints(String(element)))}.${String(element)}`).join(',')};`;
}

/** Split one message into instructions, reading each element's length in code points; throws on a malformed one. */
export function decodeInstructions(message: string): string[][] {
  const points = [...message];
  const instructions: string[][] = [];
  let elements: string[] = [];
  let at = 0;
  while (at < points.length) {
    const dot = points.indexOf('.', at);
    const length = Number(points.slice(at, dot).join(''));
    if (dot <= at || !Number.isInteger(length) || length < 0) throw new Error(`malformed instruction at ${String(at)}`);
    const end = dot + 1 + length;
    elements.push(points.slice(dot + 1, end).join(''));
    const terminator = points[end];
    if (terminator === ';') {
      instructions.push(elements);
      elements = [];
    } else if (terminator !== ',') throw new Error(`malformed instruction at ${String(end)}`);
    at = end + 1;
  }
  return instructions;
}

/** Base64 of UTF-8 text, as a `blob` carries it. */
function base64(text: string): string {
  return btoa(String.fromCharCode(...new TextEncoder().encode(text)));
}

export class FakeTunnelSocket {
  static readonly CONNECTING = 0;
  static readonly OPEN = 1;
  static readonly CLOSING = 2;
  static readonly CLOSED = 3;
  /** Every socket made since the last `reset`, oldest first. */
  static made: FakeTunnelSocket[] = [];

  static reset(): void {
    FakeTunnelSocket.made = [];
  }

  /** The latest socket; throws when none was made. */
  static latest(): FakeTunnelSocket {
    const socket = FakeTunnelSocket.made.at(-1);
    if (socket === undefined) throw new Error('no tunnel socket was made');
    return socket;
  }

  readyState = FakeTunnelSocket.CONNECTING;
  /** Raw text messages the client sent. */
  readonly sent: string[] = [];
  /** The code the client closed with, when it closed first. */
  closedByClient: number | undefined;
  onopen: ((event: unknown) => void) | null = null;
  onclose: Listener | null = null;
  onerror: ((event: unknown) => void) | null = null;
  onmessage: ((event: { data: string }) => void) | null = null;
  private readonly closeListeners: Listener[] = [];

  constructor(
    readonly url: string,
    readonly protocol?: string | string[],
  ) {
    FakeTunnelSocket.made.push(this);
  }

  addEventListener(type: string, listener: Listener): void {
    if (type === 'close') this.closeListeners.push(listener);
  }

  send(text: string): void {
    this.sent.push(text);
  }

  /** The client's instructions, decoded, without its internal pings. */
  sentInstructions(): string[][] {
    return this.sent.flatMap(decodeInstructions).filter(([opcode]) => opcode !== '');
  }

  /** The client closes; the hub answers the close handshake with `code` (default 1000) on the next microtask. */
  close(code = 1000): void {
    if (this.readyState >= FakeTunnelSocket.CLOSING) return;
    this.readyState = FakeTunnelSocket.CLOSING;
    this.closedByClient = code;
    queueMicrotask(() => {
      this.finish(code);
    });
  }

  /** The upgrade succeeds and the hub sends its internal `0.,36.<uuid>;` instruction, which opens the tunnel. */
  open(uuid = '00000000-0000-4000-8000-000000000000'): void {
    this.readyState = FakeTunnelSocket.OPEN;
    this.onopen?.({});
    this.receive(encodeInstruction('', uuid));
  }

  /** The hub relays one message (one or more complete instructions). */
  receive(message: string): void {
    this.onmessage?.({ data: message });
  }

  /** The gateway sends one instruction. */
  instruction(opcode: string, ...args: readonly (string | number)[]): void {
    this.receive(encodeInstruction(opcode, ...args));
  }

  /** The gateway's first frame: the client turns CONNECTED. */
  sync(timestamp = 0): void {
    this.instruction('sync', timestamp);
  }

  /** The gateway sends `text` as clipboard stream `stream` (`clipboard`, one `blob`, `end`). */
  clipboard(text: string, stream = 0): void {
    this.instruction('clipboard', stream, 'text/plain');
    this.instruction('blob', stream, base64(text));
    this.instruction('end', stream);
  }

  /** The hub relays a gateway `error` with its fixed text (R-035). */
  gatewayError(status: number): void {
    this.instruction('error', 'Aborted. See logs.', status);
  }

  /** The gateway sends `disconnect`. */
  disconnect(): void {
    this.instruction('disconnect');
  }

  /** The hub closes the socket with `code` (K4 §5); 1006 before `open` is a refused upgrade. */
  closeWith(code: number): void {
    this.finish(code);
  }

  private finish(code: number): void {
    if (this.readyState === FakeTunnelSocket.CLOSED) return;
    this.readyState = FakeTunnelSocket.CLOSED;
    const event = { code, reason: '', wasClean: code === 1000 };
    for (const listener of this.closeListeners) listener(event);
    this.onclose?.(event);
  }
}

/** `FakeTunnelSocket` typed as the WebSocket constructor the tunnel takes. */
export const FAKE_TUNNEL_SOCKET = FakeTunnelSocket as unknown as SocketConstructor;
