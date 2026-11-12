import { afterAll, beforeEach, describe, expect, test } from "bun:test";
import { readFileSync, truncateSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { scalar } from "./postgres.ts";
import {
  DATABASE, RESTORE, backUp, backUpWithMissingRole, count, databasesLike, dockerShim, owner, resetSample, run,
  state, tearDownSample,
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

async function expectLiveDatabaseIntact(): Promise<void> {
  for (const [table, rows] of TABLES) expect(await count(table)).toBe(rows);
  expect(await databasesLike(`${DATABASE}_restoring`)).toEqual([]);
  expect(await databasesLike(`${DATABASE}_previous`)).toEqual([]);
}

describe("restore.sh arguments (DB-007)", () => {
  test("DB-007: restore refuses without --yes and changes nothing", async () => {
    const path = await backUp();
    await owner("delete from devices.device_tags");
    expect((await run(RESTORE, [path])).exitCode).not.toBe(0);
    expect(await count("devices.device_tags")).toBe("0");
  });

  test.each([
    ["an unknown option", "--force"],
    ["--check", "--check"],
    ["a second dump", "SECOND"],
  ])("DB-007: restore --yes refuses %s and changes nothing", async (_name, extra) => {
    const path = await backUp();
    const result = await run(RESTORE, ["--yes", path, extra === "SECOND" ? path : extra]);
    expect(result.exitCode).not.toBe(0);
    await expectLiveDatabaseIntact();
  });
});

describe("restore.sh --yes (DB-007)", () => {
  test("DB-007: a full restore brings back every row, keeps the old database and prints how to drop it", async () => {
    const path = await backUp();
    await owner("delete from devices.devices where name = 'mac'");
    await owner("delete from devices.device_tags");
    const result = await run(RESTORE, ["--yes", path]);
    expect(result.exitCode).toBe(0);
    expect(result.stdout).toContain("migrate");
    for (const [table, rows] of TABLES) expect(await count(table)).toBe(rows);
    const [previous] = await databasesLike(`${DATABASE}_previous`);
    expect(previous).toBeDefined();
    expect(result.stdout).toContain(`drop database "${previous ?? ""}"`);
    expect(await scalar("select count(*) from devices.devices", { user: "anima_owner", database: previous ?? "" })).toBe("1");
    expect(await databasesLike(`${DATABASE}_restoring`)).toEqual([]);
  });

  test("DB-007: the restored database keeps its audit log privileges and later tables reach the runtime group", async () => {
    expect((await run(RESTORE, ["--yes", await backUp()])).exitCode).toBe(0);
    const can = (privilege: string) =>
      scalar(`select has_table_privilege('anima_hub', 'events.audit_log', '${privilege}')`, { database: DATABASE });
    expect(await Promise.all(["select", "insert", "update", "delete", "truncate"].map(can))).toEqual(["t", "t", "f", "f", "f"]);
    await owner("create table devices.after_restore (id int)");
    expect(await scalar("select has_table_privilege('anima_hub', 'devices.after_restore', 'insert')", { database: DATABASE })).toBe("t");
  });

  test("DB-007: owner and grants of a restored database equal those roles.sql gave anima", async () => {
    expect((await run(RESTORE, ["--yes", await backUp()])).exitCode).toBe(0);
    const facts = (name: string) =>
      scalar(
        `select pg_get_userbyid(datdba) || ' ' || (select string_agg(a::text, ',' order by a::text) from unnest(datacl) a)
           from pg_database where datname = '${name}'`,
      );
    expect(await facts(DATABASE)).toBe(await facts("anima"));
    expect(await facts("anima")).toContain("anima_runtime=c/");
  });
});

describe("restore.sh failures leave the live database alone (DB-007)", () => {
  test.each([
    ["garbage", (path: string) => writeFileSync(path, "this is not a dump")],
    ["truncated", (path: string) => truncateSync(path, 9000)],
  ])("DB-007: a %s dump leaves the database with the same rows", async (_name, damage) => {
    const path = await backUp();
    damage(path);
    expect((await run(RESTORE, ["--yes", path])).exitCode).not.toBe(0);
    await expectLiveDatabaseIntact();
  });

  test("DB-007: a restore that fails halfway leaves the database with the same rows and no new database", async () => {
    const path = await backUpWithMissingRole();
    const result = await run(RESTORE, ["--yes", path]);
    expect(result.exitCode).not.toBe(0);
    expect(result.stderr).toContain("pg_restore failed");
    await expectLiveDatabaseIntact();
  });

  test("DB-007: while the hub is listed as running, restore refuses and drops nothing", async () => {
    const path = await backUp();
    const result = await run(RESTORE, ["--yes", path], { ANIMA_PG_LOCAL: "0", FAKE_RUNNING: "hub", FAKE_CALLS: join(state.backupDir, "docker-calls") }, dockerShim());
    expect(result.exitCode).not.toBe(0);
    expect(result.stderr).toContain("hub");
    await expectLiveDatabaseIntact();
  });

  test("DB-007: a service that starts during the restore stops the swap and the new database goes", async () => {
    const path = await backUp();
    const calls = join(state.backupDir, "docker-calls");
    const env = { ANIMA_PG_LOCAL: "0", FAKE_RUNNING: "hub", FAKE_RUNNING_FROM: "2", FAKE_CALLS: calls };
    const result = await run(RESTORE, ["--yes", path], env, dockerShim());
    expect(result.exitCode).not.toBe(0);
    expect(result.stderr).toContain("stop these services first");
    expect(readFileSync(calls, "utf8").trim().split("\n")).toHaveLength(2);
    await expectLiveDatabaseIntact();
  });

  test("DB-007: the service check also asks for restarting services", async () => {
    const calls = join(state.backupDir, "docker-calls");
    await run(RESTORE, ["--yes", await backUp()], { ANIMA_PG_LOCAL: "0", FAKE_CALLS: calls }, dockerShim());
    expect(readFileSync(calls, "utf8")).toContain("--status restarting");
  });

  test("DB-007: with the services stopped, restore works through the docker path", async () => {
    const path = await backUp();
    await owner("delete from devices.device_tags");
    const result = await run(RESTORE, ["--yes", path], { ANIMA_PG_LOCAL: "0", FAKE_RUNNING: "", FAKE_CALLS: join(state.backupDir, "docker-calls") }, dockerShim());
    expect(result.stderr).toBe("");
    expect(result.exitCode).toBe(0);
    expect(await count("devices.device_tags")).toBe("1");
  });
});
