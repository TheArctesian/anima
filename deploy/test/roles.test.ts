import { beforeAll, describe, expect, test } from "bun:test";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { psql, scalar, serverDirectory } from "./postgres.ts";
import { ROLES_SQL, ROOT, installRoles } from "./roles-setup.ts";
import { finish } from "./finished-process.ts";

let secretsFolder = "";
let passwordEnv: Record<string, string> = {};

const ANIMA_ROLES = "select count(*) from pg_roles where rolname like 'anima\\_%'";

function read(folder: string, name: string): string {
  return readFileSync(join(folder, name), "utf8");
}

function password(name: string): string {
  return read(secretsFolder, name);
}

beforeAll(async () => {
  ({ secretsFolder, passwordEnv } = await installRoles());
});

/** Copy of roles.sql without its first statement, so a rerun fails at a CREATE ROLE that carries a password. */
function failingRolesScript(): string {
  const file = join(secretsFolder, "failing-roles.sql");
  writeFileSync(file, readFileSync(ROLES_SQL, "utf8").replace("create role anima_runtime nologin;", ""));
  return file;
}

function roleFlags(role: string, flags: string[]): Promise<string> {
  return scalar(`select ${flags.join(" or ")} from pg_roles where rolname = '${role}'`);
}

describe("roles.sql inputs (DB-003)", () => {
  test.each([
    ["missing", "no_such_file"],
    ["empty", "empty_password"],
  ])("DB-003: a %s password file stops roles.sql and changes no role", async (_kind, file) => {
    const folder = mkdtempSync(join(tmpdir(), "anima-inputs-"));
    try {
      writeFileSync(join(folder, "empty_password"), "");
      writeFileSync(join(folder, "other_password"), "other");
      const before = await scalar(ANIMA_ROLES);
      for (const env of [
        { ANIMA_OWNER_PASSWORD_FILE: join(folder, file), ANIMA_HUB_PASSWORD_FILE: join(folder, "other_password") },
        { ANIMA_OWNER_PASSWORD_FILE: join(folder, "other_password"), ANIMA_HUB_PASSWORD_FILE: join(folder, file) },
      ]) {
        const result = await psql(["-f", ROLES_SQL], { env });
        expect(result.exitCode).not.toBe(0);
        expect(result.stderr).toContain("missing or empty");
        expect(await scalar(ANIMA_ROLES)).toBe(before);
      }
    } finally {
      rmSync(folder, { recursive: true, force: true });
    }
  });

  test("DB-003: the init script refuses an unset, a missing and an empty password file before it runs psql", async () => {
    const script = join(ROOT, "deploy", "postgres", "init", "10-anima-roles.sh");
    const run = (env: Record<string, string>) =>
      finish(["/bin/sh", script], { env: { PATH: process.env["PATH"] ?? "", POSTGRES_USER: "postgres", ...env } });
    const owner = passwordEnv["ANIMA_OWNER_PASSWORD_FILE"] ?? "";
    const hub = passwordEnv["ANIMA_HUB_PASSWORD_FILE"] ?? "";
    const empty = join(secretsFolder, "empty_password");
    writeFileSync(empty, "");
    const cases: Record<string, string>[] = [
      {},
      { ANIMA_OWNER_PASSWORD_FILE: owner },
      { ANIMA_OWNER_PASSWORD_FILE: owner, ANIMA_HUB_PASSWORD_FILE: join(secretsFolder, "no_such_file") },
      { ANIMA_OWNER_PASSWORD_FILE: empty, ANIMA_HUB_PASSWORD_FILE: hub },
    ];
    for (const env of cases) {
      const result = await run(env);
      expect(result.exitCode).not.toBe(0);
      expect(result.stderr).toMatch(/ANIMA_|missing or empty/);
    }
  });

});

describe("hub login start check (DB-003)", () => {
  test("DB-003: anima_hub signs in as itself, owns no database, has no server-level role and no SET role", async () => {
    const url = new URL(read(secretsFolder, "hub_database_url").replace(/^postgres:/, "http:"));
    expect(url.search).toBe("");
    expect(read(secretsFolder, "owner_database_url")).not.toContain("?");
    const database = url.pathname.slice(1);
    const identity = await scalar("select session_user || ',' || current_user", { user: url.username, database });
    expect(identity).toBe("anima_hub,anima_hub");
    expect(await scalar("select pg_get_userbyid(datdba) <> 'anima_hub' from pg_database where datname = current_database()", { user: url.username, database })).toBe("t");
    for (const role of ["pg_execute_server_program", "pg_write_server_files", "pg_read_server_files", "pg_maintain"]) {
      expect(await scalar(`select pg_has_role('anima_hub', '${role}', 'member')`)).toBe("f");
    }
    expect(await scalar("select rolconfig is null from pg_roles where rolname = 'anima_hub'")).toBe("t");
  });

});

describe("roles.sql attributes (DB-003)", () => {
  test("DB-003: anima_owner can log in but is not superuser, createdb, createrole, bypassrls or replication", async () => {
    expect(await roleFlags("anima_owner", ["rolcanlogin"])).toBe("t");
    expect(await roleFlags("anima_owner", ["rolsuper", "rolcreatedb", "rolcreaterole", "rolbypassrls", "rolreplication"])).toBe("f");
  });

  test("DB-003: anima_hub can log in, has no elevated rights and belongs to anima_runtime only", async () => {
    expect(await roleFlags("anima_hub", ["rolcanlogin"])).toBe("t");
    expect(await roleFlags("anima_hub", ["rolsuper", "rolcreatedb", "rolcreaterole", "rolbypassrls", "rolreplication"])).toBe("f");
    const groups = await scalar(
      `select string_agg(g.rolname, ',' order by g.rolname) from pg_auth_members m
         join pg_roles g on g.oid = m.roleid join pg_roles u on u.oid = m.member
         where u.rolname = 'anima_hub'`,
    );
    expect(groups).toBe("anima_runtime");
  });

  test("DB-003: anima_runtime is a plain group that cannot log in", async () => {
    expect(await roleFlags("anima_runtime", ["rolcanlogin"])).toBe("f");
    expect(await roleFlags("anima_runtime", ["rolsuper", "rolcreatedb", "rolcreaterole", "rolbypassrls"])).toBe("f");
  });

});

describe("roles.sql database access (DB-003, LOG-006, LOG-007)", () => {
  test("DB-003: anima_owner owns the database anima", async () => {
    expect(await scalar("select pg_get_userbyid(datdba) from pg_database where datname = 'anima'")).toBe("anima_owner");
  });

  test("DB-003: anima_hub connects to anima and not to postgres or template1", async () => {
    expect(await scalar("select has_database_privilege('anima_hub', 'anima', 'connect')")).toBe("t");
    expect(await scalar("select has_database_privilege('anima_hub', 'postgres', 'connect')")).toBe("f");
    expect(await scalar("select has_database_privilege('anima_hub', 'template1', 'connect')")).toBe("f");
    expect(await scalar("select has_database_privilege('anima_hub', 'anima', 'temporary')")).toBe("f");
    expect((await psql(["-c", "select 1"], { user: "anima_hub", database: "anima" })).exitCode).toBe(0);
  });

  test("DB-003: anima_hub owns nothing after the migrations ran", async () => {
    const owned = `select count(*) from pg_shdepend d join pg_roles r on r.oid = d.refobjid
      where r.rolname = 'anima_hub' and d.deptype = 'o'`;
    expect(await scalar(owned, { database: "anima" })).toBe("0");
    expect(await scalar(owned)).toBe("0");
  });

  test("LOG-006: anima_hub can read and append the audit log but not change or delete it", async () => {
    const can = (privilege: string) =>
      scalar(`select has_table_privilege('anima_hub', 'events.audit_log', '${privilege}')`, { database: "anima" });
    expect(await can("select")).toBe("t");
    expect(await can("insert")).toBe("t");
    expect(await can("update")).toBe("f");
    expect(await can("delete")).toBe("f");
  });

});

describe("roles.sql passwords stay out of logs (LOG-007)", () => {
  const serverLog = () => readFileSync(join(serverDirectory(), "server.log"), "utf8");

  test("LOG-007: no password reaches the server log even when the client asks to log every statement", async () => {
    const options = [
      "-c log_min_duration_statement=0",
      "-c log_statement=all",
      "-c log_min_duration_sample=0",
      "-c log_statement_sample_rate=1",
      "-c log_transaction_sample_rate=1",
    ];
    const env = { ...passwordEnv, PGOPTIONS: options.join(" ") };
    expect((await psql(["-f", failingRolesScript()], { env })).exitCode).not.toBe(0);
    for (const name of ["owner_password", "hub_password"]) expect(serverLog()).not.toContain(password(name));
  });

  test("LOG-007: a failing re-run does not print a password and the server log has none", async () => {
    const rerun = await psql(["-f", failingRolesScript()], { env: passwordEnv });
    expect(rerun.exitCode).not.toBe(0);
    expect(rerun.stderr).toContain("anima_owner");
    for (const name of ["owner_password", "hub_password"]) {
      expect(rerun.stdout + rerun.stderr).not.toContain(password(name));
      expect(serverLog()).not.toContain(password(name));
    }
  });
});
