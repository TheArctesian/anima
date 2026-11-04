import { afterEach, describe, expect, test } from "bun:test";
import { existsSync, mkdirSync, readFileSync, rmSync } from "node:fs";
import { join } from "node:path";
import {
  BACKUP, fixture, outcome, PASSWORD, removeFixtures, REPOSITORY, REPOSITORY_SECRET, resticCommands, run,
  scriptEnv, sqlOf,
} from "./backup-fixture.ts";

const SNAPSHOT_ITEMS = ["config.yaml", "db.sqlite", "derp_server_private.key", "noise_private.key", "policy.hujson"];

afterEach(removeFixtures);

describe("backup.sh", () => {
  test("VPS-006 snapshots the database with VACUUM INTO and checks the copy with integrity_check", () => {
    const result = run("backup.sh");
    expect(result.code).toBe(0);
    const [vacuum, integrity] = sqlOf(result.calls);
    expect(vacuum).toMatch(/^VACUUM INTO '.+\/db\.sqlite'$/);
    expect(integrity).toBe("PRAGMA integrity_check");
    const sqliteCalls = result.calls.filter((call) => call.name === "sqlite3");
    expect(sqliteCalls[0]?.argv).toContain(join(result.root, "headscale", "lib", "db.sqlite"));
    expect(sqliteCalls[0]?.argv).toContain("-readonly");
    expect(sqliteCalls[1]?.argv.at(-2)).toBe(vacuum?.slice("VACUUM INTO '".length, -"/db.sqlite'".length) + "/db.sqlite");
  });

  test("VPS-006 backs up the database snapshot, both private keys, config.yaml and policy.hujson", () => {
    const result = run("backup.sh");
    expect(result.code).toBe(0);
    expect(readFileSync(join(result.root, "snapshot.list"), "utf8").trim().split("\n")).toEqual(SNAPSHOT_ITEMS);
    expect(readFileSync(join(result.root, "snapshot-db"), "utf8")).toBe("dummy database");
  });

  test("VPS-006 stages the snapshot readable by its owner only, whatever the source modes", () => {
    const result = run("backup.sh");
    const modes = readFileSync(join(result.root, "snapshot.modes"), "utf8").trim().split("\n")
      .filter((line) => !line.startsWith("total")).map((line) => line.slice(0, 10));
    expect(modes).toEqual([...SNAPSHOT_ITEMS.map(() => "-rw-------"), "drwx------"]);
  });

  test("VPS-006 gives restic the repository and password as files, with host anima-vps and tag headscale", () => {
    const result = run("backup.sh");
    const commands = resticCommands(result.calls);
    expect(commands).toHaveLength(1);
    expect(commands[0]?.slice(0, 4)).toEqual([
      "--repository-file", join(result.root, "secrets", "restic-repository"),
      "--password-file", join(result.root, "secrets", "restic-password"),
    ]);
    expect(commands[0]).toEqual(expect.arrayContaining(["backup", "--host", "anima-vps", "--tag", "headscale"]));
    expect(readFileSync(join(result.root, "seen-password"), "utf8")).toBe(PASSWORD);
    expect(readFileSync(join(result.root, "seen-repository"), "utf8")).toBe(REPOSITORY);
  });

  test("VPS-006 reads the repository and password from the systemd credentials folder", () => {
    const root = fixture();
    const env = { CREDENTIALS_DIRECTORY: join(root, "secrets"), RESTIC_REPOSITORY_FILE: "", RESTIC_PASSWORD_FILE: "" };
    const result = run("backup.sh", env, root);
    expect(result.code).toBe(0);
    expect(resticCommands(result.calls)[0]).toEqual(expect.arrayContaining([join(root, "secrets", "restic-password")]));
  });

  test("VPS-006 never puts the password or repository in argv, the environment or the output", () => {
    for (const fail of ["", "restic-backup"]) {
      const result = run("backup.sh", { FAKE_FAIL: fail, RESTIC_PASSWORD: "", RESTIC_REPOSITORY: "" });
      const seen = [result.out, ...result.calls.flatMap((call) => [...call.argv, ...call.env])].join("\n");
      for (const secret of [PASSWORD, REPOSITORY_SECRET]) expect(seen).not.toContain(secret);
      for (const call of result.calls) expect(call.env.filter((line) => /^RESTIC_(PASSWORD|REPOSITORY)=/.test(line))).toEqual([]);
    }
  });

  test("VPS-006 removes the temporary snapshot after a successful backup", () => {
    const result = run("backup.sh");
    expect(result.code).toBe(0);
    expect(existsSync(join(result.root, "snapshot.list"))).toBe(true);
    expect(sqlOf(result.calls)[0]).toStartWith(`VACUUM INTO '${join(result.root, "tmp")}/`);
    expect(result.leftover).toEqual([]);
  });

  const failures: [string, Record<string, string>, string][] = [
    ["sqlite3 VACUUM INTO fails", { FAKE_FAIL: "vacuum" }, "VACUUM INTO failed"],
    ["integrity_check is not ok", { FAKE_INTEGRITY: "*** in database main ***\nPage 3 is never used" }, "did not return ok"],
    ["integrity_check cannot run", { FAKE_FAIL: "integrity-run" }, "integrity_check could not run"],
    ["restic backup fails", { FAKE_FAIL: "restic-backup" }, "restic backup failed"],
    ["the password file is missing", { RESTIC_PASSWORD_FILE: "/nonexistent/restic-password" }, "password file is missing"],
    ["no credentials are set", { RESTIC_REPOSITORY_FILE: "", RESTIC_PASSWORD_FILE: "" }, "repository file is not set"],
  ];
  test.each(failures)("VPS-006 exits non-zero and removes the snapshot when %s", (_name, env, message) => {
    const result = run("backup.sh", env);
    expect(result.code).not.toBe(0);
    expect(result.out).toContain(message);
    expect(result.leftover).toEqual([]);
  });

  test("VPS-006 does not send a snapshot whose integrity_check is not ok", () => {
    const result = run("backup.sh", { FAKE_INTEGRITY: "row 1 missing from index" });
    expect(resticCommands(result.calls)).toEqual([]);
  });

  test("VPS-006 exits non-zero and removes the snapshot when a key file is missing", () => {
    const root = fixture();
    rmSync(join(root, "headscale", "lib", "derp_server_private.key"));
    const result = run("backup.sh", {}, root);
    expect(result.code).not.toBe(0);
    expect(result.out).toContain("could not copy");
    expect(result.leftover).toEqual([]);
  });

  test("VPS-006 refuses a temporary folder whose path would break the VACUUM INTO quoting", () => {
    const root = fixture();
    const quoted = join(root, "tmp", "it's");
    mkdirSync(quoted);
    const result = run("backup.sh", { TMPDIR: quoted }, root);
    expect(result.code).not.toBe(0);
    expect(result.out).toContain("contains a quote");
    expect(sqlOf(result.calls)).toEqual([]);
    expect(result.leftover).toEqual(["it's"]);
    expect(Bun.spawnSync(["ls", "-A", quoted]).stdout.toString()).toBe("");
  });

  test("VPS-006 removes the snapshot when systemd stops the backup with SIGTERM", async () => {
    const root = fixture();
    const backup = Bun.spawn(["bash", join(BACKUP, "backup.sh")], { env: scriptEnv(root, { FAKE_SLEEP: "1" }) });
    const deadline = Date.now() + 5000;
    while (!existsSync(join(root, "snapshot.list")) && Date.now() < deadline) await Bun.sleep(20);
    backup.kill("SIGTERM");
    const result = outcome(root, await backup.exited, "");
    expect(result.code).toBe(143);
    expect(result.leftover).toEqual([]);
  }, 10_000);

  test("VPS-006 never runs restic forget or prune from the VPS", () => {
    const result = run("backup.sh");
    expect(resticCommands(result.calls).flat()).not.toEqual(expect.arrayContaining(["forget"]));
    expect(resticCommands(result.calls).flat()).not.toEqual(expect.arrayContaining(["prune"]));
    const code = readFileSync(join(BACKUP, "backup.sh"), "utf8").split("\n").filter((line) => !line.trim().startsWith("#"));
    expect(code.filter((line) => /\b(forget|prune)\b/.test(line))).toEqual([]);
    expect(code.filter((line) => /\bset\s+-[a-z]*x/.test(line))).toEqual([]);
  });
});

describe("shellcheck", () => {
  test("VPS-006 backup.sh and prune.sh pass shellcheck (a missing shellcheck fails, CQ-024)", () => {
    const shellcheck = Bun.which("shellcheck");
    expect(shellcheck).not.toBeNull();
    const result = Bun.spawnSync([shellcheck ?? "shellcheck", join(BACKUP, "backup.sh"), join(BACKUP, "prune.sh")]);
    expect(result.stdout.toString()).toBe("");
    expect(result.exitCode).toBe(0);
  });
});
