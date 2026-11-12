// deploy/certs/install.sh, the lego hook (DNS-002, SEC-001), run in a temporary folder.
import { afterEach, describe, expect, test } from "bun:test";
import { chmodSync, existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, statSync, utimesSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { finish } from "./finished-process.ts";

const HOOK = join(import.meta.dir, "..", "certs", "install.sh");
const folders: string[] = [];
afterEach(() => {
  for (const folder of folders.splice(0)) rmSync(folder, { recursive: true, force: true });
});

interface Run {
  exitCode: number;
  output: string;
  dest: string;
  folder: string;
}

function setup(): { folder: string; chain: string; key: string; dest: string } {
  const folder = mkdtempSync(join(tmpdir(), "anima-certs-"));
  folders.push(folder);
  const chain = join(folder, "chain.crt");
  const key = join(folder, "chain.key");
  writeFileSync(chain, "CHAIN\n");
  writeFileSync(key, "KEY\n");
  return { folder, chain, key, dest: join(folder, "hub") };
}

async function run(env: Record<string, string> = {}, existing?: ReturnType<typeof setup>, umask = "077"): Promise<Run> {
  const { folder, chain, key, dest } = existing ?? setup();
  const result = await finish(["sh", "-c", `umask ${umask}; exec sh "$0"`, HOOK], {
    env: { PATH: process.env["PATH"] ?? "", ANIMA_CERTS_DIR: dest, ANIMA_CERTS_OWNER: "", ANIMA_CERTS_STAMP: join(folder, "stamp"), LEGO_CERT_PATH: chain, LEGO_CERT_KEY_PATH: key, ...env },
  });
  return { exitCode: result.exitCode, output: result.stdout + result.stderr, dest, folder };
}

describe("install.sh", () => {
  test("DNS-002 writes key and chain as one tls.pem, mode 0400", async () => {
    const { exitCode, dest } = await run();
    expect(exitCode).toBe(0);
    expect(readFileSync(join(dest, "tls.pem"), "utf8")).toBe("KEY\nCHAIN\n");
    expect(statSync(join(dest, "tls.pem")).mode & 0o777).toBe(0o400);
  });

  test("DNS-002 leaves no temporary file behind", async () => {
    const { dest } = await run();
    expect(readdirSync(dest)).toEqual(["tls.pem"]);
  });

  test("SEC-001 never prints the key", async () => {
    expect((await run()).output).not.toContain("KEY");
  });

  test("DNS-002 fails and writes nothing when lego gives no key path", async () => {
    const { exitCode, dest } = await run({ LEGO_CERT_KEY_PATH: "" });
    expect(exitCode).not.toBe(0);
    expect(existsSync(join(dest, "tls.pem"))).toBe(false);
  });

  test("DNS-002 replaces a different installed file and leaves an identical one untouched", async () => {
    const first = setup();
    await run({}, first);
    const before = statSync(join(first.dest, "tls.pem")).ino;
    expect((await run({}, first)).output).toContain("up to date");
    expect(statSync(join(first.dest, "tls.pem")).ino).toBe(before);
    writeFileSync(first.chain, "NEWCHAIN\n");
    await run({}, first);
    expect(readFileSync(join(first.dest, "tls.pem"), "utf8")).toBe("KEY\nNEWCHAIN\n");
    expect(readdirSync(first.dest)).toEqual(["tls.pem"]);
  });

  test("SEC-001 chmod runs before chown and never after it (root without capabilities cannot chmod a file it gave away)", async () => {
    const env = setup();
    const shims = join(env.folder, "shims");
    const log = join(env.folder, "calls.log");
    mkdirSync(shims);
    for (const tool of ["chmod", "chown"]) {
      const real = Bun.which(tool, { PATH: "/bin:/usr/bin:/usr/sbin:/sbin" });
      if (real === null) throw new Error(`${tool} not found`);
      const shim = join(shims, tool);
      // The shim records the call and does nothing else; the order is what is tested.
      writeFileSync(shim, `#!/bin/sh\necho "${tool} $*" >>"${log}"\n[ "${tool}" = chmod ] && exec "${real}" "$@"\nexit 0\n`);
      chmodSync(shim, 0o755);
    }
    const result = await run({ PATH: `${shims}:${process.env["PATH"] ?? ""}`, ANIMA_CERTS_OWNER: "10001:10001" }, env);
    expect(result.exitCode).toBe(0);
    const calls = readFileSync(log, "utf8").trim().split("\n");
    const lastChmod = calls.map((call) => call.startsWith("chmod")).lastIndexOf(true);
    const firstChown = calls.findIndex((call) => call.startsWith("chown"));
    expect(firstChown).toBeGreaterThan(-1);
    expect(lastChmod).toBeLessThan(firstChown);
    expect(calls[firstChown]).toContain("10001:10001");
  });

});

describe("install.sh stamp, folder and temporary file", () => {
  test("DNS-002 reinstalls a file that was emptied, even though the content to install is unchanged", async () => {
    const env = setup();
    await run({}, env);
    const target = join(env.dest, "tls.pem");
    chmodSync(target, 0o600);
    writeFileSync(target, "");
    expect((await run({}, env)).output).toContain("installed");
    expect(readFileSync(target, "utf8")).toBe("KEY\nCHAIN\n");
  });

  test("DNS-002 reinstalls an older file whose checksum still matches", async () => {
    const env = setup();
    await run({}, env);
    const target = join(env.dest, "tls.pem");
    const old = new Date(Date.now() - 86_400_000);
    utimesSync(target, old, old);
    expect((await run({}, env)).output).toContain("installed");
    expect(statSync(target).mtimeMs).toBeGreaterThan(old.getTime() + 60_000);
  });

  test("DNS-002 repairs a deleted tls.pem", async () => {
    const env = setup();
    await run({}, env);
    rmSync(join(env.dest, "tls.pem"));
    expect((await run({}, env)).exitCode).toBe(0);
    expect(readFileSync(join(env.dest, "tls.pem"), "utf8")).toBe("KEY\nCHAIN\n");
  });

  test("DNS-002 writes the stamp only after the rename succeeded", async () => {
    const env = setup();
    const shims = join(env.folder, "shims");
    mkdirSync(shims);
    writeFileSync(join(shims, "mv"), "#!/bin/sh\nexit 1\n");
    chmodSync(join(shims, "mv"), 0o755);
    const result = await run({ PATH: `${shims}:${process.env["PATH"] ?? ""}` }, env);
    expect(result.exitCode).not.toBe(0);
    expect(existsSync(join(env.folder, "stamp"))).toBe(false);
    expect(existsSync(join(env.dest, "tls.pem"))).toBe(false);
    expect(readdirSync(env.dest)).toEqual([]);
  });

  test("DNS-002 makes the temporary file in the destination folder, so the rename is atomic", async () => {
    const env = setup();
    const shims = join(env.folder, "shims");
    const log = join(env.folder, "mktemp.log");
    mkdirSync(shims);
    const real = Bun.which("mktemp", { PATH: "/bin:/usr/bin" });
    writeFileSync(join(shims, "mktemp"), `#!/bin/sh\necho "$1" >>"${log}"\nexec "${real}" "$@"\n`);
    chmodSync(join(shims, "mktemp"), 0o755);
    await run({ PATH: `${shims}:${process.env["PATH"] ?? ""}` }, env);
    expect(readFileSync(log, "utf8").trim()).toStartWith(`${env.dest}/`);
  });

  test("SEC-001 sets its own umask: the stamp is private and the folder is 0755 even under umask 000", async () => {
    const env = setup();
    await run({}, env, "000");
    expect(statSync(join(env.folder, "stamp")).mode & 0o777).toBe(0o600);
    expect(statSync(env.dest).mode & 0o777).toBe(0o755);
  });
});
