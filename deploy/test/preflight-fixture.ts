// The secrets folder fixture of the bin/preflight tests: a folder of secret files with the modes
// make-secrets sets, and the updater key pair (DB-002, R-108, D-44).
import { chmodSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { finish } from "./finished-process.ts";

export const SCRIPT = join(import.meta.dir, "..", "bin", "preflight");

/** The test's own user and group stand in for root and the hub's group 10001. */
export const OWNER = String(process.getuid?.());
export const GROUP = String(process.getgid?.());

export async function preflight(...args: string[]): Promise<{ ok: boolean; message: string }> {
  const result = await finish(["sh", SCRIPT, ...args]);
  return { ok: result.exitCode === 0, message: result.stderr };
}

/** The secrets check of `folder`, with the test's user and group as the owner and hub group. */
export async function secrets(folder: string): Promise<{ ok: boolean; message: string }> {
  return preflight("secrets", folder, OWNER, GROUP);
}

export const names = ["postgres_password", "owner_password", "hub_password", "owner_database_url", "hub_database_url", "anima_key", "cloudflare_dns_token", "r2_access_key_id", "r2_secret_access_key"];
export const modeOf = (name: string) => (name === "cloudflare_dns_token" ? 0o400 : 0o440);

export function write(folder: string, name: string, content = "x", mode = modeOf(name)): void {
  rmSync(join(folder, name), { force: true });
  writeFileSync(join(folder, name), content);
  chmodSync(join(folder, name), mode);
}

export async function withFolder(body: (folder: string) => Promise<void>): Promise<void> {
  const folder = mkdtempSync(join(tmpdir(), "anima-preflight-"));
  try {
    await body(folder);
  } finally {
    rmSync(folder, { recursive: true, force: true });
  }
}

export const pair = ["updater/updater_key", "updater/updater_key_password"];
export function withPair(folder: string): void {
  mkdirSync(join(folder, "updater"), { recursive: true });
  chmodSync(join(folder, "updater"), 0o750);
  for (const name of [...names, ...pair]) write(folder, name);
}
