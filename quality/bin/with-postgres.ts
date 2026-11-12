// Runs a command with DATABASE_URL pointing at a throwaway PostgreSQL server (ruling R-009).
// If DATABASE_URL is already set, the command uses that server and nothing is started.
// A missing PostgreSQL installation fails the gate; database tests are never skipped (CQ-024).
// Before the command runs, the hub's migrations are applied to `template1` as sqlx applies them, so each
// `#[sqlx::test]` database (a bare `create database`, a copy of `template1`) starts migrated (WP-55, R-112).
import { existsSync, mkdtempSync, readFileSync, readdirSync, rmSync, statSync } from "node:fs";
import { tmpdir, userInfo } from "node:os";
import { join } from "node:path";
import { LEDGER_DDL, applyStatements, planMigrations, sqlxSessionEnv } from "../src/gates/migrations.ts";
import {
  POSTGRES_MAJOR,
  binaryDirs,
  findTools,
  psqlArgv,
  serverOptions,
  socketDatabaseUrl,
  type SocketServer,
} from "../src/gates/postgres.ts";
import { REPO_ROOT } from "../src/repository.ts";

const PORT = 5432;
/** The folder every `#[sqlx::test(migrations = "./migrations")]` in the hub names. */
const HUB_MIGRATIONS = join(REPO_ROOT, "crates", "hub", "migrations");
const command = process.argv.slice(2);
if (command.length === 0) {
  console.error("with-postgres: give the command to run, for example `cargo test`");
  process.exit(2);
}

function run(argv: string[], env: Record<string, string | undefined> = process.env): number {
  const result = Bun.spawnSync(argv, { cwd: REPO_ROOT, env, stdin: "inherit", stdout: "inherit", stderr: "inherit" });
  return result.exitCode ?? 1;
}

if (process.env["DATABASE_URL"]) {
  process.exit(run(command));
}

const binDir = findTools(binaryDirs(process.env["PATH"] ?? ""), ["initdb", "pg_ctl", "psql"], existsSync);
if (binDir === undefined) {
  console.error(
    `with-postgres: initdb, pg_ctl and psql not found together. Install PostgreSQL ${POSTGRES_MAJOR} ` +
      `(macOS: brew install postgresql@${POSTGRES_MAJOR}) or set DATABASE_URL to a server you own.`,
  );
  process.exit(1);
}

const root = mkdtempSync(join(tmpdir(), "anima-pg-"));

/** Runs statements in `template1` with sqlx's session settings; psql exits, so no session stays open. */
function inTemplate(tools: string, server: SocketServer, statements: string[], what: string): void {
  const psql = Bun.spawnSync(psqlArgv(join(tools, "psql"), server, "template1", statements), {
    env: sqlxSessionEnv(process.env),
    stdin: "ignore",
    stdout: "ignore",
    stderr: "pipe",
  });
  if (psql.exitCode !== 0) {
    throw new Error(`migrating template1 failed at ${what} (psql exit ${psql.exitCode}):\n${psql.stderr.toString()}`);
  }
}

/** Applies the hub's migrations to `template1` and records them in `_sqlx_migrations`, as sqlx would. */
function migrateTemplate(tools: string, server: SocketServer): void {
  const files = readdirSync(HUB_MIGRATIONS)
    .filter((name) => statSync(join(HUB_MIGRATIONS, name)).isFile())
    .map((name) => ({ name, bytes: readFileSync(join(HUB_MIGRATIONS, name)) }));
  const migrations = planMigrations(files);
  const began = performance.now();
  inTemplate(tools, server, [LEDGER_DDL], "the _sqlx_migrations table");
  for (const migration of migrations) inTemplate(tools, server, applyStatements(migration), migration.file);
  const seconds = ((performance.now() - began) / 1000).toFixed(1);
  console.log(`with-postgres: template1 migrated (${migrations.length} migrations, ${seconds}s)`);
}

/** Start a server in `root`, run the command against it, stop the server. Returns the command's exit code. */
function runWithServer(tools: string): number {
  const dataDir = join(root, "data");
  const user = userInfo().username;
  // macOS PostgreSQL refuses to start without a valid locale ("postmaster became multithreaded").
  const serverEnv = { ...process.env, LC_ALL: "C", LANG: "C" };
  const quiet = { stdout: "ignore", stderr: "inherit", env: serverEnv } as const;
  const initdb = Bun.spawnSync(
    [join(tools, "initdb"), "-D", dataDir, "-U", user, "-A", "trust", "-E", "UTF8", "--locale=C", "--no-sync"],
    quiet,
  );
  if (initdb.exitCode !== 0) throw new Error(`initdb failed with exit ${initdb.exitCode}`);
  const start = Bun.spawnSync(
    [join(tools, "pg_ctl"), "-D", dataDir, "-l", join(root, "server.log"), "-o", serverOptions(root, PORT), "-w", "start"],
    quiet,
  );
  if (start.exitCode !== 0) throw new Error(`pg_ctl start failed with exit ${start.exitCode}; see ${join(root, "server.log")}`);
  try {
    migrateTemplate(tools, { socketDir: root, port: PORT, user });
    return run(command, { ...process.env, DATABASE_URL: socketDatabaseUrl(root, PORT, user) });
  } finally {
    Bun.spawnSync([join(tools, "pg_ctl"), "-D", dataDir, "-m", "immediate", "-w", "stop"], quiet);
  }
}

try {
  const code = runWithServer(binDir);
  rmSync(root, { recursive: true, force: true });
  process.exit(code);
} catch (error) {
  console.error(`with-postgres: ${error instanceof Error ? error.message : String(error)}`);
  process.exit(1);
}
