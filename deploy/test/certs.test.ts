// deploy/certs/install.sh, the lego hook (DNS-002, SEC-001), run in a temporary folder.
import { afterEach, describe, expect, test } from "bun:test";
import { existsSync, mkdtempSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const HOOK = join(import.meta.dir, "..", "certs", "install.sh");
const folders: string[] = [];
afterEach(() => {
  for (const folder of folders.splice(0)) rmSync(folder, { recursive: true, force: true });
});

function runHook(env: Record<string, string> = {}) {
  const folder = mkdtempSync(join(tmpdir(), "anima-certs-"));
  folders.push(folder);
  const chain = join(folder, "chain.crt");
  const key = join(folder, "chain.key");
  writeFileSync(chain, "CHAIN\n");
  writeFileSync(key, "KEY\n");
  const dest = join(folder, "hub");
  const result = Bun.spawnSync(["sh", HOOK], {
    env: { PATH: process.env["PATH"] ?? "", ANIMA_CERTS_DIR: dest, ANIMA_CERTS_OWNER: "", LEGO_CERT_PATH: chain, LEGO_CERT_KEY_PATH: key, ...env },
    stdin: "ignore",
  });
  return { result, dest };
}

describe("install.sh", () => {
  test("DNS-002 copies the chain and key with the modes the hub needs", () => {
    const { result, dest } = runHook();
    expect(result.exitCode).toBe(0);
    expect(readFileSync(join(dest, "fullchain.pem"), "utf8")).toBe("CHAIN\n");
    expect(readFileSync(join(dest, "key.pem"), "utf8")).toBe("KEY\n");
    expect(statSync(join(dest, "key.pem")).mode & 0o777).toBe(0o400);
    expect(statSync(join(dest, "fullchain.pem")).mode & 0o777).toBe(0o444);
  });

  test("DNS-002 leaves no temporary file behind", () => {
    const { dest } = runHook();
    expect(readdirSync(dest).sort()).toEqual(["fullchain.pem", "key.pem"]);
  });

  test("SEC-001 never prints the key", () => {
    const { result } = runHook();
    expect(result.stdout.toString() + result.stderr.toString()).not.toContain("KEY");
  });

  test("DNS-002 fails and writes nothing when lego gives no key path", () => {
    const { result, dest } = runHook({ LEGO_CERT_KEY_PATH: "" });
    expect(result.exitCode).not.toBe(0);
    expect(existsSync(join(dest, "key.pem"))).toBe(false);
  });
});
