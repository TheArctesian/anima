// deploy/bin/egress-check against local listeners (NOT-002, D-17). A fake `docker` first on PATH
// checks the `compose exec -T hub` prefix, records the call, and runs the rest on this machine, so
// the real bash /dev/tcp command runs. macOS has no `timeout`, so the same temp dir holds a shim.
import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { chmodSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { finish } from "./finished-process.ts";

const SCRIPT = join(import.meta.dir, "..", "bin", "egress-check");
let dir = "";
let calls = "";
let timeouts = "";

beforeAll(() => {
  dir = mkdtempSync(join(tmpdir(), "egress-"));
  calls = join(dir, "calls");
  timeouts = join(dir, "timeouts");
  // Records every argument after the prefix, one per line, ended by a "--" line.
  writeFileSync(
    join(dir, "docker"),
    `#!/bin/sh
[ "$1 $2 $3 $4" = "compose exec -T hub" ] || { echo "unexpected docker call: $*" >&2; exit 99; }
shift 4
{ for a in "$@"; do printf '%s\\n' "$a"; done; echo --; } >> "${calls}"
if [ "$1" = true ] && [ -n "\${FAKE_DOWN:-}" ]; then
  echo 'service "hub" is not running' >&2
  exit 1
fi
[ -z "\${FAKE_NO_CONNECT:-}" ] || exit 0
exec "$@"
`,
  );
  // Records the seconds, can pretend to time out (exit 124), else enforces them like coreutils.
  writeFileSync(
    join(dir, "timeout"),
    `#!/bin/sh
echo "$1" >> "${timeouts}"
[ -z "\${FAKE_TIMEOUT_EXIT:-}" ] || exit "$FAKE_TIMEOUT_EXIT"
exec perl -e 'alarm shift; exec @ARGV' "$@"
`,
  );
  chmodSync(join(dir, "docker"), 0o755);
  chmodSync(join(dir, "timeout"), 0o755);
});

afterAll(() => {
  rmSync(dir, { recursive: true, force: true });
});

async function run(args: string[], env: Record<string, string> = {}) {
  writeFileSync(calls, "");
  writeFileSync(timeouts, "");
  const result = await finish(["sh", SCRIPT, ...args], {
    env: { ...process.env, PATH: `${dir}:${process.env.PATH ?? ""}`, ...env },
  });
  const all = readFileSync(calls, "utf8")
    .split("--\n")
    .filter(Boolean)
    .map((call) => call.split("\n").filter(Boolean));
  return {
    code: result.exitCode,
    out: result.stdout,
    err: result.stderr,
    // Calls that test a target; the first call of a run is the `true` probe.
    calls: all.filter((call) => call[0] !== "true"),
    probed: all.some((call) => call[0] === "true"),
    timeouts: readFileSync(timeouts, "utf8").split("\n").filter(Boolean),
  };
}

function listen() {
  return Bun.listen({ hostname: "127.0.0.1", port: 0, socket: { data: () => undefined } });
}

function closedPort(): number {
  const server = listen();
  const { port } = server;
  void server.stop(true);
  return port;
}

describe("egress-check results", () => {
  test("NOT-002 reports an open listener and exits 0", async () => {
    const server = listen();
    const result = await run([`127.0.0.1:${server.port}`]);
    void server.stop(true);
    expect(result.out).toContain(`open 127.0.0.1:${server.port}`);
    expect(result.code).toBe(0);
  });

  test("NOT-002 names a closed port as blocked and exits 1", async () => {
    const port = closedPort();
    const result = await run([`127.0.0.1:${port}`]);
    expect(result.out).toContain(`blocked 127.0.0.1:${port}`);
    expect(result.code).toBe(1);
  });

  test("NOT-002 mixed targets report both lines and exit 1", async () => {
    const server = listen();
    const port = closedPort();
    const result = await run([`127.0.0.1:${server.port}`, `127.0.0.1:${port}`]);
    void server.stop(true);
    expect(result.out).toContain(`open 127.0.0.1:${server.port}`);
    expect(result.out).toContain(`blocked 127.0.0.1:${port}`);
    expect(result.code).toBe(1);
  });

  test.each(["a;b:443", "host:0", "host:99999", "$(x):443", "host", "host:44a", "-h:443", "h:443;id", "a b:443"])(
    "NOT-002 refuses %p before any docker call",
    async (target) => {
      const result = await run(["127.0.0.1:1", target]);
      expect(result.code).toBe(2);
      expect(result.calls).toEqual([]);
      expect(result.probed).toBe(false);
    },
  );

  test("D-17 checks 443, 465 and 587 by default", async () => {
    const result = await run([], { FAKE_NO_CONNECT: "1" });
    expect(result.calls.map((call) => call.at(-1))).toEqual(["443", "465", "587"]);
    expect(result.code).toBe(0);
  });

});

describe("egress-check arguments and reasons", () => {
  test("NOT-002 passes host and port as arguments, never inside the bash -c string", async () => {
    const result = await run(["example.com:587"], { FAKE_NO_CONNECT: "1" });
    expect(result.calls).toEqual([
      ["timeout", "5", "bash", "-c", 'exec 3<>"/dev/tcp/$1/$2"', "egress", "example.com", "587"],
    ]);
  });

  test("NOT-002 reports a refused port with its reason", async () => {
    const port = closedPort();
    expect((await run([`127.0.0.1:${port}`])).out).toContain(`blocked 127.0.0.1:${port} (refused)`);
  });

  test("NOT-002 reports an unresolvable name with its reason and exits 1", async () => {
    const result = await run(["no-such-host.invalid:443"]);
    expect(result.out).toContain("blocked no-such-host.invalid:443 (unresolved)");
    expect(result.code).toBe(1);
  });

  test("NOT-002 reports a timeout (exit 124) as timed out", async () => {
    const result = await run(["127.0.0.1:1"], { FAKE_TIMEOUT_EXIT: "124" });
    expect(result.out).toContain("blocked 127.0.0.1:1 (timed out)");
    expect(result.code).toBe(1);
  });

  test("NOT-002 does not call a missing hub container blocked", async () => {
    const result = await run(["127.0.0.1:1"], { FAKE_DOWN: "1" });
    expect(result.out).not.toContain("blocked");
    expect(result.err).toContain("not running");
    expect(result.err).toContain("cannot run in the hub container");
    expect(result.code).toBe(2);
    expect(result.calls).toEqual([]);
  });

  test("D-17 passes the default timeout of 5 seconds to timeout", async () => {
    const server = listen();
    const result = await run([`127.0.0.1:${server.port}`]);
    void server.stop(true);
    expect(result.timeouts).toEqual(["5"]);
  });

  test("D-17 passes EGRESS_TIMEOUT through to timeout", async () => {
    const server = listen();
    const result = await run([`127.0.0.1:${server.port}`], { EGRESS_TIMEOUT: "3" });
    void server.stop(true);
    expect(result.timeouts).toEqual(["3"]);
  });

  test("D-17 refuses a bad EGRESS_TIMEOUT before any docker call", async () => {
    const result = await run(["127.0.0.1:1"], { EGRESS_TIMEOUT: "5;id" });
    expect(result.code).toBe(2);
    expect(result.calls).toEqual([]);
  });
});
