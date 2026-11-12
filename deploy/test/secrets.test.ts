import { afterEach, describe, expect, test } from "bun:test";
import { chmodSync, existsSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";
import { GENERATED, makeSecrets, newSecretsPath, read, removeScratchFolders, secretsFolderWith } from "./make-secrets.ts";

afterEach(removeScratchFolders);

function mode(folder: string, name = ""): number {
  return statSync(join(folder, name)).mode & 0o777;
}

describe("make-secrets values (DB-002, SEC-004)", () => {
  test("DB-002: makes every generated secret file with mode 0440 in a 0700 folder", async () => {
    const folder = newSecretsPath();
    expect((await makeSecrets(folder)).exitCode).toBe(0);
    expect(mode(folder)).toBe(0o700);
    for (const name of GENERATED) expect(mode(folder, name)).toBe(0o440);
  });

  test("DNS-003: without a terminal it leaves the Cloudflare token to the operator and says so on stderr", async () => {
    const folder = newSecretsPath();
    const { stderr, stdout } = await makeSecrets(folder);
    expect(existsSync(join(folder, "cloudflare_dns_token"))).toBe(false);
    expect(stderr).toContain("cloudflare_dns_token");
    expect(stdout).not.toContain("cloudflare_dns_token");
  });

  test("DB-002: each database URL holds exactly the role's password file value", async () => {
    const folder = newSecretsPath();
    await makeSecrets(folder);
    expect(read(folder, "owner_database_url")).toBe(
      `postgres://anima_owner:${read(folder, "owner_password")}@postgres:5432/anima`,
    );
    expect(read(folder, "hub_database_url")).toBe(
      `postgres://anima_hub:${read(folder, "hub_password")}@postgres:5432/anima`,
    );
  });

  test("DB-002: passwords are 64 hex characters and differ from each other", async () => {
    const folder = newSecretsPath();
    await makeSecrets(folder);
    const passwords = ["postgres_password", "owner_password", "hub_password"].map((name) => read(folder, name));
    for (const password of passwords) expect(password).toMatch(/^[0-9a-f]{64}$/);
    expect(new Set(passwords).size).toBe(3);
  });

  test("SEC-004: anima_key decodes to 32 bytes", async () => {
    const folder = newSecretsPath();
    await makeSecrets(folder);
    expect(Buffer.from(read(folder, "anima_key"), "base64")).toHaveLength(32);
  });
});

describe("make-secrets reruns (DB-002, SEC-004)", () => {
  test("DB-002: a second run changes no file", async () => {
    const folder = newSecretsPath();
    await makeSecrets(folder);
    const snapshot = () => GENERATED.map((name) => [read(folder, name), statSync(join(folder, name)).mtimeMs]);
    const before = snapshot();
    expect((await makeSecrets(folder)).exitCode).toBe(0);
    expect(snapshot()).toEqual(before);
  });

  test("SEC-004: it prints no generated value", async () => {
    const folder = newSecretsPath();
    const outputs = [(await makeSecrets(folder)).output, (await makeSecrets(folder)).output];
    for (const name of GENERATED) {
      for (const output of outputs) expect(output).not.toContain(read(folder, name));
    }
  });

  test("DB-002: an existing file keeps its content, its mode is corrected, and the URL uses it", async () => {
    const folder = secretsFolderWith({ hub_password: "my.own-password_1~" });
    chmodSync(join(folder, "hub_password"), 0o644);
    await makeSecrets(folder);
    expect(read(folder, "hub_password")).toBe("my.own-password_1~");
    expect(mode(folder, "hub_password")).toBe(0o440);
    expect(read(folder, "hub_database_url")).toContain("anima_hub:my.own-password_1~@");
  });

  test("DB-002: a supplied password that is not URL-safe is refused by file name, and nothing else is made", async () => {
    const folder = secretsFolderWith({ owner_password: "pa:ss@word/1" });
    const result = await makeSecrets(folder);
    expect(result.exitCode).not.toBe(0);
    expect(result.stderr).toContain("owner_password");
    expect(result.output).not.toContain("pa:ss@word/1");
    expect(existsSync(join(folder, "owner_database_url"))).toBe(false);
  });

  test("SEC-004: a run leaves no temporary file behind", async () => {
    const folder = newSecretsPath();
    await makeSecrets(folder);
    expect(readdirSync(folder).filter((name) => name.startsWith("."))).toEqual([]);
  });
});

describe("make-secrets existing files (DB-002, SEC-004)", () => {
  const goodKey = Buffer.alloc(32, 7).toString("base64");

  test("DB-002: a password file with a trailing newline is refused by name", async () => {
    const folder = secretsFolderWith({ hub_password: "abc123\n" });
    const result = await makeSecrets(folder);
    expect(result.exitCode).not.toBe(0);
    expect(result.stderr).toContain("hub_password");
    expect(existsSync(join(folder, "hub_database_url"))).toBe(false);
  });

  test.each([
    ["anima_key", Buffer.alloc(31, 7).toString("base64")],
    ["anima_key", "not-base64!"],
    ["anima_key", ""],
    ["cloudflare_dns_token", ""],
    ["hub_database_url", "postgres://anima_hub:other@postgres:5432/anima"],
    ["owner_database_url", "postgres://anima_owner:x@postgres:5432/anima\n"],
  ])("SEC-004: an existing %s with content %j is refused by name and the value is not printed", async (name, content) => {
    const folder = secretsFolderWith({ [name]: content });
    const result = await makeSecrets(folder);
    expect(result.exitCode).not.toBe(0);
    expect(result.stderr).toContain(name);
    if (content.trim() !== "") expect(result.output).not.toContain(content.trim());
  });

  test("SEC-004: an existing key of 32 bytes is kept", async () => {
    const folder = secretsFolderWith({ anima_key: goodKey });
    expect((await makeSecrets(folder)).exitCode).toBe(0);
    expect(read(folder, "anima_key")).toBe(goodKey);
  });

  test("DB-002: a URL file that equals the URL built from its password file is kept", async () => {
    const folder = secretsFolderWith({
      owner_password: "abc",
      owner_database_url: "postgres://anima_owner:abc@postgres:5432/anima",
    });
    expect((await makeSecrets(folder)).exitCode).toBe(0);
    expect(read(folder, "owner_database_url")).toBe("postgres://anima_owner:abc@postgres:5432/anima");
  });
});
