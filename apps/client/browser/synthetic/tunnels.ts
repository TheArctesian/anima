import type { WebSocketRoute } from '@playwright/test';
import { randomUUID } from 'node:crypto';
import { HUB_ORIGIN } from './access';
import type { HubState } from './model';
import type { Recording } from './recorder';

// `/terminal` (K4 section 8) and `/tunnel` (K4 sections 2 to 4): the smallest answer that shows a session opened.
// The hub refuses a bad ticket with HTTP 403 before the upgrade; a mocked socket cannot, so it closes at once.

export const TICKET_REFUSED_CLOSE = 1008;
const PROMPT = 'ann@example-host:~$ ';
const PAGE_WIDTH = 1024;
const PAGE_HEIGHT = 768;

export interface TunnelParts {
  readonly state: HubState;
  readonly recording: Recording;
}

/** One Guacamole instruction: each element length-prefixed in characters, as the protocol writes it. */
export function instruction(opcode: string, ...args: readonly (string | number)[]): string {
  const elements = [opcode, ...args.map(String)];
  return `${elements.map((element) => `${String(element.length)}.${element}`).join(',')};`;
}

function redeem(parts: TunnelParts, socket: WebSocketRoute, path: string): boolean {
  const url = new URL(socket.url());
  const result = parts.state.tickets.redeem(url.searchParams.get('token'), path, parts.state.now());
  parts.recording.record({
    channel: path === '/terminal' ? 'terminal' : 'tunnel',
    operation: result.ok ? 'ticket accepted' : 'ticket refused',
    variables: null,
    hadAuthorization: false,
    hadCookie: false,
    origin: HUB_ORIGIN,
    signedIn: result.ok,
  });
  if (!result.ok) socket.close({ code: TICKET_REFUSED_CLOSE, reason: 'Forbidden' });
  return result.ok;
}

/** SSH bytes: the prompt, once the client's first `resize` has opened the PTY (K4 section 8.4). */
export function serveTerminal(parts: TunnelParts, socket: WebSocketRoute): void {
  if (!redeem(parts, socket, '/terminal')) return;
  let opened = false;
  socket.onMessage((message) => {
    if (opened || typeof message !== 'string' || !message.includes('"resize"')) return;
    opened = true;
    socket.send(Buffer.from(PROMPT));
  });
}

/** Guacamole: the internal open instruction, then a one-colour display and a `sync` (K4 section 4). */
export function serveGuacamole(parts: TunnelParts, socket: WebSocketRoute): void {
  if (!redeem(parts, socket, '/tunnel')) return;
  const stamp = String(parts.state.now().getTime());
  socket.send(instruction('', randomUUID()));
  for (const frame of [
    instruction('size', 0, PAGE_WIDTH, PAGE_HEIGHT),
    instruction('rect', 0, 0, 0, PAGE_WIDTH, PAGE_HEIGHT),
    instruction('cfill', 12, 0, 32, 64, 96, 255),
    instruction('sync', stamp),
  ]) {
    socket.send(frame);
  }
  socket.onMessage((message) => {
    // The hub answers the internal `ping` with the same instruction and forwards nothing else here.
    if (typeof message === 'string' && message.startsWith('0.,4.ping')) socket.send(message);
  });
}
