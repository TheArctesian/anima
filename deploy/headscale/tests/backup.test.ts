import { afterEach, describe, expect, test } from "bun:test";
import { chmodSync, existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

// VPS-006 / D-12: backup.sh and prune.sh run against a temporary Headscale folder with fake
// sqlite3 and restic. The dummy files are placeholders, never real keys.

const BACKUP = resolve(import.meta.dir, "..", "backup");
const PASSWORD = "pw-SENTINEL-7f3a";
const REPOSITORY_SECRET = "repo-SENTINEL-91c2";
const REPOSITORY = `rest:http://headscale-vps:${REPOSITORY_SECRET}@10.10.0.5:8000/headscale-vps/headscale`;
const SNAPSHOT_ITEMS = ["config.yaml", "db.sqlite", "derp_server_private.key", "noise_private.key", "policy.hujson"];

// Each call logs "CALL <name>", one "ARG <value>" line per argument and one "ENV <k=v>" line per variable.
const RECORD = String.raw`log="$FAKE_DIR/calls.log"
printf 'CALL %s\n' "$(basename "$0")" >> "$log"
for a in "$@"; do printf 'ARG %s\n' "$a" >> "$log"; done
env | sed 's/^/ENV /' >> "$log"`;

const FAKE_SQLITE = String.raw`#!/bin/bash
${RECORD}
prev=""; for a in "$@"; do db="$prev"; prev="$a"; done; sql="$prev"
case "$sql" in
  "VACUUM INTO "*)
    [[ "$FAKE_FAIL" == vacuum ]] && { echo "Error: disk I/O error" >&2; exit 1; }
    cp "$db" "$(printf '%s' "$sql" | sed "s/^VACUUM INTO '\(.*\)'$/\1/")" ;;
  "PRAGMA integrity_check")
    [[ "$FAKE_FAIL" == integrity-run ]] && { echo "Error: file is not a database" >&2; exit 26; }
    if [[ -n "$FAKE_INTEGRITY" ]]; then echo "$FAKE_INTEGRITY"; else echo ok; fi ;;
  *) echo "unexpected sql" >&2; exit 2 ;;
esac
`;

const FAKE_RESTIC = String.raw`#!/bin/bash
${RECORD}
while [[ $# -gt 0 ]]; do
  case "$1" in
    --password-file) cp "$2" "$FAKE_DIR/seen-password"; shift 2 ;;
    --repository-file) cp "$2" "$FAKE_DIR/seen-repository"; shift 2 ;;
    --repo|--cache-dir) shift 2 ;;
    --no-cache) shift ;;
    *) break ;;
  esac
done
[[ "$FAKE_FAIL" == "restic-$1" ]] && { echo "Fatal: unable to save snapshot" >&2; exit 1; }
if [[ "$1" == backup ]]; then
  for dir in "$@"; do :; done
  ls "$dir" > "$FAKE_DIR/snapshot.list"
  cp "$dir/db.sqlite" "$FAKE_DIR/snapshot-db"
fi
echo "restic $1 done"
`;

interface Call {
  name: string;
  argv: string[];
  env: string[];
}

interface Run {
  code: number;
  out: string;
  calls: Call[];
  root: string;
  leftover: string[];
}

const roots: string[] = [];
afterEach(() => {
  for (const root of roots.splice(0)) rmSync(root, { recursive: true, force: true });
});

function write(path: string, text: string, mode = 0o600): void {
  mkdirSync(resolve(path, ".."), { recursive: true });
  writeFileSync(path, text);
  chmodSync(path, mode);
}

function fixture(): string {
  const root = mkdtempSync(join(tmpdir(), "anima-backup-test-"));
  roots.push(root);
  write(join(root, "bin", "sqlite3"), FAKE_SQLITE, 0o755);
  write(join(root, "bin", "restic"), FAKE_RESTIC, 0o755);
  write(join(root, "headscale", "lib", "db.sqlite"), "dummy database");
  write(join(root, "headscale", "lib", "noise_private.key"), "dummy noise key");
  write(join(root, "headscale", "lib", "derp_server_private.key"), "dummy derp key");
  write(join(root, "headscale", "config", "config.yaml"), "server_url: https://example.invalid\n");
  write(join(root, "headscale", "config", "policy.hujson"), "{}\n");
  write(join(root, "secrets", "restic-repository"), REPOSITORY);
  write(join(root, "secrets", "restic-password"), PASSWORD);
  mkdirSync(join(root, "tmp"));
  return root;
}

function parseCalls(log: string): Call[] {
  const calls: Call[] = [];
  for (const line of log.split("\n")) {
    if (line.startsWith("CALL ")) calls.push({ name: line.slice(5), argv: [], env: [] });
    const last = calls.at(-1);
    if (last === undefined) continue;
    if (line.startsWith("ARG ")) last.argv.push(line.slice(4));
    if (line.startsWith("ENV ")) last.env.push(line.slice(4));
  }
  return calls;
}

function run(script: string, env: Record<string, string> = {}, root = fixture()): Run {
  const result = Bun.spawnSync(["bash", join(BACKUP, script)], {
    env: {
      PATH: process.env.PATH ?? "/usr/bin:/bin",
      TMPDIR: join(root, "tmp"),
      FAKE_DIR: root,
      HEADSCALE_DIR: join(root, "headscale"),
      RESTIC_REPOSITORY_FILE: join(root, "secrets", "restic-repository"),
      RESTIC_PASSWORD_FILE: join(root, "secrets", "restic-password"),
      SQLITE3: join(root, "bin", "sqlite3"),
      RESTIC: join(root, "bin", "restic"),
      ...env,
    },
  });
  const logPath = join(root, "calls.log");
  return {
    code: result.exitCode,
    out: result.stdout.toString() + result.stderr.toString(),
    calls: existsSync(logPath) ? parseCalls(readFileSync(logPath, "utf8")) : [],
    root,
    leftover: readdirSync(join(root, "tmp")),
  };
}

function sqlOf(calls: Call[]): string[] {
  return calls.filter((call) => call.name === "sqlite3").map((call) => call.argv.at(-1) ?? "");
}

function resticCommands(calls: Call[]): string[][] {
  return calls.filter((call) => call.name === "restic").map((call) => call.argv);
}

/** Values of `key` in `[section]` of a systemd unit file, in order. */
function unitValues(file: string, section: string, key: string): string[] {
  let current = "";
  const values: string[] = [];
  for (const raw of readFileSync(join(BACKUP, file), "utf8").split("\n")) {
    const line = raw.trim();
    if (line === "" || line.startsWith("#")) continue;
    const header = /^\[(.+)\]$/.exec(line);
    if (header) {
      current = header[1] ?? "";
      continue;
    }
    const eq = line.indexOf("=");
    if (current === section && line.slice(0, eq) === key) values.push(line.slice(eq + 1));
  }
  return values;
}

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

  test("VPS-006 gives restic the repository and password as files, with host anima-vps and tag headscale", () => {
    const result = run("backup.sh");
    const [backup, snapshots] = resticCommands(result.calls);
    expect(backup?.slice(0, 4)).toEqual([
      "--repository-file", join(result.root, "secrets", "restic-repository"),
      "--password-file", join(result.root, "secrets", "restic-password"),
    ]);
    expect(backup).toEqual(expect.arrayContaining(["backup", "--host", "anima-vps", "--tag", "headscale"]));
    expect(snapshots).toEqual(expect.arrayContaining(["snapshots", "--latest", "1", "--tag", "headscale"]));
    expect(readFileSync(join(result.root, "seen-password"), "utf8")).toBe(PASSWORD);
    expect(readFileSync(join(result.root, "seen-repository"), "utf8")).toBe(REPOSITORY);
  });

  test("VPS-006 reads the repository and password from the systemd credentials folder", () => {
    const root = fixture();
    const result = run("backup.sh", { CREDENTIALS_DIRECTORY: join(root, "secrets"), RESTIC_REPOSITORY_FILE: "", RESTIC_PASSWORD_FILE: "" }, root);
    expect(result.code).toBe(0);
    expect(resticCommands(result.calls)[0]).toEqual(expect.arrayContaining([join(root, "secrets", "restic-password")]));
  });

  test("VPS-006 never puts the password or repository in argv, the environment or the output", () => {
    for (const fail of ["", "restic-backup", "restic-snapshots"]) {
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
    ["restic snapshots fails", { FAKE_FAIL: "restic-snapshots" }, "could not confirm"],
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

  test("VPS-006 never runs restic forget or prune from the VPS", () => {
    const result = run("backup.sh");
    expect(resticCommands(result.calls).flat()).not.toEqual(expect.arrayContaining(["forget"]));
    expect(resticCommands(result.calls).flat()).not.toEqual(expect.arrayContaining(["prune"]));
    const code = readFileSync(join(BACKUP, "backup.sh"), "utf8").split("\n").filter((line) => !line.trim().startsWith("#"));
    expect(code.filter((line) => /\b(forget|prune)\b/.test(line))).toEqual([]);
    expect(code.filter((line) => /\bset\s+-[a-z]*x/.test(line))).toEqual([]);
  });
});

describe("prune.sh", () => {
  test("VPS-006 applies 14 daily, 8 weekly and 12 monthly snapshots grouped by host and tag, then checks", () => {
    const root = fixture();
    mkdirSync(join(root, "repository"));
    const result = run("prune.sh", { RESTIC_REPOSITORY_PATH: join(root, "repository") }, root);
    expect(result.code).toBe(0);
    const [forget, check] = resticCommands(result.calls);
    expect(forget?.slice(0, 4)).toEqual(["--repo", join(root, "repository"), "--password-file", join(root, "secrets", "restic-password")]);
    expect(forget).toEqual(expect.arrayContaining([
      "forget", "--host", "anima-vps", "--tag", "headscale", "--group-by", "host,tags",
      "--keep-daily", "14", "--keep-weekly", "8", "--keep-monthly", "12", "--prune",
    ]));
    expect(check).toContain("check");
    expect(result.out).not.toContain(PASSWORD);
  });

  test("VPS-006 prune exits non-zero when restic forget fails", () => {
    const root = fixture();
    mkdirSync(join(root, "repository"));
    const result = run("prune.sh", { RESTIC_REPOSITORY_PATH: join(root, "repository"), FAKE_FAIL: "restic-forget" }, root);
    expect(result.code).not.toBe(0);
    expect(result.out).toContain("forget --prune failed");
  });
});

describe("systemd units", () => {
  test("VPS-006 the backup timer runs daily at 03:15 UTC with a random delay and catches up missed runs", () => {
    expect(unitValues("headscale-backup.timer", "Timer", "OnCalendar")).toEqual(["*-*-* 03:15:00 UTC"]);
    expect(unitValues("headscale-backup.timer", "Timer", "RandomizedDelaySec")).toEqual(["15m"]);
    expect(unitValues("headscale-backup.timer", "Timer", "Persistent")).toEqual(["true"]);
    expect(unitValues("headscale-backup.timer", "Timer", "Unit")).toEqual(["headscale-backup.service"]);
  });

  test("VPS-006 the backup unit is a sandboxed oneshot that reads its secrets with LoadCredential", () => {
    const value = (key: string) => unitValues("headscale-backup.unit", "Service", key);
    expect(value("Type")).toEqual(["oneshot"]);
    expect(value("ProtectSystem")).toEqual(["strict"]);
    expect(value("ReadOnlyPaths")).toEqual(["/opt/anima/headscale"]);
    expect(value("PrivateTmp")).toEqual(["yes"]);
    expect(value("NoNewPrivileges")).toEqual(["yes"]);
    expect(value("ProtectHome")).toEqual(["yes"]);
    expect(value("CapabilityBoundingSet")).toEqual([""]);
    expect(value("IPAddressDeny")).toEqual(["any"]);
    expect(value("LoadCredential").map((entry) => entry.split(":")[0])).toEqual(["restic-repository", "restic-password"]);
    expect(value("Environment")).toEqual(expect.arrayContaining([
      "RESTIC_REPOSITORY_FILE=%d/restic-repository", "RESTIC_PASSWORD_FILE=%d/restic-password",
    ]));
    expect(value("Environment").filter((entry) => /^RESTIC_(PASSWORD|REPOSITORY)=/.test(entry))).toEqual([]);
  });

  test("VPS-006 rest-server on the home PC is append-only with private repositories, as user restic", () => {
    const value = (key: string) => unitValues("rest-server.unit", "Service", key);
    const argv = value("ExecStart")[0]?.split(/\s+/) ?? [];
    expect(argv).toEqual(expect.arrayContaining(["--append-only", "--private-repos", "--path", "/srv/restic"]));
    expect(argv).toEqual(expect.arrayContaining(["--listen", "${HOME_PC_LAN_IP}:8000"]));
    expect(argv).not.toContain("--no-auth");
    expect(value("User")).toEqual(["restic"]);
    expect(value("ProtectSystem")).toEqual(["strict"]);
    expect(value("ReadWritePaths")).toEqual(["/srv/restic"]);
  });

  test("VPS-006 prune runs weekly on the home PC, offline, as user restic", () => {
    expect(unitValues("restic-prune.timer", "Timer", "OnCalendar")).toEqual(["Sun *-*-* 05:00:00"]);
    expect(unitValues("restic-prune.timer", "Timer", "Persistent")).toEqual(["true"]);
    expect(unitValues("restic-prune.unit", "Service", "User")).toEqual(["restic"]);
    expect(unitValues("restic-prune.unit", "Service", "PrivateNetwork")).toEqual(["yes"]);
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
