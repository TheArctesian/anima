import { chmodSync, existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

// VPS-006 / D-12: backup.sh and prune.sh run against a temporary Headscale folder with fake
// sqlite3 and restic. The dummy files are placeholders, never real keys.

export const BACKUP = resolve(import.meta.dir, "..", "backup");
export const PASSWORD = "pw-SENTINEL-7f3a";
export const REPOSITORY_SECRET = "repo-SENTINEL-91c2";
export const REPOSITORY = `rest:http://headscale-vps:${REPOSITORY_SECRET}@10.10.0.5:8000/headscale-vps/headscale`;

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

// backup: lists the folder (names, then `ls -l` modes) and may sleep; snapshots --json: prints $FAKE_SNAPSHOTS.
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
[[ "$FAKE_FAIL" == "restic-$1" ]] && { echo "Fatal: unable to open repository" >&2; exit 1; }
if [[ "$1" == backup ]]; then
  for dir in "$@"; do :; done
  ls -l "$dir" > "$FAKE_DIR/snapshot.modes"
  ls -ld "$dir" >> "$FAKE_DIR/snapshot.modes"
  cp "$dir/db.sqlite" "$FAKE_DIR/snapshot-db"
  ls "$dir" > "$FAKE_DIR/snapshot.list"
  [[ -n "$FAKE_SLEEP" ]] && sleep "$FAKE_SLEEP"
fi
if [[ "$1" == snapshots ]]; then
  if [[ -n "$FAKE_SNAPSHOTS" ]]; then cat "$FAKE_SNAPSHOTS"; else echo "[]"; fi
  exit 0
fi
echo "restic $1 done"
`;

export interface Call {
  name: string;
  argv: string[];
  env: string[];
}

export interface Run {
  code: number;
  out: string;
  calls: Call[];
  root: string;
  leftover: string[];
}

const roots: string[] = [];

/** Removes every fixture made so far; call from `afterEach` in each test file. */
export function removeFixtures(): void {
  for (const root of roots.splice(0)) rmSync(root, { recursive: true, force: true });
}

function write(path: string, text: string, mode = 0o600): void {
  mkdirSync(resolve(path, ".."), { recursive: true });
  writeFileSync(path, text);
  chmodSync(path, mode);
}

/** A temporary root with fake commands, a dummy Headscale folder, secret files and a TMPDIR. */
export function fixture(): string {
  const root = mkdtempSync(join(tmpdir(), "anima-backup-test-"));
  roots.push(root);
  write(join(root, "bin", "sqlite3"), FAKE_SQLITE, 0o755);
  write(join(root, "bin", "restic"), FAKE_RESTIC, 0o755);
  write(join(root, "headscale", "lib", "db.sqlite"), "dummy database", 0o644);
  write(join(root, "headscale", "lib", "noise_private.key"), "dummy noise key");
  write(join(root, "headscale", "lib", "derp_server_private.key"), "dummy derp key");
  write(join(root, "headscale", "config", "config.yaml"), "server_url: https://example.invalid\n", 0o644);
  write(join(root, "headscale", "config", "policy.hujson"), "{}\n", 0o644);
  write(join(root, "secrets", "restic-repository"), REPOSITORY);
  write(join(root, "secrets", "restic-password"), PASSWORD);
  mkdirSync(join(root, "tmp"));
  mkdirSync(join(root, "repository"));
  return root;
}

/** Writes a `restic snapshots --json` answer for the fake and returns its path. */
export function snapshotList(root: string, times: string[]): string {
  const path = join(root, "snapshots.json");
  write(path, JSON.stringify(times.map((time) => ({ time, hostname: "anima-vps", tags: ["headscale"] }))));
  return path;
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

export function scriptEnv(root: string, env: Record<string, string>): Record<string, string> {
  return {
    PATH: process.env.PATH ?? "/usr/bin:/bin",
    TMPDIR: join(root, "tmp"),
    FAKE_DIR: root,
    HEADSCALE_DIR: join(root, "headscale"),
    RESTIC_REPOSITORY_FILE: join(root, "secrets", "restic-repository"),
    RESTIC_PASSWORD_FILE: join(root, "secrets", "restic-password"),
    RESTIC_REPOSITORY_PATH: join(root, "repository"),
    SQLITE3: join(root, "bin", "sqlite3"),
    RESTIC: join(root, "bin", "restic"),
    ...env,
  };
}

export function outcome(root: string, code: number, out: string): Run {
  const logPath = join(root, "calls.log");
  return {
    code,
    out,
    calls: existsSync(logPath) ? parseCalls(readFileSync(logPath, "utf8")) : [],
    root,
    leftover: readdirSync(join(root, "tmp")),
  };
}

export function run(script: string, env: Record<string, string> = {}, root = fixture()): Run {
  const result = Bun.spawnSync(["bash", join(BACKUP, script)], { env: scriptEnv(root, env) });
  return outcome(root, result.exitCode, result.stdout.toString() + result.stderr.toString());
}

export function sqlOf(calls: Call[]): string[] {
  return calls.filter((call) => call.name === "sqlite3").map((call) => call.argv.at(-1) ?? "");
}

export function resticCommands(calls: Call[]): string[][] {
  return calls.filter((call) => call.name === "restic").map((call) => call.argv);
}

/** Values of `key` in `[section]` of a systemd unit file, in order. */
export function unitValues(file: string, section: string, key: string): string[] {
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
