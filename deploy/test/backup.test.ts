import { afterAll, beforeAll, beforeEach, describe, expect, test } from "bun:test";
import { existsSync, mkdtempSync, readdirSync, rmSync, statSync, utimesSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { psql, scalar, serverEnvironment, toolDirectory } from "./postgres.ts";
import { ROOT, installRoles, migrate } from "./roles-setup.ts";

const BACKUP = join(ROOT, "deploy", "backup", "backup.sh");
const RESTORE = join(ROOT, "deploy", "backup", "restore.sh");
// A database of this file's own, so roles.test.ts keeps `anima` untouched.
const DATABASE = "anima_backup_test";

let backupDir = "";

interface ScriptResult {
  exitCode: number;
  stdout: string;
  stderr: string;
}

function run(script: string, args: string[], env: Record<string, string> = {}): ScriptResult {
  const { PGHOST, PGPORT, superuser } = serverEnvironment();
  const result = Bun.spawnSync(["bash", script, ...args], {
    env: {
      ...process.env,
      PATH: `${toolDirectory()}:${process.env["PATH"] ?? ""}`,
      ANIMA_PG_LOCAL: "1",
      ANIMA_DB_NAME: DATABASE,
      ANIMA_DB_SUPERUSER: superuser,
      ANIMA_BACKUP_DIR: backupDir,
      PGHOST,
      PGPORT,
      ...env,
    },
    stdin: "ignore",
  });
  return { exitCode: result.exitCode ?? 1, stdout: result.stdout.toString(), stderr: result.stderr.toString() };
}

function dumps(): string[] {
  return readdirSync(backupDir).filter((name) => name.endsWith(".dump")).sort();
}

function backUp(env: Record<string, string> = {}): string {
  const result = run(BACKUP, [], env);
  expect(result.exitCode).toBe(0);
  return result.stdout.trim();
}

function count(table: string): string {
  return scalar(`select count(*) from ${table}`, { user: "anima_owner", database: DATABASE });
}

function owner(sql: string) {
  const result = psql(["-c", sql], { user: "anima_owner", database: DATABASE });
  if (result.exitCode !== 0) throw new Error(result.stderr);
}

// Same statements as roles.sql, for a database of this file's own.
function createDatabase(): void {
  const result = psql([
    "-c", `drop database if exists ${DATABASE} with (force)`,
    "-c", `create database ${DATABASE} owner anima_owner`,
    "-c", `revoke connect, temporary on database ${DATABASE} from public`,
    "-c", `grant connect on database ${DATABASE} to anima_runtime`,
  ]);
  if (result.exitCode !== 0) throw new Error(result.stderr);
}

beforeAll(() => {
  installRoles();
});

// Every test starts from the same sample data and an empty backup folder.
beforeEach(() => {
  rmSync(backupDir, { recursive: true, force: true });
  backupDir = mkdtempSync(join(tmpdir(), "anima-backups-"));
  createDatabase();
  migrate(DATABASE);
  owner(`insert into devices.sites values ('00000000-0000-0000-0000-000000000001', 'home', '10.0.0.0/24')`);
  owner(`insert into devices.devices (id, site_id, name, host, kind, os, created_at, updated_at) values
    ('00000000-0000-0000-0000-0000000000a1', '00000000-0000-0000-0000-000000000001', 'nas', 'nas.lan', 'server', 'linux', now(), now()),
    ('00000000-0000-0000-0000-0000000000a2', '00000000-0000-0000-0000-000000000001', 'mac', 'mac.lan', 'laptop', 'macos', now(), now())`);
  owner(`insert into devices.device_tags values ('00000000-0000-0000-0000-0000000000a1', 'storage')`);
  // The hub appends through the runtime group's rights.
  const appended = psql(
    [
      "-c",
      `set role anima_runtime;
       select events.ensure_day_partition('audit_log', now());
       select events.ensure_day_partition('event_log', now());
       insert into events.audit_log values (gen_random_uuid(), now(), gen_random_uuid(), 'op', 'device.create', 'device', 'a1', '127.0.0.1', '{}');
       insert into events.audit_log values (gen_random_uuid(), now(), gen_random_uuid(), 'op', 'device.create', 'device', 'a2', '127.0.0.1', '{}');
       insert into events.event_log values (gen_random_uuid(), now(), 'info', 'hub', null, null, 'started')`,
    ],
    { database: DATABASE },
  );
  if (appended.exitCode !== 0) throw new Error(appended.stderr);
});

afterAll(() => {
  rmSync(backupDir, { recursive: true, force: true });
  psql(["-c", `drop database if exists ${DATABASE} with (force)`]);
});

describe("backup.sh (DB-007, VPS-006)", () => {
  test("DB-007: one dump with mode 0600 is written, and no partial file stays", () => {
    const path = backUp();
    expect(dumps()).toEqual([path.split("/").pop() ?? ""]);
    expect(path).toMatch(/anima-\d{8}T\d{6}Z\.dump$/);
    expect(statSync(path).mode & 0o777).toBe(0o600);
    expect(readdirSync(backupDir).filter((name) => name.endsWith(".partial"))).toEqual([]);
  });

  test("DB-007: the backup folder is created with mode 0700", () => {
    const nested = join(backupDir, "new");
    backUp({ ANIMA_BACKUP_DIR: nested });
    expect(statSync(nested).mode & 0o777).toBe(0o700);
  });

  test("DB-007: a failing pg_dump leaves no dump, removes a stale partial file and exits non-zero", () => {
    const stale = join(backupDir, "anima-20000101T000000Z.dump.partial");
    writeFileSync(stale, "half a dump");
    const result = run(BACKUP, [], { ANIMA_DB_NAME: "no_such_database" });
    expect(result.exitCode).not.toBe(0);
    expect(readdirSync(backupDir)).toEqual([]);
  });

  test("DB-007: a failing backup prunes nothing", () => {
    const old = join(backupDir, "anima-20000101T000000Z.dump");
    writeFileSync(old, "old");
    const longAgo = new Date(Date.now() - 100 * 86_400_000);
    utimesSync(old, longAgo, longAgo);
    expect(run(BACKUP, [], { ANIMA_DB_NAME: "no_such_database" }).exitCode).not.toBe(0);
    expect(existsSync(old)).toBe(true);
  });

  test("VPS-006: pruning removes dumps older than the keep days and keeps the new dump", () => {
    const old = join(backupDir, "anima-20000101T000000Z.dump");
    const recent = join(backupDir, "anima-20000102T000000Z.dump");
    writeFileSync(old, "old");
    writeFileSync(recent, "recent");
    const longAgo = new Date(Date.now() - 20 * 86_400_000);
    utimesSync(old, longAgo, longAgo);
    const yesterday = new Date(Date.now() - 86_400_000);
    utimesSync(recent, yesterday, yesterday);
    const path = backUp({ ANIMA_BACKUP_KEEP_DAYS: "14" });
    expect(existsSync(old)).toBe(false);
    expect(existsSync(recent)).toBe(true);
    expect(existsSync(path)).toBe(true);
  });
});

describe("restore.sh --check (DB-007)", () => {
  test("DB-007: --check passes for a fresh dump and leaves no check database", () => {
    const path = backUp();
    const result = run(RESTORE, ["--check", path]);
    expect(result.stderr).toBe("");
    expect(result.exitCode).toBe(0);
    expect(scalar("select count(*) from pg_database where datname = 'anima_restore_check'")).toBe("0");
  });

  test("DB-007: --check fails when the live database differs from the dump", () => {
    const path = backUp();
    owner(`delete from devices.device_tags`);
    const result = run(RESTORE, ["--check", path]);
    expect(result.exitCode).not.toBe(0);
    expect(result.stderr).toContain("devices.device_tags");
    expect(scalar("select count(*) from pg_database where datname = 'anima_restore_check'")).toBe("0");
  });

  test("DB-007: --check fails when the dump lets the runtime group change the audit log", () => {
    owner(`grant update on events.audit_log to anima_runtime`);
    const tampered = backUp();
    owner(`revoke update on events.audit_log from anima_runtime`);
    const result = run(RESTORE, ["--check", tampered]);
    expect(result.exitCode).not.toBe(0);
    expect(result.stderr).toContain("events.audit_log");
  });

  test("DB-007: --check fails when the dump stops the runtime group appending to the event log", () => {
    owner(`revoke insert on events.event_log from anima_runtime`);
    const result = run(RESTORE, ["--check", backUp()]);
    expect(result.exitCode).not.toBe(0);
    expect(result.stderr).toContain("events.event_log");
  });

  test("DB-007: --check never prints row contents", () => {
    const path = backUp();
    owner(`delete from devices.device_tags`);
    const result = run(RESTORE, ["--check", path]);
    expect(result.stdout + result.stderr).not.toContain("storage");
  });

});

describe("restore.sh --yes (DB-007)", () => {
  test("DB-007: restore refuses without --yes and changes nothing", () => {
    const path = backUp();
    owner(`delete from devices.devices`);
    const result = run(RESTORE, [path]);
    expect(result.exitCode).not.toBe(0);
    expect(count("devices.devices")).toBe("0");
  });

  test("DB-007: a full restore brings back every row and keeps the privileges", () => {
    const path = backUp();
    owner(`delete from devices.devices where name = 'mac'`);
    owner(`delete from devices.device_tags`);
    const result = run(RESTORE, ["--yes", path]);
    expect(result.exitCode).toBe(0);
    expect(result.stdout).toContain("migrate");
    for (const [table, rows] of [
      ["devices.sites", "1"],
      ["devices.devices", "2"],
      ["devices.device_tags", "1"],
      ["events.audit_log", "2"],
      ["events.event_log", "1"],
    ] as const) {
      expect(count(table)).toBe(rows);
    }
    const can = (privilege: string) =>
      scalar(`select has_table_privilege('anima_hub', 'events.audit_log', '${privilege}')`, { database: DATABASE });
    expect([can("select"), can("insert"), can("update"), can("delete"), can("truncate")]).toEqual(["t", "t", "f", "f", "f"]);
    expect(scalar(`select has_database_privilege('anima_hub', '${DATABASE}', 'connect')`)).toBe("t");
    expect(scalar(`select has_database_privilege('public', '${DATABASE}', 'connect')`)).toBe("f");
    expect(scalar(`select pg_get_userbyid(datdba) from pg_database where datname = '${DATABASE}'`)).toBe("anima_owner");
  });

  test("DB-007: after a restore, a table the owner creates still reaches the runtime group", () => {
    const path = backUp();
    expect(run(RESTORE, ["--yes", path]).exitCode).toBe(0);
    owner(`create table devices.after_restore (id int)`);
    expect(
      scalar(`select has_table_privilege('anima_hub', 'devices.after_restore', 'insert')`, { database: DATABASE }),
    ).toBe("t");
  });
});
