import { afterEach, describe, expect, test } from "bun:test";
import { chmodSync, existsSync, mkdirSync, readFileSync, readdirSync, symlinkSync } from "node:fs";
import { join } from "node:path";
import {
  makeSecrets,
  newSecretsPath,
  read,
  removeScratchFolders,
  scratchFolder,
  shimFolder,
  toolsOnly,
} from "./make-secrets.ts";

afterEach(removeScratchFolders);

const TOOLS = ["dirname", "mkdir", "mktemp", "chmod", "chown", "mv", "rm", "cat", "ls", "find", "id", "stty", "cut"];

describe("make-secrets producers (DB-002, SEC-004)", () => {
  test("SEC-004: without openssl it fails, stores nothing, and a later run makes correct files", () => {
    const folder = newSecretsPath();
    const result = makeSecrets(folder, { path: toolsOnly(TOOLS) });
    expect(result.exitCode).not.toBe(0);
    expect(result.stderr).toContain("openssl");
    expect(existsSync(folder) ? readdirSync(folder) : []).toEqual([]);
    expect(makeSecrets(folder).exitCode).toBe(0);
    expect(read(folder, "owner_password")).toMatch(/^[0-9a-f]{64}$/);
    expect(Buffer.from(read(folder, "anima_key"), "base64")).toHaveLength(32);
  });

  test.each([
    ["fails", "exit 1"],
    ["prints nothing", "exit 0"],
    ["prints a short value", "printf 'abcd\\n'"],
    ["prints an unexpected character", "printf '%064d\\n' 0 | tr 0 g"],
  ])("SEC-004: when openssl %s, nothing is stored", (_name, body) => {
    const folder = newSecretsPath();
    const result = makeSecrets(folder, { shimDirectories: [shimFolder({ openssl: body })] });
    expect(result.exitCode).not.toBe(0);
    expect(readdirSync(folder)).toEqual([]);
  });

  test("SEC-004: when chmod fails, the temporary file is removed", () => {
    const folder = newSecretsPath();
    const result = makeSecrets(folder, { shimDirectories: [shimFolder({ chmod: "exit 1" })] });
    expect(result.exitCode).not.toBe(0);
    expect(readdirSync(folder)).toEqual([]);
  });
});

describe("make-secrets folder trust (SEC-004)", () => {
  test("SEC-004: a folder that the current user does not own is refused", () => {
    const folder = newSecretsPath();
    const result = makeSecrets(folder, { shimDirectories: [shimFolder({ find: "exit 0" })] });
    expect(result.exitCode).not.toBe(0);
    expect(result.stderr).toContain("does not belong");
    expect(readdirSync(folder)).toEqual([]);
  });

  test("SEC-004: a folder that group or others can write is refused", () => {
    const folder = newSecretsPath();
    mkdirSync(folder, { mode: 0o700 });
    chmodSync(folder, 0o770);
    const result = makeSecrets(folder);
    expect(result.exitCode).not.toBe(0);
    expect(result.stderr).toContain("writable");
    expect(readdirSync(folder)).toEqual([]);
  });

  test("SEC-004: a symbolic link at a secret name is refused", () => {
    const folder = newSecretsPath();
    mkdirSync(folder, { mode: 0o700 });
    symlinkSync(join(scratchFolder(), "elsewhere"), join(folder, "hub_password"));
    const result = makeSecrets(folder);
    expect(result.exitCode).not.toBe(0);
    expect(result.stderr).toContain("hub_password");
    expect(existsSync(join(folder, "postgres_password"))).toBe(false);
  });
});

describe("make-secrets owners (DB-002)", () => {
  test("DB-002: every file gets its owner, and chown runs before chmod", () => {
    const folder = newSecretsPath();
    const log = join(scratchFolder(), "calls.log");
    const record = (command: string) => `printf '%s %s %s\\n' ${command} "$1" "$(basename "$2")" >>"${log}"`;
    const shims = shimFolder({ chown: record("chown"), chmod: `${record("chmod")}\n/bin/chmod "$@"` });
    const result = makeSecrets(folder, {
      chown: "1",
      shimDirectories: [shims],
      args: ["--cloudflare-token-stdin"],
      stdin: "cf-token-value\n",
    });
    expect(result.exitCode).toBe(0);
    const calls = readFileSync(log, "utf8").trim().split("\n").map((line) => line.split(" "));
    const owners: Record<string, string> = {};
    for (const [command = "", argument = "", file = ""] of calls) {
      const name = file.replace(/^\.(.*)\.[^.]+$/, "$1");
      if (command === "chown") {
        owners[name] = argument;
      } else {
        expect(owners[name]).toBeDefined();
      }
    }
    expect(owners).toEqual({
      postgres_password: "root:999",
      owner_password: "root:999",
      hub_password: "root:999",
      owner_database_url: "root:10001",
      hub_database_url: "root:10001",
      anima_key: "root:10001",
      cloudflare_dns_token: "root:root",
    });
  });
});
