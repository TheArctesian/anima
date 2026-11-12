// The hub's migrations as sqlx 0.8.6 reads them (sqlx-core `migrate/source.rs`, `migrate/migration.rs`),
// so `with-postgres` can migrate `template1` once and every `#[sqlx::test]` database starts as a copy
// whose `_sqlx_migrations` rows sqlx accepts as already applied (WP-55, R-112).
import { createHash } from "node:crypto";

/** One migration file, as read from the migrations folder. */
export interface MigrationFile {
  name: string;
  bytes: Uint8Array;
}

/** One migration to apply: its SQL text, sent verbatim, and the `_sqlx_migrations` row sqlx expects. */
export interface Migration {
  file: string;
  version: bigint;
  description: string;
  sql: string;
  checksumHex: string;
}

/** A migration file this plan cannot apply exactly as sqlx would; the template is not built. */
export class MigrationPlanError extends Error {
  constructor(file: string, reason: string) {
    super(`${file}: ${reason}`);
    this.name = "MigrationPlanError";
  }
}

/** sqlx's migrations table, created exactly as `ensure_migrations_table` creates it. */
export const LEDGER_DDL = `CREATE TABLE IF NOT EXISTS _sqlx_migrations (
    version BIGINT PRIMARY KEY,
    description TEXT NOT NULL,
    installed_on TIMESTAMPTZ NOT NULL DEFAULT now(),
    success BOOLEAN NOT NULL,
    checksum BYTEA NOT NULL,
    execution_time BIGINT NOT NULL
);`;

/** Session settings sqlx sends at connect (`connection/establish.rs`), as libpq environment variables. */
export const SQLX_SESSION_ENV = {
  PGDATESTYLE: "ISO, MDY",
  PGCLIENTENCODING: "UTF8",
  PGTZ: "UTC",
  PGOPTIONS: "-c extra_float_digits=2",
} as const;

const I64_MIN = -(2n ** 63n);
const I64_MAX = 2n ** 63n - 1n;

/** Rust's `i64::from_str`: an optional sign, then ASCII digits, in range. */
function parseVersion(file: string, text: string): bigint {
  if (!/^[+-]?\d+$/.test(text)) {
    throw new MigrationPlanError(file, "expected an integer version prefix, as sqlx requires");
  }
  const version = BigInt(text);
  if (version < I64_MIN || version > I64_MAX) throw new MigrationPlanError(file, "version is out of the i64 range");
  return version;
}

/** Rust's `str::trim_end_matches(".sql")`: removes the suffix as many times as it repeats. */
function withoutSqlSuffixes(rest: string): string {
  let trimmed = rest;
  while (trimmed.endsWith(".sql")) trimmed = trimmed.slice(0, -".sql".length);
  return trimmed;
}

function readSql(file: MigrationFile): string {
  let sql: string;
  try {
    sql = new TextDecoder("utf-8", { fatal: true, ignoreBOM: true }).decode(file.bytes);
  } catch {
    throw new MigrationPlanError(file.name, "is not valid UTF-8");
  }
  if (sql.includes("\0")) throw new MigrationPlanError(file.name, "contains a NUL byte");
  if (sql.startsWith("-- no-transaction")) {
    throw new MigrationPlanError(file.name, "is a `-- no-transaction` migration; with-postgres applies migrations in a transaction only");
  }
  // `psql -c` runs a string that starts with a backslash as a meta-command, not as SQL.
  if (sql.startsWith("\\")) throw new MigrationPlanError(file.name, "starts with a backslash, which psql -c does not send verbatim");
  return sql;
}

/** The migration a file names, or undefined for a file sqlx ignores (no `_`, or not `.sql`). */
function toMigration(file: MigrationFile): Migration | undefined {
  const split = file.name.indexOf("_");
  if (split === -1) return undefined;
  const rest = file.name.slice(split + 1);
  if (!rest.endsWith(".sql")) return undefined;
  const version = parseVersion(file.name, file.name.slice(0, split));
  if (rest.endsWith(".up.sql") || rest.endsWith(".down.sql")) {
    throw new MigrationPlanError(file.name, "is a reversible migration; with-postgres applies simple migrations only");
  }
  const sql = readSql(file);
  const checksumHex = createHash("sha384").update(file.bytes).digest("hex");
  return { file: file.name, version, description: withoutSqlSuffixes(rest).replaceAll("_", " "), sql, checksumHex };
}

/** The migrations sqlx would apply to a new database, in the order it applies them. */
export function planMigrations(files: MigrationFile[]): Migration[] {
  const migrations = files.map(toMigration).filter((migration) => migration !== undefined);
  migrations.sort((a, b) => (a.version < b.version ? -1 : a.version > b.version ? 1 : 0));
  for (const [index, migration] of migrations.entries()) {
    const previous = migrations[index - 1];
    if (previous?.version === migration.version) {
      throw new MigrationPlanError(migration.file, `has the same version as ${previous.file}`);
    }
  }
  return migrations;
}

/** The `_sqlx_migrations` row sqlx inserts with the migration, in the same transaction. */
export function ledgerRow(migration: Migration): string {
  const description = migration.description.replaceAll("'", "''");
  return (
    "INSERT INTO _sqlx_migrations ( version, description, success, checksum, execution_time ) " +
    `VALUES ( ${migration.version.toString()}, '${description}', TRUE, decode('${migration.checksumHex}', 'hex'), -1 )`
  );
}

/** The statements that apply one migration as sqlx does: the SQL and its row, in one transaction. */
export function applyStatements(migration: Migration): string[] {
  return ["BEGIN", migration.sql, ledgerRow(migration), "COMMIT"];
}

/** `env` without libpq's `PG*` variables, plus the session settings sqlx uses. */
export function sqlxSessionEnv(env: Record<string, string | undefined>): Record<string, string | undefined> {
  const kept = Object.entries(env).filter(([name]) => !name.startsWith("PG"));
  return { ...Object.fromEntries(kept), ...SQLX_SESSION_ENV };
}
