// deploy/bin/cli-formula and deploy/bin/crate-version: the Homebrew formula of a release and
// the version of the crate `anima-cli` (INS-010 to INS-012; task B8h). Nothing here reaches
// Homebrew, crates.io or the bucket.
import { afterAll, describe, expect, test } from "bun:test";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const ROOT = join(import.meta.dir, "..", "..");
const BIN = join(ROOT, "deploy", "bin");
const ROOM = mkdtempSync(join(tmpdir(), "cli-release-"));
const MAC = "a".repeat(64);
const LINUX = "b".repeat(64);

afterAll(() => {
  rmSync(ROOM, { recursive: true, force: true });
});

function run(program: string[], folder: string): { exitCode: number; printed: string } {
  const result = Bun.spawnSync(program, { cwd: folder, stdout: "pipe", stderr: "pipe" });
  return { exitCode: result.exitCode, printed: result.stdout.toString() };
}

const formula = (...given: string[]): { exitCode: number; printed: string } => run(["/bin/sh", join(BIN, "cli-formula"), ...given], ROOM);

describe("the Homebrew formula of a release (INS-010)", () => {
  const text = formula("0.8.15", MAC, LINUX).printed;

  test("it installs the file of the release for each platform, with the SHA-256 of that file", () => {
    const base = "https://releases.anima.ing/anima/0.8.15";
    const mac = `      url "${base}/anima-darwin-universal.tar.gz"\n      sha256 "${MAC}"\n`;
    expect(text.split(mac)).toHaveLength(3);
    expect(text).toContain(`      url "${base}/anima-linux-x86_64.tar.gz"\n      sha256 "${LINUX}"\n`);
    expect(text).toContain('  version "0.8.15"\n');
  });

  test("it installs `anima` only, builds nothing, and its test asks the program for its version", () => {
    expect(text).toContain('  def install\n    bin.install "anima"\n  end\n');
    expect(text).not.toMatch(/depends_on|system |cargo|rust/);
    expect(text).toContain('assert_match version.to_s, shell_output("#{bin}/anima --version")');
  });

  test.skipIf(Bun.which("ruby") === null)("it is a Ruby file that parses", () => {
    const file = join(ROOM, "anima.rb");
    writeFileSync(file, text);
    expect(run(["ruby", "-c", file], ROOM).exitCode).toBe(0);
  });

  test.each([
    ["a version that is no version", ["latest", MAC, LINUX]],
    ["a SHA-256 that is too short", ["0.8.15", "abc", LINUX]],
    ["a SHA-256 with a quote, which would end the text of the formula", ["0.8.15", MAC, `${"b".repeat(63)}"`]],
    ["one value too few", ["0.8.15", MAC]],
  ])("%s is refused, and no formula is printed", (_name, given) => {
    expect(formula(...given)).toEqual({ exitCode: 1, printed: "" });
  });
});

const MANIFEST = '[package]\nname = "anima-cli"\nversion.workspace = true\nedition.workspace = true\n\n[dependencies]\ntokio = { version = "1" }\n';
const LOCK = '[[package]]\nname = "anima-cli"\nversion = "0.1.0"\ndependencies = [\n "tokio",\n]\n\n[[package]]\nname = "anima-hub"\nversion = "0.1.0"\n';

/** A folder shaped as the root of the repository, with the two files that the command changes. */
function root(manifest: string, lock: string): string {
  const folder = mkdtempSync(join(ROOM, "root-"));
  mkdirSync(join(folder, "crates", "anima-cli"), { recursive: true });
  writeFileSync(join(folder, "crates", "anima-cli", "Cargo.toml"), manifest);
  writeFileSync(join(folder, "Cargo.lock"), lock);
  return folder;
}

const setVersion = (folder: string, ...given: string[]): number => run(["/bin/sh", join(BIN, "crate-version"), ...given], folder).exitCode;

describe("the version of the crate (INS-012)", () => {
  test("it writes the version of the release in the package and in its entry of the lock file, and nowhere else", () => {
    const folder = root(MANIFEST, LOCK);
    expect(setVersion(folder, "0.8.15")).toBe(0);
    expect(readFileSync(join(folder, "crates", "anima-cli", "Cargo.toml"), "utf8")).toBe(MANIFEST.replace("version.workspace = true", 'version = "0.8.15"'));
    expect(readFileSync(join(folder, "Cargo.lock"), "utf8")).toBe(LOCK.replace('name = "anima-cli"\nversion = "0.1.0"', 'name = "anima-cli"\nversion = "0.8.15"'));
  });

  test("the files of this repository have the two places that it changes", () => {
    expect(readFileSync(join(ROOT, "crates", "anima-cli", "Cargo.toml"), "utf8").split("\nversion.workspace = true\n")).toHaveLength(2);
    expect(readFileSync(join(ROOT, "Cargo.lock"), "utf8").split('\nname = "anima-cli"\nversion = ')).toHaveLength(2);
  });

  test.each([
    ["a version that is no version", MANIFEST, LOCK, ["1.0"]],
    ["a package whose version is already written", MANIFEST.replace("version.workspace = true", 'version = "0.1.0"'), LOCK, ["0.8.15"]],
    ["a lock file with no entry of the package", MANIFEST, LOCK.replace("anima-cli", "other"), ["0.8.15"]],
  ])("%s is refused, and the two files stay as they were", (_name, manifest, lock, given) => {
    const folder = root(manifest, lock);
    expect(setVersion(folder, ...given)).toBe(1);
    expect(readFileSync(join(folder, "crates", "anima-cli", "Cargo.toml"), "utf8")).toBe(manifest);
    expect(readFileSync(join(folder, "Cargo.lock"), "utf8")).toBe(lock);
  });
});
