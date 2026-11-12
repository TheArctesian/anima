import { describe, expect, test } from "bun:test";
import { createHash } from "node:crypto";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import {
  MigrationPlanError,
  applyStatements,
  ledgerRow,
  planMigrations,
  sqlxSessionEnv,
} from "../src/gates/migrations.ts";
import { REPO_ROOT } from "../src/repository.ts";

const text = (name: string, sql: string) => ({ name, bytes: new TextEncoder().encode(sql) });

describe("planMigrations (R-112, sqlx 0.8.6 rules)", () => {
  test("orders by numeric version and names each from the rest of the file name", () => {
    const plan = planMigrations([text("10_b_c.sql", "select 2;"), text("0002_a.sql", "select 1;")]);
    expect(plan.map((migration) => [migration.version, migration.description])).toEqual([
      [2n, "a"],
      [10n, "b c"],
    ]);
  });

  test("checksums the file bytes with SHA-384, unchanged", () => {
    const sql = "-- comment\r\ncreate table t ();\n";
    const [migration] = planMigrations([text("1_t.sql", sql)]);
    expect(migration?.checksumHex).toBe(createHash("sha384").update(sql).digest("hex"));
    expect(migration?.sql).toBe(sql);
  });

  test("keeps a byte order mark in the SQL, as Rust's read_to_string does", () => {
    const [migration] = planMigrations([text("1_t.sql", "﻿select 1;")]);
    expect(migration?.sql.startsWith("﻿")).toBe(true);
  });

  test("ignores files sqlx ignores: no underscore or not .sql", () => {
    const plan = planMigrations([text("README.md", "x"), text("0001.sql", "x"), text("1_a.sql.bak", "x")]);
    expect(plan).toEqual([]);
  });

  test("strips every trailing .sql, as trim_end_matches does", () => {
    expect(planMigrations([text("1_a.sql.sql", "select 1;")])[0]?.description).toBe("a");
  });

  test("refuses what it cannot apply as sqlx would", () => {
    const refused = [
      text("x1_a.sql", "select 1;"),
      text("99999999999999999999_a.sql", "select 1;"),
      text("1_a.sql", "-- no-transaction\ncreate index concurrently i on t (c);"),
      text("1_a.up.sql", "select 1;"),
      text("1_a.down.sql", "select 1;"),
      text("1_a.sql", "\\set x 1"),
      text("1_a.sql", "select '\0';"),
      { name: "1_a.sql", bytes: new Uint8Array([0xff, 0xfe]) },
    ];
    for (const file of refused) expect(() => planMigrations([file])).toThrow(MigrationPlanError);
  });

  test("refuses two files with the same version", () => {
    expect(() => planMigrations([text("1_a.sql", "x"), text("01_b.sql", "y")])).toThrow(/same version/);
  });

  test("plans every migration in the hub's folder", () => {
    const folder = join(REPO_ROOT, "crates", "hub", "migrations");
    const names = readdirSync(folder).filter((name) => name.endsWith(".sql"));
    const plan = planMigrations(names.map((name) => ({ name, bytes: readFileSync(join(folder, name)) })));
    expect(plan).toHaveLength(names.length);
  });
});

describe("applying a migration", () => {
  test("sends the SQL and its _sqlx_migrations row in one transaction", () => {
    const [migration] = planMigrations([text("0300_checks_schema.sql", "create table t ();")]);
    if (migration === undefined) throw new Error("no migration planned");
    expect(applyStatements(migration)).toEqual(["BEGIN", "create table t ();", ledgerRow(migration), "COMMIT"]);
    expect(ledgerRow(migration)).toBe(
      "INSERT INTO _sqlx_migrations ( version, description, success, checksum, execution_time ) " +
        `VALUES ( 300, 'checks schema', TRUE, decode('${migration.checksumHex}', 'hex'), -1 )`,
    );
  });

  test("quotes a description that holds an apostrophe", () => {
    const [migration] = planMigrations([text("1_it's.sql", "select 1;")]);
    if (migration === undefined) throw new Error("no migration planned");
    expect(ledgerRow(migration)).toContain("'it''s'");
  });

  test("uses sqlx's session settings and drops the caller's libpq variables", () => {
    const env = sqlxSessionEnv({ PATH: "/bin", PGHOST: "elsewhere", PGOPTIONS: "-c search_path=x" });
    expect(env).toEqual({
      PATH: "/bin",
      PGDATESTYLE: "ISO, MDY",
      PGCLIENTENCODING: "UTF8",
      PGTZ: "UTC",
      PGOPTIONS: "-c extra_float_digits=2",
    });
  });
});
