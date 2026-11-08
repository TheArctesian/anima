import { createServer, type Server } from 'node:http';
import type { AddressInfo } from 'node:net';

// The wall under every test browser (WP-44 ruling S3): all three engines are launched with this proxy, so a connection a
// browser makes on its own, past `context.route` (a preconnect, a CSP report), has nowhere to go. The proxy refuses every
// request and every CONNECT and records the host the browser asked for. It never connects out and never sees a path, a
// query or a body (LOG-007): a CONNECT carries only a host, and a plain request is refused before its body is read.

export interface Attempt {
  readonly via: 'CONNECT' | 'HTTP';
  /** `host:port` as the browser asked for it. */
  readonly target: string;
}

export interface DenyProxy {
  /** `http://127.0.0.1:<port>`, the Playwright `proxy.server`. */
  readonly url: string;
  attempts(): Promise<readonly Attempt[]>;
  close(): Promise<void>;
}

const ATTEMPTS_PATH = '/attempts';
const REFUSED = 403;

/** The `host:port` of an absolute-form request target (`http://host/path`), without the path or query. */
function targetOfAbsolute(requestUrl: string): string {
  if (!URL.canParse(requestUrl)) return 'unparsable';
  const url = new URL(requestUrl);
  return url.port === '' ? `${url.hostname}:80` : url.host;
}

function listening(server: Server): Promise<number> {
  return new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', () => resolve((server.address() as AddressInfo).port));
  });
}

/** Starts the proxy on a free loopback port. */
export async function startDenyProxy(): Promise<DenyProxy> {
  const seen: Attempt[] = [];
  const server = createServer((request, response) => {
    const requestUrl = request.url ?? '';
    // Origin-form (`/attempts`) is a request to the proxy itself; a proxied request is absolute-form.
    if (requestUrl === ATTEMPTS_PATH) {
      response.writeHead(200, { 'content-type': 'application/json' }).end(JSON.stringify(seen));
      return;
    }
    seen.push({ via: 'HTTP', target: targetOfAbsolute(requestUrl) });
    response.writeHead(REFUSED, { connection: 'close' }).end();
  });
  server.on('connect', (request, socket) => {
    seen.push({ via: 'CONNECT', target: request.url ?? 'unparsable' });
    socket.on('error', () => undefined);
    socket.end(`HTTP/1.1 ${String(REFUSED)} Forbidden\r\nConnection: close\r\n\r\n`);
  });
  const port = await listening(server);
  const url = `http://127.0.0.1:${String(port)}`;
  return {
    url,
    async attempts() {
      // eslint-disable-next-line no-restricted-globals -- test code in Node: it reads the proxy's own record, not the hub
      const response = await fetch(`${url}${ATTEMPTS_PATH}`);
      return (await response.json()) as Attempt[];
    },
    close: () =>
      new Promise((resolve) => {
        server.closeAllConnections();
        server.close(() => resolve());
      }),
  };
}
