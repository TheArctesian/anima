// deploy/certs/run.sh with a fake lego and a fake sleep (DNS-002, DNS-003). No network.
import { afterEach, describe, expect, test } from "bun:test";
import { chmodSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const DEPLOY = join(import.meta.dir, "..");
const folders: string[] = [];
afterEach(() => {
  for (const folder of folders.splice(0)) rmSync(folder, { recursive: true, force: true });
});

// Fake lego: logs its arguments. `run` writes a certificate into the --path folder unless
// FAKE_LEGO_RUN_EXIT is set. `renew` writes FAKE_LEGO_RENEW_CONTENT as the new chain, does
// not call the hook (as if the hook had failed) and exits with FAKE_LEGO_RENEW_EXIT.
const FAKE_LEGO = `#!/bin/sh
echo "$*" >>"$FAKE_LOG/lego.log"
while [ $# -gt 0 ]; do [ "$1" = --path ] && path=$2; shift; done
set -- $(tail -1 "$FAKE_LOG/lego.log")
mkdir -p "$path/certificates"
case " $* " in
*" run "*)
    [ -n "\${FAKE_LEGO_RUN_EXIT:-}" ] && exit "$FAKE_LEGO_RUN_EXIT"
    echo FIRSTCHAIN >"$path/certificates/hub.anima.ing.crt"
    echo FIRSTKEY >"$path/certificates/hub.anima.ing.key"
    ;;
*" renew "*)
    [ -n "\${FAKE_LEGO_RENEW_CONTENT:-}" ] && echo "$FAKE_LEGO_RENEW_CONTENT" >"$path/certificates/hub.anima.ing.crt"
    exit "\${FAKE_LEGO_RENEW_EXIT:-0}"
    ;;
esac
`;
// Fake sleep: logs its argument; the (LIMIT+1)th call fails, which ends run.sh's loop.
const FAKE_SLEEP = `#!/bin/sh
echo "$1" >>"$FAKE_LOG/sleep.log"
count=$(wc -l <"$FAKE_LOG/sleep.log")
[ "$count" -le "\${FAKE_SLEEP_LIMIT:-1}" ]
`;

interface Outcome {
  exitCode: number;
  lego: string[];
  sleeps: string[];
  tls: string | null;
  folder: string;
}

function runScript(options: { chain?: string; env?: Record<string, string> } = {}): Outcome {
  const folder = mkdtempSync(join(tmpdir(), "anima-run-"));
  folders.push(folder);
  const bin = join(folder, "bin");
  const state = join(folder, "state");
  const log = join(folder, "log");
  mkdirSync(bin);
  mkdirSync(log);
  mkdirSync(join(state, "certificates"), { recursive: true });
  for (const [name, body] of [["lego", FAKE_LEGO], ["sleep", FAKE_SLEEP]] as const) {
    writeFileSync(join(bin, name), body);
    chmodSync(join(bin, name), 0o755);
  }
  if (options.chain !== undefined) {
    writeFileSync(join(state, "certificates", "hub.anima.ing.crt"), options.chain);
    writeFileSync(join(state, "certificates", "hub.anima.ing.key"), "OLDKEY\n");
  }
  const result = Bun.spawnSync(["sh", join(DEPLOY, "certs", "run.sh")], {
    env: {
      PATH: `${bin}:${process.env["PATH"] ?? ""}`,
      FAKE_LOG: log,
      ANIMA_ACME_EMAIL: "ops@example.invalid",
      ANIMA_LEGO: join(bin, "lego"),
      ANIMA_LEGO_STATE: state,
      ANIMA_INSTALL_HOOK: join(DEPLOY, "certs", "install.sh"),
      ANIMA_CERTS_DIR: join(folder, "hub"),
      ANIMA_CERTS_OWNER: "",
      ANIMA_CERTS_STAMP: join(folder, "stamp"),
      ...options.env,
    },
    stdin: "ignore",
  });
  const lines = (name: string) => (existsSync(join(log, name)) ? readFileSync(join(log, name), "utf8").trim().split("\n") : []);
  const tlsPath = join(folder, "hub", "tls.pem");
  return {
    exitCode: result.exitCode ?? 1,
    lego: lines("lego.log"),
    sleeps: lines("sleep.log"),
    tls: existsSync(tlsPath) ? readFileSync(tlsPath, "utf8") : null,
    folder,
  };
}

describe("run.sh", () => {
  test("DNS-002 orders the first certificate when lego state holds none, with the e-mail and accept-tos", () => {
    const outcome = runScript();
    const first = outcome.lego[0] ?? "";
    expect(first).toContain("--email ops@example.invalid");
    expect(first).toContain("--accept-tos");
    expect(first).toContain("--dns cloudflare");
    expect(first).toContain("--domains hub.anima.ing");
    expect(first).toMatch(/ run --run-hook .*install\.sh$/);
    expect(outcome.tls).toBe("FIRSTKEY\nFIRSTCHAIN\n");
  });

  test("DNS-002 an empty certificate file counts as none and is ordered again", () => {
    const outcome = runScript({ chain: "" });
    expect(outcome.lego[0]).toContain(" run ");
  });

  test("DNS-002 an existing certificate is installed without a new order", () => {
    const outcome = runScript({ chain: "OLDCHAIN\n" });
    expect(outcome.lego.some((line) => line.includes(" run "))).toBe(false);
    expect(outcome.tls).toBe("OLDKEY\nOLDCHAIN\n");
  });

  test("DNS-002 a start with an existing certificate checks renewal, with the hook, before the first sleep", () => {
    const outcome = runScript({ chain: "OLDCHAIN\n", env: { FAKE_LEGO_RENEW_CONTENT: "NEWCHAIN", FAKE_SLEEP_LIMIT: "0" } });
    expect(outcome.lego.filter((line) => line.includes(" renew "))).toHaveLength(1);
    expect(outcome.lego[0]).toMatch(/ renew .*--renew-hook .*install\.sh$/);
    expect(outcome.sleeps).toEqual(["43200"]);
    expect(outcome.tls).toBe("OLDKEY\nNEWCHAIN\n");
  });

  test("DNS-002 waits 3600 seconds after a failed first order, then exits non-zero", () => {
    const outcome = runScript({ env: { FAKE_LEGO_RUN_EXIT: "1" } });
    expect(outcome.exitCode).not.toBe(0);
    expect(outcome.sleeps).toEqual(["3600"]);
  });

  test("DNS-002 checks renewal every 43200 seconds with --days 30 and the hook", () => {
    const outcome = runScript({ chain: "OLDCHAIN\n" });
    expect(outcome.sleeps[0]).toBe("43200");
    const renew = outcome.lego.find((line) => line.includes(" renew ")) ?? "";
    expect(renew).toContain("--days 30");
    expect(renew).toContain("--email ops@example.invalid");
    expect(renew).toMatch(/--renew-hook .*install\.sh$/);
  });

  test("DNS-002 installs a renewed certificate even when lego's hook failed, and when lego itself failed", () => {
    for (const exit of ["0", "1"]) {
      const outcome = runScript({ chain: "OLDCHAIN\n", env: { FAKE_LEGO_RENEW_CONTENT: "NEWCHAIN", FAKE_LEGO_RENEW_EXIT: exit } });
      expect(outcome.tls).toBe("OLDKEY\nNEWCHAIN\n");
    }
  });

  test("DNS-002 settings from the environment replace the defaults", () => {
    const outcome = runScript({ chain: "OLDCHAIN\n", env: { ANIMA_RENEW_EVERY_SECONDS: "7" } });
    expect(outcome.sleeps[0]).toBe("7");
  });
});
