import { join } from "node:path";

/** PostgreSQL major version the hub targets (spec DB-001). */
export const POSTGRES_MAJOR = 17;

/** Directories searched for `initdb` and `pg_ctl` after `PATH`, most specific first. */
export function binaryDirs(pathEnv: string, major: number = POSTGRES_MAJOR): string[] {
  const fromPath = pathEnv.split(":").filter((dir) => dir !== "");
  return [
    `/opt/homebrew/opt/postgresql@${major}/bin`,
    `/usr/local/opt/postgresql@${major}/bin`,
    `/usr/lib/postgresql/${major}/bin`,
    ...fromPath,
  ];
}

/** First directory that holds every named tool, or undefined. */
export function findTools(dirs: string[], tools: string[], exists: (path: string) => boolean): string | undefined {
  return dirs.find((dir) => tools.every((tool) => exists(join(dir, tool))));
}

/**
 * Connection URL for a server that listens only on a Unix socket in `socketDir`.
 * `sqlx::test` connects to this server and creates one throwaway database per test.
 */
export function socketDatabaseUrl(socketDir: string, port: number, user: string): string {
  return `postgres://${encodeURIComponent(user)}@localhost:${port}/postgres?host=${encodeURIComponent(socketDir)}`;
}

/** `pg_ctl -o` options: no TCP listener, socket in `socketDir`, fsync off (throwaway data). */
export function serverOptions(socketDir: string, port: number): string {
  return `-c listen_addresses='' -c unix_socket_directories='${socketDir}' -c port=${port} -c fsync=off -c max_connections=200`;
}

/** A server that listens only on a Unix socket in `socketDir`. */
export interface SocketServer {
  socketDir: string;
  port: number;
  user: string;
}

/**
 * `psql` argv that sends each statement to `database` as one simple query, stopping at the first error.
 * `-c` text goes to the server as given: psql neither expands `:variables` nor reads meta-commands in it,
 * unless the text starts with a backslash. `-X` skips `~/.psqlrc`.
 */
export function psqlArgv(psql: string, server: SocketServer, database: string, statements: string[]): string[] {
  const connection = ["-h", server.socketDir, "-p", String(server.port), "-U", server.user, "-d", database];
  return [psql, "-X", "-q", "-v", "ON_ERROR_STOP=1", ...connection, ...statements.flatMap((statement) => ["-c", statement])];
}

/** Asks `with-postgres` to migrate `template1` with the hub's migrations before the command runs. */
export const MIGRATED_TEMPLATE = "--migrated-template";

/** `with-postgres` arguments: an optional leading `--migrated-template`, then the command. */
export function withPostgresArgs(args: string[]): { migratedTemplate: boolean; command: string[] } {
  const migratedTemplate = args[0] === MIGRATED_TEMPLATE;
  return { migratedTemplate, command: migratedTemplate ? args.slice(1) : args };
}
