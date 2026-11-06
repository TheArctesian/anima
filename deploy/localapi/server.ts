// The I/O edge of the LocalAPI proxy (D-4): forwards the allowed calls to tailscaled's unix socket.
// Config is read once at startup; the fetch handler takes it as a parameter.
import { decide } from "./allow.ts";

export interface LocalApiConfig {
  port: number;
  socketPath: string;
  upstreamTimeoutMs: number;
}

const DEFAULT_UPSTREAM_TIMEOUT_MS = 10_000;
export const MAX_RESPONSE_BYTES = 1024 * 1024;
const UPSTREAM_HOST = "http://local-tailscaled.sock";

export function readConfig(env: Record<string, string | undefined>): LocalApiConfig {
  const port = Number(env["ANIMA_LOCALAPI_LISTEN_PORT"] ?? "8080");
  if (!Number.isInteger(port) || port < 1 || port > 65535) throw new Error("ANIMA_LOCALAPI_LISTEN_PORT is not a port");
  return { port, socketPath: env["ANIMA_LOCALAPI_SOCKET"] || "/run/tailscale/tailscaled.sock", upstreamTimeoutMs: DEFAULT_UPSTREAM_TIMEOUT_MS };
}

const hasBody = (request: Request): boolean =>
  request.headers.has("transfer-encoding") || (Number(request.headers.get("content-length") ?? "0") || 0) > 0;

function reply(status: number, body: string, line: string): Response {
  console.log(`${line} ${status}`);
  return new Response(body, { status, headers: { "content-type": "text/plain" } });
}

/** Reads at most MAX_RESPONSE_BYTES; null when the upstream sends more. */
async function readCapped(response: Response): Promise<Uint8Array | null> {
  const chunks: Uint8Array[] = [];
  let size = 0;
  for await (const chunk of response.body ?? []) {
    size += chunk.byteLength;
    if (size > MAX_RESPONSE_BYTES) return null;
    chunks.push(chunk);
  }
  return Buffer.concat(chunks);
}

async function forward(config: LocalApiConfig, method: string, upstream: string, line: string): Promise<Response> {
  try {
    const response = await fetch(UPSTREAM_HOST + upstream, {
      unix: config.socketPath,
      method,
      headers: {},
      redirect: "manual",
      signal: AbortSignal.timeout(config.upstreamTimeoutMs),
    });
    const body = await readCapped(response);
    if (body === null) return reply(502, "upstream response too large\n", line);
    console.log(`${line} ${response.status}`);
    const type = response.headers.get("content-type");
    return new Response(body.byteLength === 0 ? null : body, { status: response.status, headers: type === null ? {} : { "content-type": type } });
  } catch (error) {
    if (error instanceof DOMException && error.name === "TimeoutError") return reply(504, "upstream timed out\n", line);
    return reply(502, "upstream unreachable\n", line);
  }
}

export function localApiHandler(config: LocalApiConfig): (request: Request) => Promise<Response> {
  return async (request) => {
    const url = new URL(request.url);
    const decision = decide(request.method, url.pathname, url.searchParams);
    const line = `${request.method} ${url.pathname}${decision.kind === "forward" && decision.logIp ? ` ip=${decision.logIp}` : ""} ${decision.kind}`;
    if (decision.kind === "refuse") return reply(403, `${decision.reason}\n`, line);
    if (hasBody(request)) return reply(403, "request body not allowed\n", `${line}-body`);
    return forward(config, decision.method, decision.upstream, line);
  };
}

async function health(config: LocalApiConfig): Promise<number> {
  try {
    const response = await fetch(`http://127.0.0.1:${config.port}/health`);
    return response.status === 204 ? 0 : 1;
  } catch {
    return 1;
  }
}

if (import.meta.main) {
  const config = readConfig(process.env);
  if (process.argv[2] === "health") process.exit(await health(config));
  const handler = localApiHandler(config);
  Bun.serve({
    hostname: "0.0.0.0",
    port: config.port,
    fetch: (request) => (new URL(request.url).pathname === "/health" ? new Response(null, { status: 204 }) : handler(request)),
  });
  console.log(`localapi proxy listening on ${config.port}`);
}
