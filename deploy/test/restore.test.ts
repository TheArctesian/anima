import { afterAll, beforeEach, describe, expect, test } from "bun:test";
import { writeFileSync } from "node:fs";
import { scalar } from "./postgres.ts";
import {
  DATABASE, RESTORE, backUp, backUpWithMissingRole, count, databasesLike, dockerShim, owner, resetSample, run,
  tearDownSample,
} from "./backup-fixture.ts";

beforeEach(resetSample);
afterAll(tearDownSample);

const TABLES = [
  ["devices.sites", "1"],
  ["devices.devices", "2"],
  ["devices.device_tags", "1"],
  ["events.audit_log", "2"],
  ["events.event_log", "1"],
] as const;

function expectLiveDatabaseIntact(): void {
  for (const [table, rows] of TABLES) expect(count(table)).toBe(rows);
  expect(databasesLike(`${DATABASE}_restoring`)).toEqual([]);
  expect(databasesLike(`${DATABASE}_previous`)).toEqual([]);
}

describe("restore.sh arguments (DB-007)", () => {
  test("DB-007: restore refuses without --yes and changes nothing", () => {
    const path = backUp();
    owner("delete from devices.device_tags");
    expect(run(RESTORE, [path]).exitCode).not.toBe(0);
    expect(count("devices.device_tags")).toBe("0");
  });

  test.each([
    ["an unknown option", "--force"],
    ["--check", "--check"],
    ["a second dump", "SECOND"],
  ])("DB-007: restore --yes refuses %s and changes nothing", (_name, extra) => {
    const path = backUp();
    const result = run(RESTORE, ["--yes", path, extra === "SECOND" ? path : extra]);
    expect(result.exitCode).not.toBe(0);
    expectLiveDatabaseIntact();
  });
});

describe("restore.sh --yes (DB-007)", () => {
  test("DB-007: a full restore brings back every row, keeps the old database and prints how to drop it", () => {
    const path = backUp();
    owner("delete from devices.devices where name = 'mac'");
    owner("delete from devices.device_tags");
    const result = run(RESTORE, ["--yes", path]);
    expect(result.exitCode).toBe(0);
    expect(result.stdout).toContain("migrate");
    for (const [table, rows] of TABLES) expect(count(table)).toBe(rows);
    const [previous] = databasesLike(`${DATABASE}_previous`);
    expect(previous).toBeDefined();
    expect(result.stdout).toContain(`drop database "${previous ?? ""}"`);
    expect(scalar("select count(*) from devices.devices", { user: "anima_owner", database: previous ?? "" })).toBe("1");
    expect(databasesLike(`${DATABASE}_restoring`)).toEqual([]);
  });

  test("DB-007: the restored database keeps its audit log privileges and later tables reach the runtime group", () => {
    expect(run(RESTORE, ["--yes", backUp()]).exitCode).toBe(0);
    const can = (privilege: string) =>
      scalar(`select has_table_privilege('anima_hub', 'events.audit_log', '${privilege}')`, { database: DATABASE });
    expect(["select", "insert", "update", "delete", "truncate"].map(can)).toEqual(["t", "t", "f", "f", "f"]);
    owner("create table devices.after_restore (id int)");
    expect(scalar("select has_table_privilege('anima_hub', 'devices.after_restore', 'insert')", { database: DATABASE })).toBe("t");
  });

  test("DB-007: owner and grants of a restored database equal those roles.sql gave anima", () => {
    expect(run(RESTORE, ["--yes", backUp()]).exitCode).toBe(0);
    const facts = (name: string) =>
      scalar(
        `select pg_get_userbyid(datdba) || ' ' || (select string_agg(a::text, ',' order by a::text) from unnest(datacl) a)
           from pg_database where datname = '${name}'`,
      );
    expect(facts(DATABASE)).toBe(facts("anima"));
    expect(facts("anima")).toContain("anima_runtime=c/");
  });
});

describe("restore.sh failures leave the live database alone (DB-007)", () => {
  test.each([
    ["garbage", (path: string) => writeFileSync(path, "this is not a dump")],
    ["truncated", (path: string) => Bun.spawnSync(["truncate", "-s", "9000", path])],
  ])("DB-007: a %s dump leaves the database with the same rows", (_name, damage) => {
    const path = backUp();
    damage(path);
    expect(run(RESTORE, ["--yes", path]).exitCode).not.toBe(0);
    expectLiveDatabaseIntact();
  });

  test("DB-007: a restore that fails halfway leaves the database with the same rows and no new database", () => {
    const path = backUpWithMissingRole();
    const result = run(RESTORE, ["--yes", path]);
    expect(result.exitCode).not.toBe(0);
    expect(result.stderr).toContain("pg_restore failed");
    expectLiveDatabaseIntact();
  });

  test("DB-007: while the hub is listed as running, restore refuses and drops nothing", () => {
    const path = backUp();
    const result = run(RESTORE, ["--yes", path], { ANIMA_PG_LOCAL: "0", FAKE_RUNNING: "hub" }, dockerShim());
    expect(result.exitCode).not.toBe(0);
    expect(result.stderr).toContain("hub");
    expectLiveDatabaseIntact();
  });

  test("DB-007: with the services stopped, restore works through the docker path", () => {
    const path = backUp();
    owner("delete from devices.device_tags");
    const result = run(RESTORE, ["--yes", path], { ANIMA_PG_LOCAL: "0", FAKE_RUNNING: "" }, dockerShim());
    expect(result.stderr).toBe("");
    expect(result.exitCode).toBe(0);
    expect(count("devices.device_tags")).toBe("1");
  });
});
