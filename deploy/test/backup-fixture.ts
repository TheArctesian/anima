// Shared setup for the backup, restore and restore-check tests: a sample database, the scripts
// run against the throwaway server in local mode, and small readers.
import { setDefaultTimeout } from "bun:test";
import { chmodSync, existsSync, mkdtempSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { binaryDirs, findTools } from "../../quality/src/gates/postgres.ts";
import { finish, type FinishedProcess } from "./finished-process.ts";
import { psql, scalar, serverDirectory } from "./postgres.ts";
import { ROOT, installRoles, migrate } from "./roles-setup.ts";

// Each test makes a database, migrates it and runs a script or two; a loaded machine needs time.
setDefaultTimeout(60_000);

export const BACKUP = join(ROOT, "deploy", "backup", "backup.sh");
export const RESTORE = join(ROOT, "deploy", "backup", "restore.sh");
export const RESTORE_CHECK = join(ROOT, "deploy", "backup", "restore-check.sh");
// A database of this suite's own, so roles.test.ts keeps `anima` untouched.
export const DATABASE = "anima_backup_test";

export type ScriptResult = FinishedProcess;

export const state = { backupDir: "" };

/** Directory that holds psql, pg_dump and pg_restore. */
export function toolDirectory(): string {
  const dir = findTools(binaryDirs(process.env["PATH"] ?? ""), ["psql", "pg_dump", "pg_restore"], existsSync);
  if (dir === undefined) throw new Error("psql, pg_dump or pg_restore not found; install PostgreSQL 17 (CQ-024: these tests never skip)");
  return dir;
}

function serverEnvironment(): Record<string, string> {
  const url = new URL(process.env["DATABASE_URL"] ?? "");
  return {
    PGHOST: serverDirectory(),
    PGPORT: url.port || "5432",
    ANIMA_DB_SUPERUSER: decodeURIComponent(url.username),
  };
}

/** Runs a script in local mode. `shimDirectory` goes first on PATH. */
export async function run(
  script: string,
  args: string[],
  env: Record<string, string | undefined> = {},
  shimDirectory?: string,
): Promise<ScriptResult> {
  const path = [shimDirectory, toolDirectory(), process.env["PATH"]].filter(Boolean).join(":");
  return finish(["bash", script, ...args], {
    env: {
      ...process.env,
      PATH: path,
      ANIMA_PG_LOCAL: "1",
      ANIMA_DB_NAME: DATABASE,
      ANIMA_BACKUP_DIR: state.backupDir,
      ...serverEnvironment(),
      ...env,
    },
  });
}

/** Files in the backup folder whose names end with `suffix`. */
export function backupFiles(suffix: string): string[] {
  return readdirSync(state.backupDir).filter((name) => name.endsWith(suffix)).sort();
}

/** Runs backup.sh, expects success and returns the dump path. */
export async function backUp(env: Record<string, string> = {}, shimDirectory?: string): Promise<string> {
  const result = await run(BACKUP, [], env, shimDirectory);
  if (result.exitCode !== 0) throw new Error(`backup.sh failed: ${result.stderr}`);
  return result.stdout.trim();
}

export async function owner(sql: string, database = DATABASE): Promise<void> {
  const result = await psql(["-c", sql], { user: "anima_owner", database });
  if (result.exitCode !== 0) throw new Error(result.stderr);
}

export async function count(table: string): Promise<string> {
  return scalar(`select count(*) from ${table}`, { user: "anima_owner", database: DATABASE });
}

/** Names of databases that start with `prefix`. */
export async function databasesLike(prefix: string): Promise<string[]> {
  const names = await scalar(`select coalesce(string_agg(datname, ',' order by datname), '') from pg_database where datname like '${prefix}%'`);
  return names.split(",").filter(Boolean);
}

// Same statements as roles.sql, for a database of this suite's own.
export async function createDatabase(name = DATABASE): Promise<void> {
  const result = await psql([
    "-c", `drop database if exists ${name} with (force)`,
    "-c", `create database ${name} owner anima_owner`,
    "-c", `revoke connect, temporary on database ${name} from public`,
    "-c", `grant connect on database ${name} to anima_runtime`,
  ]);
  if (result.exitCode !== 0) throw new Error(result.stderr);
}

/** A fresh backup folder, and the sample database with rows in several tables. */
export async function resetSample(): Promise<void> {
  rmSync(state.backupDir, { recursive: true, force: true });
  state.backupDir = mkdtempSync(join(tmpdir(), "anima-backups-"));
  await installRoles();
  for (const name of await databasesLike(DATABASE)) await psql(["-c", `drop database if exists "${name}" with (force)`]);
  await createDatabase();
  await migrate(DATABASE);
  await owner(`insert into devices.sites values ('00000000-0000-0000-0000-000000000001', 'home', '10.0.0.0/24')`);
  await owner(`insert into devices.devices (id, site_id, name, host, kind, os, created_at, updated_at) values
    ('00000000-0000-0000-0000-0000000000a1', '00000000-0000-0000-0000-000000000001', 'nas', 'nas.lan', 'server', 'linux', now(), now()),
    ('00000000-0000-0000-0000-0000000000a2', '00000000-0000-0000-0000-000000000001', 'mac', 'mac.lan', 'laptop', 'macos', now(), now())`);
  await owner(`insert into devices.device_tags values ('00000000-0000-0000-0000-0000000000a1', 'storage')`);
  await appendLogs();
}

/** The hub appends to the logs through the runtime group's rights. */
export async function appendLogs(): Promise<void> {
  const appended = await psql(
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
}

/** Removes the folder and every database the suite made. */
export async function tearDownSample(): Promise<void> {
  rmSync(state.backupDir, { recursive: true, force: true });
  for (const name of await databasesLike(DATABASE)) await psql(["-c", `drop database if exists "${name}" with (force)`]);
  await psql(["-c", "drop database if exists anima_restore_check with (force)"]);
}

/**
 * A dump that lists fine but fails halfway to restore: it grants a table to a role that the
 * cluster no longer has, so the failure comes after the tables are made.
 */
export async function backUpWithMissingRole(): Promise<string> {
  await psql(["-c", "drop role if exists anima_gone"]);
  await psql(["-c", "create role anima_gone"]);
  await owner("grant select on devices.sites to anima_gone");
  const path = await backUp();
  await owner("revoke select on devices.sites from anima_gone");
  const dropped = await psql(["-c", "drop role anima_gone"]);
  if (dropped.exitCode !== 0) throw new Error(dropped.stderr);
  return path;
}

/**
 * A directory with a fake `docker`. `ps` lists $FAKE_RUNNING from its $FAKE_RUNNING_FROM-th call on
 * (default the first) and records its arguments in $FAKE_CALLS; `exec` runs the command locally.
 */
export function dockerShim(): string {
  const folder = mkdtempSync(join(tmpdir(), "anima-docker-"));
  const script = [
    "#!/bin/sh",
    'case " $* " in',
    '  *" ps "*)',
    '    echo "$*" >> "$FAKE_CALLS"',
    '    n=$(wc -l < "$FAKE_CALLS" | tr -d " ")',
    '    if [ -n "$FAKE_RUNNING" ] && [ "$n" -ge "${FAKE_RUNNING_FROM:-1}" ]; then echo "$FAKE_RUNNING"; fi',
    "    exit 0 ;;",
    '  *" exec "*) while [ "$1" != postgres ]; do shift; done; shift; exec "$@" ;;',
    "esac",
    "exit 1",
  ].join("\n");
  writeFileSync(join(folder, "docker"), `${script}\n`);
  chmodSync(join(folder, "docker"), 0o755);
  return folder;
}

/** A directory with an executable `name` that runs `body` with sh. */
export function shimDirectory(name: string, body: string): string {
  const folder = mkdtempSync(join(tmpdir(), "anima-shim-"));
  writeFileSync(join(folder, name), `#!/bin/sh\n${body}\n`);
  chmodSync(join(folder, name), 0o755);
  return folder;
}
