/**
 * A tunnel path the hub may give: an absolute path of plain segments (no `.` or `..`), with no query, fragment
 * or host (K4 §1). A `//` start would name another host, so it never matches.
 */
const TUNNEL_PATH = /^(?:\/(?!\.{1,2}(?:\/|$))[A-Za-z0-9._~-]+)+$/;

/** Loopback host names as `URL` writes them: `localhost`, 127.0.0.0/8 and `[::1]`. */
const LOOPBACK = /^(?:localhost|127(?:\.\d{1,3}){3}|\[::1\])$/;

/**
 * The WebSocket scheme for a hub URL: wss for https; ws only for http to a loopback host (a hub run locally for
 * development), since anything else would send the ticket token in clear text (SEC-002).
 */
function socketScheme(hub: URL): string | undefined {
  if (hub.protocol === 'https:') return 'wss:';
  if (hub.protocol === 'http:' && LOOPBACK.test(hub.hostname)) return 'ws:';
  return undefined;
}

/**
 * The tunnel endpoint for a hub at `origin` (`https://host[:port]`) and the ticket's `tunnelPath`, or `undefined`
 * when either is not one the client accepts. The ticket token is not part of it: the library appends
 * `?token=…` itself when it connects (D-3).
 */
export function tunnelUrl(origin: string, tunnelPath: string): string | undefined {
  if (!URL.canParse(origin) || !TUNNEL_PATH.test(tunnelPath)) return undefined;
  const hub = new URL(origin);
  const scheme = socketScheme(hub);
  if (scheme === undefined || hub.username !== '' || hub.password !== '') return undefined;
  return `${scheme}//${hub.host}${tunnelPath}`;
}
