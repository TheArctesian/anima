// The LocalAPI proxy against a fake tailscaled on a unix socket (D-4, VPN-005, LOG-007).
import { afterAll, beforeAll, describe, expect, spyOn, test } from "bun:test";
import { existsSync, mkdtempSync, rmSync } from "node:fs";
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
let mode: "plain" | "headers" | "redirect" = "plain";
const SHORT_TIMEOUT_MS = 200;
const FAKE_PROCESS = join(import.meta.dir, "localapi-fake.ts");

beforeAll(() => {
  dir = mkdtempSync(join(tmpdir(), "anima-localapi-"));
  fake = Bun.serve({
    unix: join(dir, "tailscaled.sock"),
    fetch(request) {
      seen.push({ method: request.method, url: new URL(request.url).pathname + new URL(request.url).search, headers: request.headers });
      if (mode === "headers") return new Response("{}", { headers: { "content-type": "application/json", "set-cookie": "a=b", "x-foo": "bar" } });
      if (mode === "redirect") return new Response(null, { status: 302, headers: { location: "/localapi/v0/prefs" } });
      return big ? new Response("x".repeat(MAX_RESPONSE_BYTES + 1)) : Response.json({ ok: true });
    },
  });
  proxy = Bun.serve({ port: 0, fetch: localApiHandler({ port: 0, socketPath: join(dir, "tailscaled.sock"), upstreamTimeoutMs: SHORT_TIMEOUT_MS }) });
  missingProxy = Bun.serve({ port: 0, fetch: localApiHandler({ port: 0, socketPath: join(dir, "absent.sock"), upstreamTimeoutMs: SHORT_TIMEOUT_MS }) });
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

/** Runs the fake tailscaled in its own process, so an unthrottled body cannot block this one. */
async function withSpawnedFake(behaviour: "endless" | "silent", run: (socketPath: string) => Promise<void>): Promise<void> {
  const socketPath = join(dir, `${behaviour}.sock`);
  const child = Bun.spawn(["bun", FAKE_PROCESS, behaviour, socketPath], { stdout: "ignore", stderr: "ignore" });
  try {
    for (let tries = 0; tries < 100 && !existsSync(socketPath); tries += 1) await Bun.sleep(50);
    await run(socketPath);
  } finally {
    child.kill();
    await child.exited;
  }
}

const proxyFor = (socketPath: string) => Bun.serve({ port: 0, fetch: localApiHandler({ port: 0, socketPath, upstreamTimeoutMs: SHORT_TIMEOUT_MS }) });

describe("D-4 limits and header filtering", () => {
  test("D-4 an endless upstream body gives 502 promptly", async () => {
    await withSpawnedFake("endless", async (socketPath) => {
      const endless = proxyFor(socketPath);
      const started = Date.now();
      try {
        const response = await at(endless, "/localapi/v0/status", { signal: AbortSignal.timeout(10_000) });
        expect(response.status).toBe(502);
        expect(Date.now() - started).toBeLessThan(3000);
      } finally {
        await endless.stop(true);
      }
    });
  });

  test("D-4 an upstream that never answers gives 504", async () => {
    await withSpawnedFake("silent", async (socketPath) => {
      const silent = proxyFor(socketPath);
      try {
        expect((await at(silent, "/localapi/v0/status")).status).toBe(504);
      } finally {
        await silent.stop(true);
      }
    });
  });

  test("D-4 the reply carries no upstream header except content-type", async () => {
    mode = "headers";
    try {
      const response = await at(proxy, "/localapi/v0/status");
      expect(response.status).toBe(200);
      expect(response.headers.get("content-type")).toBe("application/json");
      expect(response.headers.has("set-cookie")).toBe(false);
      expect(response.headers.has("x-foo")).toBe(false);
    } finally {
      mode = "plain";
    }
  });

  test("D-4 an upstream redirect is returned as it is, not followed", async () => {
    mode = "redirect";
    seen.length = 0;
    try {
      const response = await at(proxy, "/localapi/v0/status", { redirect: "manual" });
      expect(response.status).toBe(302);
      expect(seen.map((s) => s.url)).toEqual(["/localapi/v0/status"]);
    } finally {
      mode = "plain";
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
