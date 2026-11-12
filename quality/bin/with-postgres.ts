// Runs a command with DATABASE_URL pointing at a throwaway PostgreSQL server (ruling R-009).
// If DATABASE_URL is already set, the command uses that server and nothing is started.
// A missing PostgreSQL installation fails the gate; database tests are never skipped (CQ-024).
import { existsSync, mkdtempSync, rmSync } from "node:fs";
import { tmpdir, userInfo } from "node:os";
import { join } from "node:path";
import { POSTGRES_MAJOR, binaryDirs, findTools, serverOptions, socketDatabaseUrl } from "../src/gates/postgres.ts";
import { REPO_ROOT } from "../src/repository.ts";

const PORT = 5432;
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

const binDir = findTools(binaryDirs(process.env["PATH"] ?? ""), ["initdb", "pg_ctl"], existsSync);
if (binDir === undefined) {
  console.error(
    `with-postgres: initdb and pg_ctl not found. Install PostgreSQL ${POSTGRES_MAJOR} ` +
      `(macOS: brew install postgresql@${POSTGRES_MAJOR}) or set DATABASE_URL to a server you own.`,
  );
  process.exit(1);
}

const root = mkdtempSync(join(tmpdir(), "anima-pg-"));

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
