// A synthetic hub on 127.0.0.1 for deploy/acceptance/probe.ts (criteria 4, 12, 13). By default it answers as the
// real hub does (K7 headers, CORS for the client origins, 401 on an open /metrics, 4403 on a foreign socket,
// FORBIDDEN for a READ key); each fault makes one answer wrong. Every value is synthetic.
import { createHash } from "node:crypto";
import type { Server } from "bun";
import { CLIENT_ORIGINS, FOREIGN_ORIGIN, READ_ONLY_MESSAGE, REFUSED_ROOTS } from "../acceptance/refusals.ts";

export type Fault =
  | "health-not-ok"
  | "csp-unsafe-script"
  | "nosniff-missing"
  | "index-immutable"
  | "asset-no-cache"
  | "asset-hsts-missing"
  | "cookie-signs-in"
  | "socket-acknowledges"
  | "cors-client-missing"
  | "cors-foreign-allowed"
  | "metrics-open"
  | "metrics-content-type"
  | "metrics-undeclared-family"
  | "metrics-family-missing"
  | "metrics-unknown-label"
  | "metrics-volatile-label"
  | "key-writes"
  | "root-deleteDevice-allowed"
  | "roots-read-only"
  | "metrics-redirect"
  | "socket-down"
  | "cookie-set";

const SCRIPT = 'import("/_app/immutable/entry/start.Ab1_cD.js").then((kit) => kit.start());';
const INDEX = `<!doctype html><html><head><link href="/_app/immutable/entry/start.Ab1_cD.js" rel="modulepreload">
</head><body><script>${SCRIPT}</script></body></html>`;
const DEVICE = 'device_id="0190a000-0000-7000-8001-000000000001",device_name="pve-01",site="0190a000-0000-7000-8000-000000000001"';
const CHECK = `check_id="0190a000-0000-7000-8003-000000000001",check_name="ping \\"core\\"",check_type="ping",${DEVICE}`;

/** A K7 exposition: every family, two windows, one device; `faults` bend it. */
export function exposition(faults: ReadonlySet<Fault>): string {
  const family = (name: string, samples: string[]) => [`# HELP ${name} Synthetic.`, `# TYPE ${name} gauge`, ...samples];
  const check = faults.has("metrics-unknown-label") ? `${CHECK},severity="high"` : CHECK;
  const name = faults.has("metrics-volatile-label") ? CHECK.replace('ping \\"core\\"', "2026-11-11T09:30:00Z") : check;
  const lines = [
    ...family("anima_check_status", [`anima_check_status{${name}} 1`]),
    ...family("anima_check_up", [`anima_check_up{${check}} 1`]),
    ...family("anima_check_response_time_seconds", [`anima_check_response_time_seconds{${check}} 0.012`]),
    ...family("anima_check_uptime_ratio", ["1d", "30d"].map((w) => `anima_check_uptime_ratio{${check},window="${w}"} 0.99`)),
    ...family("anima_check_certificate_days_left", []),
    ...(faults.has("metrics-family-missing") ? [] : family("anima_check_domain_days_left", [])),
    ...family("anima_device_status", [`anima_device_status{${DEVICE}} 1`]),
    ...(faults.has("metrics-undeclared-family") ? [`anima_check_flaps{${check}} 3`] : []),
  ];
  return `${lines.join("\n")}\n`;
}

function staticHeaders(origin: string, faults: ReadonlySet<Fault>, asset: boolean): Headers {
  const hash = `'sha256-${createHash("sha256").update(SCRIPT).digest("base64")}'`;
  const scripts = faults.has("csp-unsafe-script") && !asset ? `${hash} 'unsafe-inline'` : hash;
  const policy = `default-src 'self'; script-src 'self' ${scripts}; connect-src 'self' ${origin.replace("http", "ws")}; `
    + "img-src 'self' data: blob:; style-src 'self' 'unsafe-inline'; font-src 'self'; object-src 'none'; "
    + "base-uri 'none'; form-action 'none'; frame-ancestors 'none'";
  const headers = new Headers({
    "content-security-policy": policy,
    "x-content-type-options": "nosniff",
    "referrer-policy": "no-referrer",
    "cross-origin-opener-policy": "same-origin",
    "strict-transport-security": "max-age=31536000",
  });
  const immutable = asset ? !faults.has("asset-no-cache") : faults.has("index-immutable");
  headers.set("cache-control", immutable ? "public, max-age=31536000, immutable" : "no-cache");
  if (faults.has(asset ? "asset-hsts-missing" : "nosniff-missing")) {
    headers.delete(asset ? "strict-transport-security" : "x-content-type-options");
  }
  return headers;
}

function preflight(request: Request, faults: ReadonlySet<Fault>): Response {
  const origin = request.headers.get("origin") ?? "";
  const clients = faults.has("cors-client-missing") ? CLIENT_ORIGINS.slice(0, 1) : CLIENT_ORIGINS;
  if (!clients.includes(origin) && !faults.has("cors-foreign-allowed")) {
    return new Response(null, { status: 200, headers: { vary: "origin" } });
  }
  const headers = { "access-control-allow-origin": origin, "access-control-allow-methods": "POST" };
  return new Response(null, { status: 200, headers });
}

const json = (value: unknown) => Response.json(value);
const refusal = (code: string, message = "Refused.") => json({ data: null, errors: [{ message, extensions: { code } }] });

/** The answer to an API key's document: `me`, FORBIDDEN by kind or by READ, else a validation error. */
function keyAnswer(query: string, faults: ReadonlySet<Fault>): Response {
  const [, operation, root] = /^(query|mutation|) ?\{ (\w+)/.exec(query) ?? [];
  if (root === "me") {
    return json({ data: { me: { __typename: "Operator" } } });
  }
  const allowed = faults.has("root-deleteDevice-allowed") && root === "deleteDevice";
  if (root !== undefined && REFUSED_ROOTS.includes(root) && !allowed) {
    return faults.has("roots-read-only") && operation === "mutation"
      ? refusal("FORBIDDEN", READ_ONLY_MESSAGE)
      : refusal("FORBIDDEN", "This token cannot do that. Sign in to the hub to do it.");
  }
  if (operation === "mutation" && !faults.has("key-writes") && !allowed) {
    return refusal("FORBIDDEN", READ_ONLY_MESSAGE);
  }
  return json({ data: null, errors: [{ message: 'Unknown argument "animaProbeOnly".' }] });
}

interface HubState {
  origin: string;
  faults: Set<Fault>;
  /** Every GraphQL document sent with the key. */
  keyDocuments: string[];
  /** Where `metrics-redirect` sends a scrape with the key: another origin. */
  redirectTo: string;
}

export interface FakeHub extends HubState {
  /** Closes the listener and every open connection; Bun's promise for it may never settle, so it is not awaited. */
  stop: () => void;
}

/** Whether a request carries a `__Host-anima` cookie from the foreign origin, as the probe must send it. */
function foreignCookie(request: Request): boolean {
  const cookie = request.headers.get("cookie") ?? "";
  return request.headers.get("origin") === FOREIGN_ORIGIN && /^__Host-anima=[A-Za-z0-9_-]{43}$/.test(cookie);
}

async function graphql(request: Request, hub: HubState, key: string): Promise<Response> {
  const { query } = (await request.json()) as { query: string };
  if (request.headers.get("authorization") === `Bearer ${key}`) {
    hub.keyDocuments.push(query);
    return keyAnswer(query, hub.faults);
  }
  if (hub.faults.has("cookie-signs-in") && foreignCookie(request)) {
    return json({ data: { me: { __typename: "Operator" } } });
  }
  const refused = refusal("UNAUTHENTICATED", "Sign in to continue.");
  if (hub.faults.has("cookie-set") && foreignCookie(request)) {
    refused.headers.set("set-cookie", "__Host-anima=; Max-Age=0; Path=/; Secure; HttpOnly; SameSite=Strict");
  }
  return refused;
}

function metrics(request: Request, hub: HubState, key: string): Response {
  if (hub.faults.has("metrics-redirect") && request.headers.has("authorization")) {
    return new Response(null, { status: 307, headers: { location: hub.redirectTo } });
  }
  if (request.headers.get("authorization") !== `Bearer ${key}` && !hub.faults.has("metrics-open")) {
    return new Response(null, { status: 401 });
  }
  const type = hub.faults.has("metrics-content-type") ? "text/plain" : "text/plain; version=0.0.4; charset=utf-8";
  return new Response(exposition(hub.faults), { headers: { "content-type": type } });
}

type Answer = Promise<Response> | Response | undefined;

function route(request: Request, server: Server<{ foreign: boolean }>, hub: HubState, key: string): Answer {
  const { pathname } = new URL(request.url);
  if (pathname === "/graphql" && request.method === "OPTIONS") {
    return preflight(request, hub.faults);
  }
  if (pathname === "/graphql" && request.method === "GET") {
    const protocol = request.headers.get("sec-websocket-protocol") === "graphql-transport-ws";
    const data = { foreign: foreignCookie(request) && protocol };
    if (hub.faults.has("socket-down")) {
      return new Response(null, { status: 403 });
    }
    const upgraded = server.upgrade(request, { data, headers: { "sec-websocket-protocol": "graphql-transport-ws" } });
    return upgraded ? undefined : new Response(null, { status: 405 });
  }
  if (pathname === "/graphql") {
    return graphql(request, hub, key);
  }
  if (pathname === "/metrics") {
    return metrics(request, hub, key);
  }
  if (pathname === "/health") {
    return new Response(hub.faults.has("health-not-ok") ? "starting" : "ok");
  }
  const asset = pathname.startsWith("/_app/immutable/");
  const headers = staticHeaders(hub.origin, hub.faults, asset);
  return new Response(asset ? "export const start = () => {};" : INDEX, { headers });
}

/** Starts the synthetic hub on a free loopback port; `key` is the one API key it knows (a READ key). */
export function startFakeHub(key: string): FakeHub {
  const state: HubState = { origin: "", faults: new Set(), keyDocuments: [], redirectTo: "" };
  const server = Bun.serve<{ foreign: boolean }>({
    hostname: "127.0.0.1",
    port: 0,
    fetch: (request, served) => route(request, served, state, key),
    websocket: {
      message(socket) {
        if (state.faults.has("socket-acknowledges") && socket.data.foreign) {
          socket.send(JSON.stringify({ type: "connection_ack" }));
        } else {
          socket.close(4403, "Forbidden");
        }
      },
    },
  });
  state.origin = `http://127.0.0.1:${String(server.port)}`;
  return Object.assign(state, {
    stop: () => {
      void server.stop(true);
    },
  });
}
