// The LocalAPI proxy against a fake tailscaled on a unix socket (D-4, VPN-005, LOG-007).
import { afterAll, beforeAll, describe, expect, spyOn, test } from "bun:test";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { localApiHandler, MAX_RESPONSE_BYTES } from "../localapi/server.ts";

interface Seen {
  method: string;
  url: string;
  headers: Headers;
}

let dir = "";
const seen: Seen[] = [];
let fake: ReturnType<typeof Bun.serve>;
let proxy: ReturnType<typeof Bun.serve>;
let missingProxy: ReturnType<typeof Bun.serve>;
let big = false;

beforeAll(() => {
  dir = mkdtempSync(join(tmpdir(), "anima-localapi-"));
  fake = Bun.serve({
    unix: join(dir, "tailscaled.sock"),
    fetch(request) {
      seen.push({ method: request.method, url: new URL(request.url).pathname + new URL(request.url).search, headers: request.headers });
      return big ? new Response("x".repeat(MAX_RESPONSE_BYTES + 1)) : Response.json({ ok: true });
    },
  });
  proxy = Bun.serve({ port: 0, fetch: localApiHandler({ port: 0, socketPath: join(dir, "tailscaled.sock") }) });
  missingProxy = Bun.serve({ port: 0, fetch: localApiHandler({ port: 0, socketPath: join(dir, "absent.sock") }) });
});

afterAll(async () => {
  await proxy.stop(true);
  await missingProxy.stop(true);
  await fake.stop(true);
  rmSync(dir, { recursive: true, force: true });
});

const at = (server: ReturnType<typeof Bun.serve>, path: string, init?: RequestInit) => fetch(`http://127.0.0.1:${server.port}${path}`, init);

describe("D-4 forwarding", () => {
  test("VPN-005 an allowed status call reaches the socket with Host local-tailscaled.sock and no client credentials", async () => {
    seen.length = 0;
    const response = await at(proxy, "/localapi/v0/status?peers=true", {
      headers: { Authorization: "Basic eDp5", Cookie: "a=b", Origin: "https://evil.example", "X-Forwarded-For": "1.2.3.4" },
    });
    expect(response.status).toBe(200);
    expect(await response.json()).toEqual({ ok: true });
    expect(seen).toHaveLength(1);
    expect(seen[0]?.method).toBe("GET");
    expect(seen[0]?.url).toBe("/localapi/v0/status?peers=true");
    expect(seen[0]?.headers.get("host")).toBe("local-tailscaled.sock");
    for (const name of ["authorization", "cookie", "origin", "x-forwarded-for"]) expect(seen[0]?.headers.has(name)).toBe(false);
  });

  test("D-4 an allowed ping is sent as POST with the rebuilt query", async () => {
    seen.length = 0;
    const response = await at(proxy, "/localapi/v0/ping?type=disco&ip=100.64.0.5", { method: "POST" });
    expect(response.status).toBe(200);
    expect(seen.map((s) => `${s.method} ${s.url}`)).toEqual(["POST /localapi/v0/ping?ip=100.64.0.5&type=disco"]);
  });

  test.each([
    ["GET", "/localapi/v0/prefs"],
    ["POST", "/localapi/v0/logout"],
    ["POST", "/localapi/v0/ping?ip=100.128.0.1&type=disco"],
    ["GET", "/localapi/v0/status/"],
    ["PUT", "/localapi/v0/status"],
  ])("D-4 %s %s is refused with 403 and the socket sees nothing", async (method, path) => {
    seen.length = 0;
    const response = await at(proxy, path, { method });
    expect(response.status).toBe(403);
    expect(response.headers.get("content-type")).toContain("text/plain");
    expect(seen).toHaveLength(0);
  });

  test("D-4 a request body is refused even on an allowed call", async () => {
    seen.length = 0;
    const response = await at(proxy, "/localapi/v0/ping?ip=100.64.0.5&type=disco", { method: "POST", body: "{}" });
    expect(response.status).toBe(403);
    const chunked = await at(proxy, "/localapi/v0/ping?ip=100.64.0.5&type=disco", {
      method: "POST",
      headers: { Connection: "close" },
      body: new ReadableStream({ start: (c) => (c.enqueue(new TextEncoder().encode("x")), c.close()) }),
      duplex: "half",
    } as RequestInit);
    expect(chunked.status).toBe(403);
    expect(seen).toHaveLength(0);
  });
});

describe("D-4 upstream failures", () => {
  test("D-4 a missing socket gives 502", async () => {
    expect((await at(missingProxy, "/localapi/v0/status")).status).toBe(502);
  });

  test("D-4 an upstream body over the cap gives 502", async () => {
    big = true;
    try {
      expect((await at(proxy, "/localapi/v0/status")).status).toBe(502);
    } finally {
      big = false;
    }
  });
});

describe("LOG-007 the log line", () => {
  test("LOG-007 logs method, path and decision, the ping ip only, and no header or other query value", async () => {
    const log = spyOn(console, "log").mockImplementation(() => undefined);
    try {
      await at(proxy, "/localapi/v0/ping?ip=100.64.0.5&type=disco", { method: "POST", headers: { Authorization: "Basic c2VjcmV0" } });
      await at(proxy, "/localapi/v0/status?peers=true&token=hunter2", { headers: { Cookie: "session=hunter2" } });
      const lines = log.mock.calls.map((args) => args.join(" "));
      expect(lines).toHaveLength(2);
      expect(lines[0]).toContain("POST /localapi/v0/ping ip=100.64.0.5 forward 200");
      expect(lines[1]).toContain("GET /localapi/v0/status refuse 403");
      for (const line of lines) expect(line).not.toMatch(/hunter2|c2VjcmV0|Basic|session|token/);
    } finally {
      log.mockRestore();
    }
  });
});
