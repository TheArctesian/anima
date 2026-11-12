// The probe's requests to one hub origin (criteria 4, 12, 13; A-2): HTTP answers read whole up to a size limit,
// and one cross-origin `graphql-transport-ws` socket. TLS is always verified; a CA file only adds trust. Redirects
// are never followed, so a header never leaves the origin.
// Verification is stated on every request, so no environment variable can turn it off. A failed request is
// `undefined`, never an error text.
import { FOREIGN_ORIGIN, type SocketOutcome } from "./refusals.ts";

const TIMEOUT_MS = 10_000;
const BODY_LIMIT = 16 * 1024 * 1024;
const LOOPBACK = /^(?:localhost|127(?:\.\d{1,3}){3}|\[::1\])$/;

/** What the probe talks to: the hub's origin and, for a local hub, an extra trusted certificate. */
export interface Target {
  origin: URL;
  ca: string | undefined;
}

/** One whole HTTP answer. */
export interface Answer {
  status: number;
  headers: Headers;
  body: string;
}

/** The hub origin from the command line: https, or http for a loopback host; no path, query, fragment or user. */
export function hubOrigin(text: string): URL | undefined {
  let url: URL;
  try {
    url = new URL(text);
  } catch {
    return undefined;
  }
  const plain = url.protocol === "http:" && LOOPBACK.test(url.hostname);
  const bare = url.username === "" && url.password === "" && url.pathname === "/" && !/[?#]/.test(text);
  return (url.protocol === "https:" || plain) && bare ? url : undefined;
}

const PROXY_VARIABLES = new Set(["http_proxy", "https_proxy", "all_proxy"]);

/**
 * Whether a proxy variable is set, in either case. Bun's fetch would use it: a proxy can forge a 401 or 403 to a
 * CONNECT, and on loopback http it would see the key (S-S2).
 */
export function proxyConfigured(env: Readonly<Record<string, string | undefined>>): boolean {
  return Object.entries(env).some(([name, value]) => PROXY_VARIABLES.has(name.toLowerCase()) && (value ?? "") !== "");
}

/** Verification on, always: stated here so `NODE_TLS_REJECT_UNAUTHORIZED=0` cannot turn it off (S-M1). */
function tls(target: Target): { rejectUnauthorized: true; ca?: string } {
  return target.ca === undefined ? { rejectUnauthorized: true } : { rejectUnauthorized: true, ca: target.ca };
}

/** The body as text, or undefined past the size limit. */
async function body(response: Response): Promise<string | undefined> {
  const chunks: Uint8Array[] = [];
  let size = 0;
  const reader = response.body?.getReader();
  for (let next = await reader?.read(); next !== undefined && !next.done; next = await reader?.read()) {
    size += next.value.byteLength;
    if (size > BODY_LIMIT) {
      await reader?.cancel();
      return undefined;
    }
    chunks.push(next.value);
  }
  return new TextDecoder().decode(Buffer.concat(chunks));
}

/** One request to the hub; undefined when no whole answer came (network, TLS, timeout or size). */
export async function ask(target: Target, path: string, init: RequestInit = {}): Promise<Answer | undefined> {
  try {
    const response = await fetch(new URL(path, target.origin), {
      ...init,
      redirect: "manual",
      signal: AbortSignal.timeout(TIMEOUT_MS),
      tls: tls(target),
    });
    const text = await body(response);
    return text === undefined ? undefined : { status: response.status, headers: response.headers, body: text };
  } catch {
    return undefined;
  }
}

/** A `POST /graphql` of one document with these extra headers. */
export function postGraphql(target: Target, query: string, headers: Record<string, string>): Promise<Answer | undefined> {
  const sent = { "content-type": "application/json", ...headers };
  return ask(target, "/graphql", { method: "POST", headers: sent, body: JSON.stringify({ query }) });
}

/** A CORS preflight for `POST /graphql` from `origin`, as the desktop web view sends it. */
export function preflight(target: Target, origin: string): Promise<Answer | undefined> {
  const headers = {
    origin,
    "access-control-request-method": "POST",
    "access-control-request-headers": "authorization,content-type",
  };
  return ask(target, "/graphql", { method: "OPTIONS", headers });
}

/** A synthetic browser cookie: random, never a real token, and never printed. */
export function fakeCookie(): string {
  return `__Host-anima=${Buffer.from(crypto.getRandomValues(new Uint8Array(32))).toString("base64url")}`;
}

/** The `type` of a `graphql-transport-ws` message, or undefined when it is not one. */
function messageType(data: unknown): unknown {
  try {
    return (JSON.parse(String(data)) as { type?: unknown } | null)?.type;
  } catch {
    return undefined;
  }
}

/** What a foreign-origin `graphql-transport-ws` socket with a fake cookie does after an empty `connection_init`. */
export function socketOutcome(target: Target): Promise<SocketOutcome> {
  const url = new URL("/graphql", target.origin);
  url.protocol = target.origin.protocol === "https:" ? "wss:" : "ws:";
  return new Promise((resolve) => {
    let opened = false;
    const socket = new WebSocket(url, {
      headers: { origin: FOREIGN_ORIGIN, cookie: fakeCookie() },
      protocols: ["graphql-transport-ws"],
      tls: tls(target),
    });
    const finish = (outcome: SocketOutcome) => {
      clearTimeout(timer);
      socket.onclose = null;
      socket.onmessage = null;
      socket.close();
      resolve(outcome);
    };
    const timer = setTimeout(() => {
      finish({ kind: "silent" });
    }, TIMEOUT_MS);
    socket.onopen = () => {
      opened = true;
      socket.send(JSON.stringify({ type: "connection_init" }));
    };
    socket.onmessage = (event) => {
      finish({ kind: "message", acknowledged: messageType(event.data) === "connection_ack" });
    };
    // Closed before it opened: refused upgrade, TLS or network; the hub's answer is 4403 after an open.
    socket.onclose = (event) => {
      finish(opened ? { kind: "closed", code: event.code } : { kind: "not-opened" });
    };
  });
}
