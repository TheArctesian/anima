import { beforeAll, describe, expect, test } from "bun:test";
import { mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { psql, scalar, serverDirectory, type PsqlResult } from "./postgres.ts";

const ROOT = join(import.meta.dir, "..", "..");
const ROLES_SQL = join(ROOT, "deploy", "postgres", "roles.sql");
const MIGRATIONS = join(ROOT, "crates", "hub", "migrations");

let secretsFolder = "";
let passwordEnv: Record<string, string> = {};
let missingFileRun: PsqlResult & { rolesAfter: string };
let emptyFileRun: PsqlResult & { rolesAfter: string };

const ANIMA_ROLES = "select count(*) from pg_roles where rolname like 'anima\\_%'";

function rolesSqlWith(ownerFile: string, hubFile: string): PsqlResult & { rolesAfter: string } {
  const result = psql(["-f", ROLES_SQL], {
    env: { ANIMA_OWNER_PASSWORD_FILE: ownerFile, ANIMA_HUB_PASSWORD_FILE: hubFile },
  });
  return { ...result, rolesAfter: scalar(ANIMA_ROLES) };
}

function read(folder: string, name: string): string {
  return readFileSync(join(folder, name), "utf8");
}

function password(name: string): string {
  return read(secretsFolder, name);
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
  // Before the roles exist: a missing or an empty password file must make none of them.
  const empty = join(secretsFolder, "empty_password");
  writeFileSync(empty, "");
  missingFileRun = rolesSqlWith(join(secretsFolder, "no_such_file"), passwordEnv["ANIMA_HUB_PASSWORD_FILE"] ?? "");
  emptyFileRun = rolesSqlWith(passwordEnv["ANIMA_OWNER_PASSWORD_FILE"] ?? "", empty);
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
  test("DB-003: a missing password file stops roles.sql and makes no role", () => {
    expect(missingFileRun.exitCode).not.toBe(0);
    expect(missingFileRun.rolesAfter).toBe("0");
  });

  test("DB-003: an empty password file stops roles.sql and makes no role", () => {
    expect(emptyFileRun.exitCode).not.toBe(0);
    expect(emptyFileRun.stderr).toContain("missing or empty");
    expect(emptyFileRun.rolesAfter).toBe("0");
  });

  test("DB-003: the init script refuses an unset, a missing and an empty password file before it runs psql", () => {
    const script = join(ROOT, "deploy", "postgres", "init", "10-anima-roles.sh");
    const run = (env: Record<string, string>) =>
      Bun.spawnSync(["/bin/sh", script], { env: { PATH: process.env["PATH"] ?? "", POSTGRES_USER: "postgres", ...env } });
    const owner = passwordEnv["ANIMA_OWNER_PASSWORD_FILE"] ?? "";
    const hub = passwordEnv["ANIMA_HUB_PASSWORD_FILE"] ?? "";
    const empty = join(secretsFolder, "empty_password");
    const cases: Record<string, string>[] = [
      {},
      { ANIMA_OWNER_PASSWORD_FILE: owner },
      { ANIMA_OWNER_PASSWORD_FILE: owner, ANIMA_HUB_PASSWORD_FILE: join(secretsFolder, "no_such_file") },
      { ANIMA_OWNER_PASSWORD_FILE: empty, ANIMA_HUB_PASSWORD_FILE: hub },
    ];
    for (const env of cases) {
      const result = run(env);
      expect(result.exitCode).not.toBe(0);
      expect(result.stderr.toString()).toMatch(/ANIMA_|missing or empty/);
    }
  });

});

describe("hub login start check (R-023)", () => {
  test("R-023: anima_hub signs in as itself, owns no database, has no server-level role and no SET role", () => {
    const url = new URL(read(secretsFolder, "hub_database_url").replace(/^postgres:/, "http:"));
    expect(url.search).toBe("");
    expect(read(secretsFolder, "owner_database_url")).not.toContain("?");
    const database = url.pathname.slice(1);
    const identity = scalar("select session_user || ',' || current_user", { user: url.username, database });
    expect(identity).toBe("anima_hub,anima_hub");
    expect(scalar("select pg_get_userbyid(datdba) <> 'anima_hub' from pg_database where datname = current_database()", { user: url.username, database })).toBe("t");
    for (const role of ["pg_execute_server_program", "pg_write_server_files", "pg_read_server_files", "pg_maintain"]) {
      expect(scalar(`select pg_has_role('anima_hub', '${role}', 'member')`)).toBe("f");
    }
    expect(scalar("select rolconfig is null from pg_roles where rolname = 'anima_hub'")).toBe("t");
  });

});

describe("roles.sql attributes (R-016)", () => {
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

});

describe("roles.sql database access (DB-003, LOG-006, LOG-007)", () => {
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

});

describe("roles.sql passwords stay out of logs (LOG-007)", () => {
  test("LOG-007: no password reaches the server log even when the client asks to log every statement", () => {
    const serverLog = () => readFileSync(join(serverDirectory(), "server.log"), "utf8");
    const failingScript = join(secretsFolder, "failing-roles.sql");
    writeFileSync(failingScript, readFileSync(ROLES_SQL, "utf8").replace("create role anima_runtime nologin;", ""));
    const env = { ...passwordEnv, PGOPTIONS: "-c log_min_duration_statement=0 -c log_statement=all" };
    expect(psql(["-f", failingScript], { env }).exitCode).not.toBe(0);
    expect(psql(["-f", ROLES_SQL], { env }).exitCode).not.toBe(0);
    for (const name of ["owner_password", "hub_password"]) expect(serverLog()).not.toContain(password(name));
  });

  test("LOG-007: a failing re-run does not print a password and the server log has none", () => {
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
