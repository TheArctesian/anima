import { afterEach, describe, expect, test } from "bun:test";
import { chmodSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const SCRIPT = join(import.meta.dir, "..", "bin", "make-secrets");
const GENERATED = [
  "postgres_password",
  "owner_password",
  "hub_password",
  "owner_database_url",
  "hub_database_url",
  "anima_key",
] as const;
// The host owner is set by chown (skipped in tests); the mode alone is checked here.
const MODES: Record<string, number> = {
  postgres_password: 0o440,
  owner_password: 0o440,
  hub_password: 0o440,
  owner_database_url: 0o440,
  hub_database_url: 0o440,
  anima_key: 0o440,
};

const folders: string[] = [];
afterEach(() => {
  for (const folder of folders.splice(0)) rmSync(folder, { recursive: true, force: true });
});

function freshFolder(): string {
  const folder = mkdtempSync(join(tmpdir(), "anima-secrets-"));
  folders.push(folder);
  return join(folder, "secrets");
}

function makeSecrets(folder: string): { exitCode: number; output: string } {
  const result = Bun.spawnSync(["sh", SCRIPT], {
    env: { ...process.env, ANIMA_SECRETS_DIR: folder, ANIMA_SECRETS_CHOWN: "0" },
    stdin: "ignore",
  });
  return { exitCode: result.exitCode ?? 1, output: result.stdout.toString() + result.stderr.toString() };
}

function read(folder: string, name: string): string {
  return readFileSync(join(folder, name), "utf8");
}

describe("make-secrets values (DB-002, SEC-004)", () => {
  test("DB-002: makes every generated secret file with its mode", () => {
    const folder = freshFolder();
    expect(makeSecrets(folder).exitCode).toBe(0);
    for (const name of GENERATED) {
      expect(statSync(join(folder, name)).mode & 0o777).toBe(MODES[name]);
    }
  });

  test("DNS-003: without a terminal it leaves the Cloudflare token to the operator and says so", () => {
    const folder = freshFolder();
    const { output } = makeSecrets(folder);
    expect(existsSync(join(folder, "cloudflare_dns_token"))).toBe(false);
    expect(output).toContain("cloudflare_dns_token");
  });

  test("DB-002: each database URL holds exactly the role's password file value", () => {
    const folder = freshFolder();
    makeSecrets(folder);
    expect(read(folder, "owner_database_url")).toBe(
      `postgres://anima_owner:${read(folder, "owner_password")}@postgres:5432/anima`,
    );
    expect(read(folder, "hub_database_url")).toBe(
      `postgres://anima_hub:${read(folder, "hub_password")}@postgres:5432/anima`,
    );
  });

  test("DB-002: passwords are 64 hex characters and differ from each other", () => {
    const folder = freshFolder();
    makeSecrets(folder);
    const passwords = ["postgres_password", "owner_password", "hub_password"].map((name) => read(folder, name));
    for (const password of passwords) expect(password).toMatch(/^[0-9a-f]{64}$/);
    expect(new Set(passwords).size).toBe(3);
  });

  test("SEC-004: anima_key decodes to 32 bytes", () => {
    const folder = freshFolder();
    makeSecrets(folder);
    expect(Buffer.from(read(folder, "anima_key"), "base64")).toHaveLength(32);
  });

});

describe("make-secrets reruns (DB-002, SEC-004)", () => {
  test("DB-002: a second run changes no file", () => {
    const folder = freshFolder();
    makeSecrets(folder);
    const before = GENERATED.map((name) => [read(folder, name), statSync(join(folder, name)).mtimeMs]);
    expect(makeSecrets(folder).exitCode).toBe(0);
    const after = GENERATED.map((name) => [read(folder, name), statSync(join(folder, name)).mtimeMs]);
    expect(after).toEqual(before);
  });

  test("SEC-004: it prints no generated value", () => {
    const folder = freshFolder();
    const first = makeSecrets(folder);
    const second = makeSecrets(folder);
    for (const name of GENERATED) {
      const value = read(folder, name);
      expect(first.output).not.toContain(value);
      expect(second.output).not.toContain(value);
    }
  });

  test("DB-002: a file that exists is kept untouched, and the URL built later uses it", () => {
    const folder = freshFolder();
    mkdirSync(folder);
    const mine = "my-own-password";
    writeFileSync(join(folder, "hub_password"), mine);
    chmodSync(join(folder, "hub_password"), 0o600);
    const before = statSync(join(folder, "hub_password"));
    makeSecrets(folder);
    expect(read(folder, "hub_password")).toBe(mine);
    expect(statSync(join(folder, "hub_password")).mtimeMs).toBe(before.mtimeMs);
    expect(statSync(join(folder, "hub_password")).mode & 0o777).toBe(0o600);
    expect(read(folder, "hub_database_url")).toContain(`anima_hub:${mine}@`);
  });

  test("SEC-004: a run leaves no temporary file behind", () => {
    const folder = freshFolder();
    makeSecrets(folder);
    const names = new Bun.Glob(".*").scanSync({ cwd: folder, dot: true });
    expect([...names]).toEqual([]);
  });
});
