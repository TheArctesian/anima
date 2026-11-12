import { afterAll, beforeEach, describe, expect, test } from "bun:test";
import { readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { finish } from "./finished-process.ts";
import { serverDirectory } from "./postgres.ts";
import {
  RESTORE_CHECK, appendLogs, backUp, backUpWithMissingRole, createDatabase, databasesLike, owner, resetSample,
  run, shimDirectory, state, tearDownSample, toolDirectory,
} from "./backup-fixture.ts";

beforeEach(resetSample);
afterAll(tearDownSample);

const noCheckDatabase = async () => expect(await databasesLike("anima_restore_check")).toEqual([]);

describe("restore-check.sh against the manifest (DB-007)", () => {
  test("DB-007: a fresh dump passes and leaves no check database", async () => {
    const result = await run(RESTORE_CHECK, [await backUp()]);
    expect(result.stderr).toBe("");
    expect(result.exitCode).toBe(0);
    await noCheckDatabase();
  });

  test("DB-007: writes, deletes and a new day partition in the live database after the dump do not matter", async () => {
    const path = await backUp();
    await appendLogs();
    await owner("delete from devices.device_tags");
    await owner("select events.ensure_day_partition('audit_log', now() + interval '3 days')");
    expect((await run(RESTORE_CHECK, [path])).exitCode).toBe(0);
  });

  test("DB-007: an edited manifest fails and names the table", async () => {
    const path = await backUp();
    const manifest = `${path}.counts`;
    writeFileSync(manifest, readFileSync(manifest, "utf8").replace("devices.devices 2", "devices.devices 3"));
    const result = await run(RESTORE_CHECK, [path]);
    expect(result.exitCode).toBe(1);
    expect(result.stderr).toContain("devices.devices");
    await noCheckDatabase();
  });

  test("DB-007: a missing manifest is a clear failure", async () => {
    const path = await backUp();
    rmSync(`${path}.counts`);
    const result = await run(RESTORE_CHECK, [path]);
    expect(result.exitCode).toBe(1);
    expect(result.stderr).toContain("restore-check.sh: the manifest");
    expect(result.stderr).toContain("is missing");
  });

  test("DB-007: a check database that cannot be dropped is a warning on stderr", async () => {
    const real = join(toolDirectory(), "psql");
    const shim = shimDirectory("psql", `case "$*" in *"drop database"*) exit 1 ;; esac\nexec "${real}" "$@"`);
    const result = await run(RESTORE_CHECK, [await backUp()], {}, shim);
    expect(result.stderr).toContain("warning: could not drop database anima_restore_check");
  });

});

describe("restore-check.sh failures (DB-007)", () => {
  test("DB-007: the output never holds row contents", async () => {
    const path = await backUp();
    await owner("select 1");
    writeFileSync(`${path}.counts`, "devices.devices 9\n");
    const result = await run(RESTORE_CHECK, [path]);
    expect(result.stdout + result.stderr).not.toMatch(/storage|nas\.lan|device\.create/);
  });

  test("DB-007: a dump with no tables fails even when its manifest is empty", async () => {
    await createDatabase("anima_backup_test_empty");
    const path = join(state.backupDir, "anima-20000101T000000Z.dump");
    const dumped = await finish([join(toolDirectory(), "pg_dump"), "-Fc", "-f", path, "-d", "anima_backup_test_empty"], {
      env: { ...process.env, PGHOST: serverDirectory(), PGUSER: "anima_owner" },
    });
    expect(dumped.exitCode).toBe(0);
    writeFileSync(`${path}.counts`, "");
    const result = await run(RESTORE_CHECK, [path]);
    expect(result.exitCode).toBe(1);
    expect(result.stderr).toContain("no tables");
  });

  test("DB-007: a restore that fails still drops the check database", async () => {
    const result = await run(RESTORE_CHECK, [await backUpWithMissingRole()]);
    expect(result.exitCode).not.toBe(0);
    await noCheckDatabase();
  });
});

describe("restore-check.sh arguments (DB-007)", () => {
  test.each([["--yes"], ["--force"]])("DB-007: %s is refused", async (option) => {
    expect((await run(RESTORE_CHECK, [option, await backUp()])).exitCode).not.toBe(0);
    await noCheckDatabase();
  });

  test("DB-007: a second dump is refused", async () => {
    const path = await backUp();
    expect((await run(RESTORE_CHECK, [path, path])).exitCode).not.toBe(0);
  });
});

describe("restore-check.sh privileges (LOG-006)", () => {
  const cases = ["event_log", "audit_log"].flatMap((table) =>
    ["select", "insert", "update", "delete", "truncate"].map((privilege) => [table, privilege] as const),
  );

  test.each(cases)("LOG-006: a dump where %s has the wrong %s privilege fails", async (table, privilege) => {
    const kept = privilege === "select" || privilege === "insert";
    await owner(kept ? `revoke ${privilege} on events.${table} from anima_runtime` : `grant ${privilege} on events.${table} to anima_runtime`);
    const result = await run(RESTORE_CHECK, [await backUp()]);
    expect(result.exitCode).toBe(1);
    expect(result.stderr).toContain(`events.${table}`);
  });

});
