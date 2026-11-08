// deploy/bin/egress-check against local listeners (NOT-002, D-17). A fake `docker` first on PATH
// checks the `compose exec -T hub` prefix, records the call, and runs the rest on this machine, so
// the real bash /dev/tcp command runs. macOS has no `timeout`, so the same temp dir holds a shim.
import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { chmodSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const SCRIPT = join(import.meta.dir, "..", "bin", "egress-check");
let dir = "";
let calls = "";

beforeAll(() => {
  dir = mkdtempSync(join(tmpdir(), "egress-"));
  calls = join(dir, "calls");
  writeFileSync(
    join(dir, "docker"),
    `#!/bin/sh
[ "$1 $2 $3 $4" = "compose exec -T hub" ] || { echo "unexpected docker call: $*" >&2; exit 99; }
shift 4
# Record "host:port", the last two arguments.
eval "pair=\\"\\$$(( $# - 1 )):\\$$#\\""
echo "$pair" >> "${calls}"
[ -z "\${FAKE_NO_CONNECT:-}" ] || exit 0
exec "$@"
`,
  );
  writeFileSync(join(dir, "timeout"), `#!/bin/sh\nshift\nexec "$@"\n`);
  chmodSync(join(dir, "docker"), 0o755);
  chmodSync(join(dir, "timeout"), 0o755);
});

afterAll(() => {
  rmSync(dir, { recursive: true, force: true });
});

function run(args: string[], env: Record<string, string> = {}) {
  writeFileSync(calls, "");
  const result = Bun.spawnSync(["sh", SCRIPT, ...args], {
    stdin: "ignore",
    env: { ...process.env, PATH: `${dir}:${process.env.PATH ?? ""}`, ...env },
  });
  return {
    code: result.exitCode,
    out: result.stdout.toString(),
    calls: readFileSync(calls, "utf8").split("\n").filter(Boolean),
  };
}

function listen() {
  return Bun.listen({ hostname: "127.0.0.1", port: 0, socket: { data() {} } });
}

function closedPort(): number {
  const server = listen();
  const { port } = server;
  void server.stop(true);
  return port;
}

describe("egress-check", () => {
  test("NOT-002 reports an open listener and exits 0", () => {
    const server = listen();
    const result = run([`127.0.0.1:${server.port}`]);
    void server.stop(true);
    expect(result.out).toContain(`open 127.0.0.1:${server.port}`);
    expect(result.code).toBe(0);
  });

  test("NOT-002 names a closed port as blocked and exits 1", () => {
    const port = closedPort();
    const result = run([`127.0.0.1:${port}`]);
    expect(result.out).toContain(`blocked 127.0.0.1:${port}`);
    expect(result.code).toBe(1);
  });

  test("NOT-002 mixed targets report both lines and exit 1", () => {
    const server = listen();
    const port = closedPort();
    const result = run([`127.0.0.1:${server.port}`, `127.0.0.1:${port}`]);
    void server.stop(true);
    expect(result.out).toContain(`open 127.0.0.1:${server.port}`);
    expect(result.out).toContain(`blocked 127.0.0.1:${port}`);
    expect(result.code).toBe(1);
  });

  test.each(["a;b:443", "host:0", "host:99999", "$(x):443", "host", "host:44a", "-h:443", "h:443;id", "a b:443"])(
    "NOT-002 refuses %p before any docker call",
    (target) => {
      const result = run(["127.0.0.1:1", target]);
      expect(result.code).toBe(2);
      expect(result.calls).toEqual([]);
    },
  );

  test("D-17 checks 443, 465 and 587 by default", () => {
    const result = run([], { FAKE_NO_CONNECT: "1" });
    expect(result.calls.map((call) => call.split(":")[1])).toEqual(["443", "465", "587"]);
    expect(result.code).toBe(0);
  });

  test("D-17 refuses a bad EGRESS_TIMEOUT before any docker call", () => {
    const result = run(["127.0.0.1:1"], { EGRESS_TIMEOUT: "5;id" });
    expect(result.code).toBe(2);
    expect(result.calls).toEqual([]);
  });
});
