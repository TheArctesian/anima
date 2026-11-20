// The refusals of apps/desktop/src-tauri/check-mach-o.sh (UPD-011), the check of each Mach-O
// file that the job `macos` of the release workflow runs before the notary step.
import { describe, expect, test } from "bun:test";
import { chmodSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const MACH_O_CHECK = join(import.meta.dir, "..", "..", "apps", "desktop", "src-tauri", "check-mach-o.sh");

/** A folder shaped as an application whose programs are copies of one universal file with an ad hoc signature. */
function adHocApp(programs: string[]): { folder: string; app: string } {
  const folder = mkdtempSync(join(tmpdir(), "anima-mach-o-"));
  const app = join(folder, "Anima.ing.app");
  const macos = join(app, "Contents", "MacOS");
  mkdirSync(macos, { recursive: true });
  const source = join(folder, "main.c");
  writeFileSync(source, "int main(void) { return 0; }\n");
  const built = join(folder, "program");
  const run = (argv: string[]): void => {
    const result = Bun.spawnSync(argv, { stdout: "ignore", stderr: "pipe" });
    if (result.exitCode !== 0) throw new Error(`${argv[0] ?? ""} failed: ${result.stderr.toString()}`);
  };
  run(["/usr/bin/cc", "-arch", "arm64", "-arch", "x86_64", "-o", built, source]);
  for (const program of programs) {
    writeFileSync(join(macos, program), readFileSync(built));
    chmodSync(join(macos, program), 0o755);
    run(["/usr/bin/codesign", "--force", "--sign", "-", join(macos, program)]);
  }
  return { folder, app };
}

function machOCheck(app: string): { exitCode: number; errors: string } {
  const result = Bun.spawnSync(["/bin/sh", MACH_O_CHECK, app, "ABCDE12345"], { stdout: "ignore", stderr: "pipe" });
  return { exitCode: result.exitCode, errors: result.stderr.toString() };
}

const PROGRAMS = ["anima", "anima-daemon", "anima-database", "anima-desktop", "anima-gateway", "anima-hub", "anima-link", "anima-local-hub"];

// The checked programs are of macOS (codesign, lipo); the pinning gate is skipped elsewhere in the same way.
describe.skipIf(process.platform !== "darwin")("check-mach-o.sh (UPD-011)", () => {
  test("an application whose programs have an ad hoc signature is refused", () => {
    const { folder, app } = adHocApp(PROGRAMS);
    try {
      const result = machOCheck(app);
      expect(result.exitCode).toBe(1);
      expect(result.errors).toContain("check-mach-o:");
    } finally {
      rmSync(folder, { recursive: true, force: true });
    }
  });

  test("an application with a program that a release does not carry is refused by its name", () => {
    const { folder, app } = adHocApp([...PROGRAMS, "extra-program"]);
    try {
      const result = machOCheck(app);
      expect(result.exitCode).toBe(1);
      expect(result.errors).toContain("Contents/MacOS holds");
    } finally {
      rmSync(folder, { recursive: true, force: true });
    }
  });
});
