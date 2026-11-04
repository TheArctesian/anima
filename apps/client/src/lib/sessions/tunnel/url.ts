/** The WebSocket scheme for each hub scheme (K4 §2). Any other scheme has no tunnel. */
const SOCKET_SCHEMES: Readonly<Record<string, string>> = { 'https:': 'wss:', 'http:': 'ws:' };

/**
 * A tunnel path the hub may give: an absolute path of plain segments (no `.` or `..`), with no query, fragment
 * or host (K4 §1). A `//` start would name another host, so it never matches.
 */
const TUNNEL_PATH = /^(?:\/(?!\.{1,2}(?:\/|$))[A-Za-z0-9._~-]+)+$/;

/**
 * The tunnel endpoint for a hub at `origin` (`https://host[:port]`) and the ticket's `tunnelPath`, or `undefined`
 * when either is not one the client accepts. The ticket token is not part of it: the library appends
 * `?token=…` itself when it connects (D-3).
 */
export function tunnelUrl(origin: string, tunnelPath: string): string | undefined {
  if (!URL.canParse(origin) || !TUNNEL_PATH.test(tunnelPath)) return undefined;
  const hub = new URL(origin);
  const scheme = SOCKET_SCHEMES[hub.protocol];
  if (scheme === undefined || hub.username !== '' || hub.password !== '') return undefined;
  return `${scheme}//${hub.host}${tunnelPath}`;
}
