// psql against the throwaway server that quality/bin/with-postgres.ts starts (R-009).
// The server trusts every local connection, so no test passes a password.
import { existsSync } from "node:fs";
import { join } from "node:path";
import { binaryDirs, findTools } from "../../quality/src/gates/postgres.ts";

export interface PsqlResult {
  exitCode: number;
  stdout: string;
  stderr: string;
}

interface Connection {
  socketDir: string;
  port: string;
  user: string;
}

function connection(): Connection {
  const url = process.env["DATABASE_URL"];
  if (!url) throw new Error("DATABASE_URL is not set: run these tests with `bun run test`, which starts PostgreSQL");
  const parsed = new URL(url);
  const socketDir = parsed.searchParams.get("host");
  if (!socketDir) throw new Error("DATABASE_URL names no socket directory (host=...)");
  return { socketDir, port: parsed.port || "5432", user: decodeURIComponent(parsed.username) };
}

/** Directory of the throwaway server; it also holds `server.log`. */
export function serverDirectory(): string {
  return connection().socketDir;
}

function psqlBinary(): string {
  const dir = findTools(binaryDirs(process.env["PATH"] ?? ""), ["psql"], existsSync);
  if (dir === undefined) throw new Error("psql not found; install PostgreSQL 17 (CQ-024: these tests never skip)");
  return join(dir, "psql");
}

export interface PsqlOptions {
  database?: string;
  /** Role to connect as; defaults to the server's superuser. */
  user?: string;
  env?: Record<string, string>;
}

/** Runs psql with `args` and never throws on a non-zero exit. */
export function psql(args: string[], options: PsqlOptions = {}): PsqlResult {
  const { socketDir, port, user } = connection();
  const result = Bun.spawnSync([psqlBinary(), "-X", "-v", "ON_ERROR_STOP=1", ...args], {
    env: {
      ...process.env,
      ...options.env,
      PGHOST: socketDir,
      PGPORT: port,
      PGUSER: options.user ?? user,
      PGDATABASE: options.database ?? "postgres",
    },
  });
  return { exitCode: result.exitCode ?? 1, stdout: result.stdout.toString(), stderr: result.stderr.toString() };
}

/** Runs one query and returns its single value as text; throws if psql fails. */
export function scalar(sql: string, options: PsqlOptions = {}): string {
  const result = psql(["-Atq", "-c", sql], options);
  if (result.exitCode !== 0) throw new Error(`psql failed: ${result.stderr}`);
  return result.stdout.trim();
}
