import { afterAll, beforeEach, describe, expect, test } from "bun:test";
import { existsSync, mkdirSync, readFileSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { BACKUP, DATABASE, RESTORE_CHECK, backUp, createDatabase, shimDirectory, backupFiles, resetSample, run, state, tearDownSample, toolDirectory } from "./backup-fixture.ts";

beforeEach(resetSample);
afterAll(tearDownSample);

function writeOld(stamp: string): string {
  const path = join(state.backupDir, `anima-${stamp}.dump`);
  writeFileSync(path, "old");
  writeFileSync(`${path}.counts`, "old");
  return path;
}

function utcStamp(daysAgo: number): string {
  return new Date(Date.now() - daysAgo * 86_400_000).toISOString().replace(/[-:]|\.\d+/g, "");
}

describe("backup.sh files (DB-007, VPS-006)", () => {
  test("DB-007: one dump and one counts manifest are written, both mode 0600, no partial file stays", async () => {
    const path = await backUp();
    expect(backupFiles(".dump")).toEqual([path.split("/").pop() ?? ""]);
    expect(path).toMatch(/anima-\d{8}T\d{6}Z\.dump$/);
    for (const file of [path, `${path}.counts`]) expect(statSync(file).mode & 0o777).toBe(0o600);
    expect(readdirSync(state.backupDir).filter((name) => name.includes(".partial"))).toEqual([]);
  });

  test("DB-007: the manifest holds table names and counts only", async () => {
    const path = await backUp();
    const lines = readFileSync(`${path}.counts`, "utf8").trim().split("\n");
    expect(lines).toContain("devices.devices 2");
    expect(lines).toContain("events.audit_log 2");
    for (const line of lines) expect(line).toMatch(/^[a-z_.0-9]+ \d+$/);
    expect(readFileSync(`${path}.counts`, "utf8")).not.toContain("nas");
  });

  test("DB-007: a row written between the snapshot and the dump is in neither, so the drill still passes", async () => {
    const real = join(toolDirectory(), "pg_dump");
    const shim = shimDirectory(
      "pg_dump",
      `psql -X -q -U anima_owner -d ${DATABASE} -c "insert into devices.device_tags values ('00000000-0000-0000-0000-0000000000a2', 'late')"\nexec "${real}" "$@"`,
    );
    const path = await backUp({}, shim);
    expect(readFileSync(`${path}.counts`, "utf8")).toContain("devices.device_tags 1\n");
    expect((await run(RESTORE_CHECK, [path])).exitCode).toBe(0);
  });

  test("DB-007: the backup folder is created with mode 0700", async () => {
    const nested = join(state.backupDir, "new");
    await backUp({ ANIMA_BACKUP_DIR: nested });
    expect(statSync(nested).mode & 0o777).toBe(0o700);
  });
});

describe("backup.sh failures (DB-007)", () => {
  test("DB-007: a failing pg_dump leaves no dump, removes a stale partial file and exits non-zero", async () => {
    writeFileSync(join(state.backupDir, "anima-20000101T000000Z.dump.partial"), "half a dump");
    const result = await run(BACKUP, [], { ANIMA_DB_NAME: "no_such_database" });
    expect(result.exitCode).not.toBe(0);
    expect(readdirSync(state.backupDir).filter((name) => !name.startsWith(".lock"))).toEqual([]);
  });

  test("DB-007: a pg_dump that exits 0 and writes garbage leaves no dump", async () => {
    const shim = shimDirectory("pg_dump", "echo garbage");
    const result = await run(BACKUP, [], {}, shim);
    expect(result.exitCode).not.toBe(0);
    expect(backupFiles(".dump")).toEqual([]);
    expect(backupFiles(".counts")).toEqual([]);
  });

  test("DB-007: a pg_dump that exits 3 gives exactly one stderr line, with no shell job message", async () => {
    const shim = shimDirectory("pg_dump", "exit 3");
    const result = await run(BACKUP, [], {}, shim);
    expect(result.exitCode).not.toBe(0);
    expect(result.stderr).toBe("backup.sh: pg_dump failed\n");
    expect(backupFiles(".dump")).toEqual([]);
  });

  test("DB-007: a bad ANIMA_BACKUP_KEEP_DAYS fails before any dump is made", async () => {
    const result = await run(BACKUP, [], { ANIMA_BACKUP_KEEP_DAYS: "abc" });
    expect(result.exitCode).not.toBe(0);
    expect(readdirSync(state.backupDir)).toEqual([]);
  });

  test("VPS-006: a failing backup prunes nothing", async () => {
    const old = writeOld("20000101T000000Z");
    expect((await run(BACKUP, [], { ANIMA_DB_NAME: "no_such_database" })).exitCode).not.toBe(0);
    expect(existsSync(old)).toBe(true);
  });

  test("DB-007: a second run while the lock folder exists fails and leaves the lock", async () => {
    mkdirSync(join(state.backupDir, ".lock.d"));
    const result = await run(BACKUP, [], { ANIMA_BACKUP_LOCK: "folder" });
    expect(result.exitCode).not.toBe(0);
    expect(result.stderr).toContain("another backup");
    expect(existsSync(join(state.backupDir, ".lock.d"))).toBe(true);
  });

  test("DB-007: when flock refuses the lock, no backup runs", async () => {
    const result = await run(BACKUP, [], {}, shimDirectory("flock", "exit 1"));
    expect(result.exitCode).not.toBe(0);
    expect(result.stderr).toContain("another backup is running");
    expect(backupFiles(".dump")).toEqual([]);
  });

  test("DB-007: an empty database gives no dump", async () => {
    await createDatabase("anima_backup_test_empty");
    const result = await run(BACKUP, [], { ANIMA_DB_NAME: "anima_backup_test_empty" });
    expect(result.exitCode).not.toBe(0);
    expect(result.stderr).toContain("no tables");
    expect(backupFiles(".dump")).toEqual([]);
    expect(backupFiles(".counts")).toEqual([]);
  });

  test("DB-007: a good run frees the lock folder", async () => {
    await backUp({ ANIMA_BACKUP_LOCK: "folder" });
    expect(existsSync(join(state.backupDir, ".lock.d"))).toBe(false);
  });
});

describe("backup.sh pruning (VPS-006)", () => {
  test("VPS-006: pruning goes by the time in the name and removes dump and manifest together", async () => {
    const old = writeOld("20000101T000000Z");
    const recent = writeOld(utcStamp(1));
    const path = await backUp({ ANIMA_BACKUP_KEEP_DAYS: "14" });
    expect(existsSync(old) || existsSync(`${old}.counts`)).toBe(false);
    expect(existsSync(recent) && existsSync(`${recent}.counts`)).toBe(true);
    expect(existsSync(path) && existsSync(`${path}.counts`)).toBe(true);
  });

  test.each(["anima-x.dump", "anima-.dump", "anima-1.dump", "anima-2000.restored.dump", "anima-20000101T000000Z.dump.bak"])(
    "VPS-006: pruning leaves %s alone",
    async (name) => {
      writeFileSync(join(state.backupDir, name), "mine");
      await backUp({ ANIMA_BACKUP_KEEP_DAYS: "0" });
      expect(existsSync(join(state.backupDir, name))).toBe(true);
    },
  );
});
