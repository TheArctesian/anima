import { beforeAll, describe, expect, test } from "bun:test";
import { mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { psql, scalar, serverDirectory } from "./postgres.ts";

const ROOT = join(import.meta.dir, "..", "..");
const ROLES_SQL = join(ROOT, "deploy", "postgres", "roles.sql");
const MIGRATIONS = join(ROOT, "crates", "hub", "migrations");

let secretsFolder = "";
let passwordEnv: Record<string, string> = {};

function password(name: string): string {
  return readFileSync(join(secretsFolder, name), "utf8");
}

function runRolesSql() {
  return psql(["-f", ROLES_SQL], { env: passwordEnv });
}

// Roles are cluster-wide and the server is shared by this file, so they are made once.
beforeAll(() => {
  secretsFolder = mkdtempSync(join(tmpdir(), "anima-roles-"));
  const made = Bun.spawnSync(["sh", join(ROOT, "deploy", "bin", "make-secrets"), secretsFolder], {
    env: { ...process.env, ANIMA_SECRETS_CHOWN: "0" },
    stdin: "ignore",
  });
  if (made.exitCode !== 0) throw new Error(`make-secrets failed: ${made.stderr.toString()}`);
  passwordEnv = {
    ANIMA_OWNER_PASSWORD_FILE: join(secretsFolder, "owner_password"),
    ANIMA_HUB_PASSWORD_FILE: join(secretsFolder, "hub_password"),
  };
  const result = runRolesSql();
  if (result.exitCode !== 0) throw new Error(`roles.sql failed: ${result.stderr}`);
  for (const file of readdirSync(MIGRATIONS).filter((name) => name.endsWith(".sql")).sort()) {
    const applied = psql(["-f", join(MIGRATIONS, file)], { user: "anima_owner", database: "anima" });
    if (applied.exitCode !== 0) throw new Error(`migration ${file} failed: ${applied.stderr}`);
  }
  return () => rmSync(secretsFolder, { recursive: true, force: true });
});

function roleFlags(role: string, flags: string[]): string {
  return scalar(`select ${flags.join(" or ")} from pg_roles where rolname = '${role}'`);
}

describe("roles.sql (DB-003, R-016)", () => {
  test("R-016: anima_owner can log in but is not superuser, createdb, createrole, bypassrls or replication", () => {
    expect(roleFlags("anima_owner", ["rolcanlogin"])).toBe("t");
    expect(roleFlags("anima_owner", ["rolsuper", "rolcreatedb", "rolcreaterole", "rolbypassrls", "rolreplication"])).toBe("f");
  });

  test("R-016: anima_hub can log in, has no elevated rights and belongs to anima_runtime only", () => {
    expect(roleFlags("anima_hub", ["rolcanlogin"])).toBe("t");
    expect(roleFlags("anima_hub", ["rolsuper", "rolcreatedb", "rolcreaterole", "rolbypassrls", "rolreplication"])).toBe("f");
    const groups = scalar(
      `select string_agg(g.rolname, ',' order by g.rolname) from pg_auth_members m
         join pg_roles g on g.oid = m.roleid join pg_roles u on u.oid = m.member
         where u.rolname = 'anima_hub'`,
    );
    expect(groups).toBe("anima_runtime");
  });

  test("R-016: anima_runtime is a plain group that cannot log in", () => {
    expect(roleFlags("anima_runtime", ["rolcanlogin"])).toBe("f");
    expect(roleFlags("anima_runtime", ["rolsuper", "rolcreatedb", "rolcreaterole", "rolbypassrls"])).toBe("f");
  });

  test("DB-003: anima_owner owns the database anima", () => {
    expect(scalar("select pg_get_userbyid(datdba) from pg_database where datname = 'anima'")).toBe("anima_owner");
  });

  test("DB-003: anima_hub connects to anima and not to postgres or template1", () => {
    expect(scalar("select has_database_privilege('anima_hub', 'anima', 'connect')")).toBe("t");
    expect(scalar("select has_database_privilege('anima_hub', 'postgres', 'connect')")).toBe("f");
    expect(scalar("select has_database_privilege('anima_hub', 'template1', 'connect')")).toBe("f");
    expect(scalar("select has_database_privilege('anima_hub', 'anima', 'temporary')")).toBe("f");
    expect(psql(["-c", "select 1"], { user: "anima_hub", database: "anima" }).exitCode).toBe(0);
  });

  test("R-016: anima_hub owns nothing after the migrations ran", () => {
    const owned = `select count(*) from pg_shdepend d join pg_roles r on r.oid = d.refobjid
      where r.rolname = 'anima_hub' and d.deptype = 'o'`;
    expect(scalar(owned, { database: "anima" })).toBe("0");
    expect(scalar(owned)).toBe("0");
  });

  test("LOG-006: anima_hub can read and append the audit log but not change or delete it", () => {
    const can = (privilege: string) =>
      scalar(`select has_table_privilege('anima_hub', 'events.audit_log', '${privilege}')`, { database: "anima" });
    expect(can("select")).toBe("t");
    expect(can("insert")).toBe("t");
    expect(can("update")).toBe("f");
    expect(can("delete")).toBe("f");
  });

  test("LOG-007: no password reaches the server log, and a failing re-run does not print one", () => {
    // Skip the first statement so the failure falls on the CREATE ROLE that carries a password.
    const failingScript = join(secretsFolder, "failing-roles.sql");
    writeFileSync(failingScript, readFileSync(ROLES_SQL, "utf8").replace("create role anima_runtime nologin;", ""));
    const rerun = psql(["-f", failingScript], { env: passwordEnv });
    expect(rerun.exitCode).not.toBe(0);
    expect(rerun.stderr).toContain("anima_owner");
    const serverLog = readFileSync(join(serverDirectory(), "server.log"), "utf8");
    for (const name of ["owner_password", "hub_password"]) {
      expect(rerun.stdout + rerun.stderr).not.toContain(password(name));
      expect(serverLog).not.toContain(password(name));
    }
  });
});
