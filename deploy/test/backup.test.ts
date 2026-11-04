import { afterAll, beforeEach, describe, expect, test } from "bun:test";
import { chmodSync, existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { BACKUP, DATABASE, appendLogs, backUp, backupFiles, resetSample, run, state, tearDownSample } from "./backup-fixture.ts";

beforeEach(resetSample);
afterAll(tearDownSample);

function writeOld(stamp: string): string {
  const path = join(state.backupDir, `anima-${stamp}.dump`);
  writeFileSync(path, "old");
  writeFileSync(`${path}.counts`, "old");
  return path;
}

function shimDirectory(name: string, body: string): string {
  const folder = mkdtempSync(join(tmpdir(), "anima-shim-"));
  writeFileSync(join(folder, name), `#!/bin/sh\n${body}\n`);
  chmodSync(join(folder, name), 0o755);
  return folder;
}

function utcStamp(daysAgo: number): string {
  return new Date(Date.now() - daysAgo * 86_400_000).toISOString().replace(/[-:]|\.\d+/g, "");
}

describe("backup.sh files (DB-007, VPS-006)", () => {
  test("DB-007: one dump and one counts manifest are written, both mode 0600, no partial file stays", () => {
    const path = backUp();
    expect(backupFiles(".dump")).toEqual([path.split("/").pop() ?? ""]);
    expect(path).toMatch(/anima-\d{8}T\d{6}Z\.dump$/);
    for (const file of [path, `${path}.counts`]) expect(statSync(file).mode & 0o777).toBe(0o600);
    expect(readdirSync(state.backupDir).filter((name) => name.includes(".partial"))).toEqual([]);
  });

  test("DB-007: the manifest holds table names and counts only", () => {
    const path = backUp();
    const lines = readFileSync(`${path}.counts`, "utf8").trim().split("\n");
    expect(lines).toContain("devices.devices 2");
    expect(lines).toContain("events.audit_log 2");
    for (const line of lines) expect(line).toMatch(/^[a-z_.0-9]+ \d+$/);
    expect(readFileSync(`${path}.counts`, "utf8")).not.toContain("nas");
  });

  test("DB-007: the manifest matches the snapshot the dump reads, not later writes", () => {
    const path = backUp();
    appendLogs();
    expect(readFileSync(`${path}.counts`, "utf8")).toContain("events.audit_log 2\n");
  });

  test("DB-007: the backup folder is created with mode 0700", () => {
    const nested = join(state.backupDir, "new");
    backUp({ ANIMA_BACKUP_DIR: nested });
    expect(statSync(nested).mode & 0o777).toBe(0o700);
  });
});

describe("backup.sh failures (DB-007)", () => {
  test("DB-007: a failing pg_dump leaves no dump, removes a stale partial file and exits non-zero", () => {
    writeFileSync(join(state.backupDir, "anima-20000101T000000Z.dump.partial"), "half a dump");
    const result = run(BACKUP, [], { ANIMA_DB_NAME: "no_such_database" });
    expect(result.exitCode).not.toBe(0);
    expect(readdirSync(state.backupDir).filter((name) => !name.startsWith(".lock"))).toEqual([]);
  });

  test("DB-007: a pg_dump that exits 0 and writes garbage leaves no dump", () => {
    const shim = shimDirectory("pg_dump", "echo garbage");
    const result = run(BACKUP, [], {}, shim);
    expect(result.exitCode).not.toBe(0);
    expect(backupFiles(".dump")).toEqual([]);
    expect(backupFiles(".counts")).toEqual([]);
  });

  test("DB-007: a bad ANIMA_BACKUP_KEEP_DAYS fails before any dump is made", () => {
    const result = run(BACKUP, [], { ANIMA_BACKUP_KEEP_DAYS: "abc" });
    expect(result.exitCode).not.toBe(0);
    expect(readdirSync(state.backupDir)).toEqual([]);
  });

  test("VPS-006: a failing backup prunes nothing", () => {
    const old = writeOld("20000101T000000Z");
    expect(run(BACKUP, [], { ANIMA_DB_NAME: "no_such_database" }).exitCode).not.toBe(0);
    expect(existsSync(old)).toBe(true);
  });

  test("DB-007: a second run while the lock folder exists fails and leaves the lock", () => {
    mkdirSync(join(state.backupDir, ".lock.d"));
    const result = run(BACKUP, [], { ANIMA_BACKUP_LOCK: "folder" });
    expect(result.exitCode).not.toBe(0);
    expect(result.stderr).toContain("another backup");
    expect(existsSync(join(state.backupDir, ".lock.d"))).toBe(true);
  });

  test("DB-007: a good run frees the lock folder", () => {
    backUp({ ANIMA_BACKUP_LOCK: "folder" });
    expect(existsSync(join(state.backupDir, ".lock.d"))).toBe(false);
  });
});

describe("backup.sh pruning (VPS-006)", () => {
  test("VPS-006: pruning goes by the time in the name and removes dump and manifest together", () => {
    const old = writeOld("20000101T000000Z");
    const recent = writeOld(utcStamp(1));
    const path = backUp({ ANIMA_BACKUP_KEEP_DAYS: "14" });
    expect(existsSync(old) || existsSync(`${old}.counts`)).toBe(false);
    expect(existsSync(recent) && existsSync(`${recent}.counts`)).toBe(true);
    expect(existsSync(path) && existsSync(`${path}.counts`)).toBe(true);
  });
});
