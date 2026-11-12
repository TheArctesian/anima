import { beforeEach, describe, expect, test } from "bun:test";
import { chmodSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { ROOT } from "./roles-setup.ts";

const OFFSITE = join(ROOT, "deploy", "backup", "offsite.sh");
const PASSWORD = "very-secret-restic-password";

let folder = "";
let calls = "";

// A fake restic records its arguments, and fails when FAKE_RESTIC_FAIL is set.
function setUp(): void {
  folder = mkdtempSync(join(tmpdir(), "anima-offsite-"));
  calls = join(folder, "calls");
  const fake = join(folder, "restic");
  writeFileSync(fake, `#!/bin/sh\nprintf '%s\\n' "$*" >> "${calls}"\necho RESTIC-STDOUT-LINE\n[ -z "$FAKE_RESTIC_FAIL" ] || { echo "$FAKE_RESTIC_FAIL" >&2; exit 1; }\n`);
  chmodSync(fake, 0o755);
  writeFileSync(join(folder, "repo"), "rest:https://user:hunter2@home.example/anima-db\n");
  writeFileSync(join(folder, "pw"), `${PASSWORD}\n`);
  for (const name of ["anima-20260101T000000Z.dump", "anima-20260102T000000Z.dump"]) {
    writeFileSync(join(folder, name), "x");
    writeFileSync(join(folder, `${name}.counts`), "x");
  }
}

function offsite(args: string[], env: Record<string, string> = {}) {
  const result = Bun.spawnSync(["bash", OFFSITE, ...args], {
    env: {
      ...process.env,
      RESTIC: join(folder, "restic"),
      ANIMA_BACKUP_DIR: folder,
      ANIMA_RESTIC_REPOSITORY_FILE: join(folder, "repo"),
      ANIMA_RESTIC_PASSWORD_FILE: join(folder, "pw"),
      ...env,
    },
    stdin: "ignore",
  });
  return { exitCode: result.exitCode ?? 1, out: result.stdout.toString() + result.stderr.toString() };
}

beforeEach(setUp);

describe("offsite.sh (VPS-006)", () => {
  test("VPS-006: sends the newest dump and its manifest with the secret files named, not read", () => {
    const result = offsite([]);
    expect(result.exitCode).toBe(0);
    const line = readFileSync(calls, "utf8").trim();
    expect(line).toContain(`--repository-file ${folder}/repo --password-file ${folder}/pw backup`);
    expect(line).toContain(`${folder}/anima-20260102T000000Z.dump ${folder}/anima-20260102T000000Z.dump.counts`);
    expect(line).not.toContain("20260101");
    expect(line).toContain("backup --host anima-vps --tag anima-db ");
    expect(result.out).not.toContain("RESTIC-STDOUT-LINE");
    expect(result.out).not.toContain(PASSWORD);
    expect(result.out).not.toContain("hunter2");
  });

  test.each(["anima-x.dump", "anima-.dump", "anima-1.dump", "anima-9999.restored.dump"])(
    "VPS-006: the newest dump is never picked from the odd name %s",
    (name) => {
      writeFileSync(join(folder, name), "x");
      writeFileSync(join(folder, `${name}.counts`), "x");
      expect(offsite([]).exitCode).toBe(0);
      const line = readFileSync(calls, "utf8");
      expect(line).toContain("anima-20260102T000000Z.dump ");
      expect(line).not.toContain(name);
    },
  );

  test("VPS-006: a failing restic fails the run and prints no secret", () => {
    const result = offsite([], { FAKE_RESTIC_FAIL: "repository unreachable" });
    expect(result.exitCode).not.toBe(0);
    expect(result.out).toContain("no snapshot is guaranteed");
    expect(result.out).not.toContain(PASSWORD);
  });

  test.each([["ANIMA_RESTIC_PASSWORD_FILE"], ["ANIMA_RESTIC_REPOSITORY_FILE"]])("VPS-006: a missing %s fails before restic runs", (name) => {
    expect(offsite([], { [name]: join(folder, "missing") }).exitCode).not.toBe(0);
    expect(() => readFileSync(calls)).toThrow();
  });

  test("VPS-006: a dump without its manifest is refused", () => {
    expect(offsite([join(folder, "anima-20260101T000000Z.dump")]).exitCode).toBe(0);
    writeFileSync(join(folder, "lonely.dump"), "x");
    const result = offsite([join(folder, "lonely.dump")]);
    expect(result.exitCode).not.toBe(0);
    expect(result.out).toContain("manifest");
  });
});

describe("the backup holds the database only (IOS-004)", () => {
  test.each(["backup.sh", "offsite.sh"])("IOS-004: %s never names a Docker volume or its path", (name) => {
    const script = readFileSync(join(ROOT, "deploy", "backup", name), "utf8");
    expect(script).not.toMatch(/docker\s+volume|\/var\/lib\/docker\/volumes/);
  });
});
