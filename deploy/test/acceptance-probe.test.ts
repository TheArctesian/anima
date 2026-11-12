// deploy/acceptance/probe.ts (criteria 4, 12, 13; SEC-001, SEC-002, D-3, D-23, D-24, R-100, A-2). Spawned runs
// against a synthetic hub on 127.0.0.1 prove that each wrong answer fails exactly its own check, that no key,
// cookie or body reaches the output, and that TLS, proxies and redirects never take the key elsewhere.
import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { chmodSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import * as refusals from "../acceptance/refusals.ts";
import { startFakeHub, type FakeHub, type Fault } from "./acceptance-probe-hub.ts";
import { finish } from "./finished-process.ts";

const SCRIPT = join(import.meta.dir, "..", "acceptance", "probe.ts");

const KEY = `anima_${"qR7_vK2-mZ9xT4".repeat(4).slice(0, 43)}`;
const OTHER_KEY = `anima_${"Gh5sD2-Fj8_kQw".repeat(4).slice(0, 43)}`;
let hub: FakeHub;
let dir = "";
const outputs: string[] = [];

/** This process's environment without any proxy variable, plus `extra`. */
function environment(extra: Record<string, string> = {}): Record<string, string | undefined> {
  const proxies = /^(?:http|https|all)_proxy$/i;
  return { ...Object.fromEntries(Object.entries(process.env).filter(([name]) => !proxies.test(name))), ...extra };
}

async function probe(args: string[], extra: Record<string, string> = {}) {
  const child = Bun.spawn(["bun", SCRIPT, ...args], { env: environment(extra), stdin: "ignore", stdout: "pipe", stderr: "pipe" });
  const [out, err, code] = await Promise.all([new Response(child.stdout).text(), new Response(child.stderr).text(), child.exited]);
  outputs.push(out, err);
  return { code, out, err };
}

const failing = (out: string) => out.split("\n").filter((line) => line.startsWith("FAIL ")).map((line) => line.slice(5).split(":")[0]);
const keyFile = (name: string, content: string, mode = 0o600) => {
  const path = join(dir, name);
  writeFileSync(path, content);
  chmodSync(path, mode);
  return path;
};

beforeAll(() => {
  dir = mkdtempSync(join(tmpdir(), "probe-"));
  mkdirSync(join(dir, "folder"));
  hub = startFakeHub(KEY);
});

afterAll(() => {
  hub.stop();
  rmSync(dir, { recursive: true, force: true });
});

describe("spawned probe against a hub that answers right", () => {
  test("criteria 4, 12, 13: every check passes and exits 0; key documents are all invalid", async () => {
    hub.faults.clear();
    const result = await probe([hub.origin, "--key-file", keyFile("key", `  ${KEY}\n`)]);
    expect(result.err).toBe("");
    expect(failing(result.out)).toEqual([]);
    expect(result.out).toContain(`\n${String(16 + 2 + refusals.REFUSED_ROOTS.length)} checks, 0 failed\n`);
    expect(result.code).toBe(0);
    const sent = hub.keyDocuments.filter((document) => document !== refusals.IDENTITY_QUERY);
    expect(sent.length).toBe(1 + refusals.REFUSED_ROOTS.length);
    expect(sent.every((document) => document.includes("animaProbeOnly: true"))).toBe(true);
  }, 30_000);

  test("without a key only the signed-out checks run", async () => {
    const result = await probe([hub.origin]);
    expect(result).toEqual({ code: 0, err: "", out: expect.stringContaining("11 checks, 0 failed (no --key-file") as unknown as string });
  }, 30_000);
});

// Each fault and the checks it must fail: exactly these, no other.
const FAULTS: [Fault, string[]][] = [
  ["health-not-ok", ["health"]],
  ["csp-unsafe-script", ["index-csp"]],
  ["nosniff-missing", ["index-headers"]],
  ["index-immutable", ["index-cache"]],
  ["asset-no-cache", ["asset-cache"]],
  ["asset-hsts-missing", ["asset-headers"]],
  ["cookie-signs-in", ["graphql-foreign-cookie"]],
  ["socket-acknowledges", ["socket-foreign-cookie"]],
  ["cors-client-missing", ["cors-client-origins"]],
  ["cors-foreign-allowed", ["cors-foreign-origin"]],
  ["metrics-open", ["metrics-without-token"]],
  ["metrics-content-type", ["metrics-with-key"]],
  ["metrics-undeclared-family", ["metrics-text"]],
  ["metrics-family-missing", ["metrics-families"]],
  ["metrics-unknown-label", ["metrics-labels"]],
  ["metrics-volatile-label", ["metrics-label-values"]],
  ["key-writes", ["read-key-mutation"]],
  ["root-deleteDevice-allowed", ["refused-root-deleteDevice"]],
  ["socket-down", ["socket-foreign-cookie"]],
  ["cookie-set", ["graphql-foreign-cookie"]],
  ["roots-read-only", refusals.REFUSED_ROOTS.filter((root) => root !== "apiKeys").map((root) => `refused-root-${root}`)],
];

describe("spawned probe against a hub that answers wrong", () => {
  for (const [fault, checks] of FAULTS) {
    test(`${fault} fails exactly ${checks.length === 1 ? String(checks[0]) : "the refused mutation roots"}`, async () => {
      hub.faults.clear();
      hub.faults.add(fault);
      const result = await probe([hub.origin, "--key-file", keyFile("key", KEY)]);
      expect({ fault, failed: failing(result.out) }).toEqual({ fault, failed: checks });
      expect(result.code).toBe(1);
    }, 30_000);
  }

  test("a redirect is never followed, so the key never reaches another origin", async () => {
    let reached = 0;
    const elsewhere = Bun.serve({ hostname: "127.0.0.1", port: 0, fetch: () => ((reached += 1), new Response("")) });
    hub.faults.clear();
    hub.faults.add("metrics-redirect");
    hub.redirectTo = `http://127.0.0.1:${String(elsewhere.port)}/metrics`;
    const result = await probe([hub.origin, "--key-file", keyFile("key", KEY)]);
    void elsewhere.stop(true);
    expect(reached).toBe(0);
    expect(failing(result.out)).toEqual(["metrics-with-key", "metrics-text", "metrics-families", "metrics-labels", "metrics-label-values"]);
    expect(result.out).toContain("FAIL metrics-with-key: answered status 307, not 200\n");
  }, 30_000);

  test("a key the hub does not know fails key-accepted and every refusal, never prints it", async () => {
    hub.faults.clear();
    const result = await probe([hub.origin, "--key-file", keyFile("other", OTHER_KEY)]);
    const metrics = ["metrics-with-key", "metrics-text", "metrics-families", "metrics-labels", "metrics-label-values"];
    const roots = refusals.REFUSED_ROOTS.map((root) => `refused-root-${root}`);
    expect(failing(result.out)).toEqual([...metrics, "key-accepted", "read-key-mutation", ...roots]);
    expect(result.code).toBe(1);
  }, 30_000);
});

/** A loopback https server with a self-signed certificate; it counts requests and keys it receives. */
async function untrustedHub() {
  const pem = join(dir, "tls.pem");
  const made = await finish(["openssl", "req", "-x509", "-newkey", "ec", "-pkeyopt", "ec_paramgen_curve:P-256", "-nodes",
    "-days", "1", "-subj", "/CN=127.0.0.1", "-addext", "subjectAltName=IP:127.0.0.1", "-keyout", join(dir, "tls.key"), "-out", pem]);
  expect(made.exitCode).toBe(0);
  const seen = { requests: 0, keys: 0 };
  const server = Bun.serve({
    hostname: "127.0.0.1",
    port: 0,
    tls: { cert: Bun.file(pem), key: Bun.file(join(dir, "tls.key")) },
    fetch(request) {
      seen.requests += 1;
      seen.keys += request.headers.has("authorization") ? 1 : 0;
      return new Response("ok");
    },
  });
  return { origin: `https://127.0.0.1:${String(server.port)}`, pem, seen, stop: () => void server.stop(true) };
}

describe("S-M1 S-S2: TLS and proxies never take the key elsewhere", () => {
  test("NODE_TLS_REJECT_UNAUTHORIZED=0 does not let an untrusted certificate through", async () => {
    const untrusted = await untrustedHub();
    const refused = await probe([untrusted.origin, "--key-file", keyFile("key", KEY)], { NODE_TLS_REJECT_UNAUTHORIZED: "0" });
    expect(refused).toEqual({ code: 2, out: "", err: "probe: the hub cannot be reached (network, TLS or timeout)\n" });
    expect(untrusted.seen).toEqual({ requests: 0, keys: 0 });
    // Trusted through --ca, the same server is reached (and fails the checks, as it is no hub).
    const trusted = await probe([untrusted.origin, "--ca", untrusted.pem]);
    untrusted.stop();
    expect(trusted.code).toBe(1);
    expect(trusted.out).toStartWith("ok health\n");
  }, 30_000);

  test("a proxy variable in either case stops the run before any connection", async () => {
    let connections = 0;
    const proxy = Bun.listen({ hostname: "127.0.0.1", port: 0, socket: { open: () => void (connections += 1), data: (socket) => void socket.end() } });
    for (const name of ["HTTPS_PROXY", "http_proxy", "ALL_PROXY"]) {
      const result = await probe([hub.origin, "--key-file", keyFile("key", KEY)], { [name]: `http://127.0.0.1:${String(proxy.port)}` });
      const err = "probe: a proxy variable (HTTP_PROXY, HTTPS_PROXY or ALL_PROXY) is set; unset it to probe the hub directly\n";
      expect({ name, ...result }).toEqual({ name, code: 2, out: "", err });
    }
    proxy.stop(true);
    expect(connections).toBe(0);
  }, 30_000);
});

describe("spawned refusals exit 2 with one fixed line", () => {
  test("bad usage, an unusable key or CA file, or no hub", async () => {
    const closed = Bun.serve({ hostname: "127.0.0.1", port: 0, fetch: () => new Response("") });
    const gone = `http://127.0.0.1:${String(closed.port)}`;
    void closed.stop(true);
    const runs = [
      [], [hub.origin, hub.origin], ["http://hub.anima.ing"], [`https://u:${KEY}@hub.example`], [hub.origin, `--${KEY}`],
      [hub.origin, "--key-file", join(dir, "missing")], [hub.origin, "--key-file", join(dir, "folder")],
      [hub.origin, "--key-file", keyFile("shape", `${KEY}x\n`)], [hub.origin, "--key-file", keyFile("pem", "password=hunter2x")],
      [hub.origin, "--key-file", keyFile("big", `${KEY}${" ".repeat(5000)}`)],
      [hub.origin, "--ca", keyFile("ca", "not a certificate")], [gone],
    ];
    for (const args of runs) {
      const result = await probe(args);
      expect({ args, code: result.code, out: result.out }).toEqual({ args, code: 2, out: "" });
      expect(result.err).toMatch(/^probe: [^\n]+\n$/);
      expect(result.err).not.toContain(dir);
    }
    expect(outputs.at(-1)).toBe("probe: the hub cannot be reached (network, TLS or timeout)\n");
  }, 60_000);

  test("a key file others may read is used, with a warning", async () => {
    hub.faults.clear();
    const result = await probe([hub.origin, "--key-file", keyFile("open", KEY, 0o644)]);
    expect(result.err).toBe("probe: warning: the key file can be read by group or other users; chmod 600 it\n");
    expect(result.code).toBe(0);
  }, 30_000);

  test("LOG-007 A-2: no run printed a key, the fake cookie name's value, a body or a file's content", () => {
    expect(outputs.length).toBeGreaterThan(40);
    for (const output of outputs) {
      for (const secret of [KEY, OTHER_KEY, "hunter2x", "qR7_vK2", "__Host-anima=", "pve-01", "Sign in to continue", "Synthetic."]) {
        expect(output).not.toContain(secret);
      }
    }
  });
});
