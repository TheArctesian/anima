// apps/desktop/src-tauri/ios-archive.sh: what a run makes, from its version and its two
// variables (mobile plan M5-4). `--plan` builds nothing, so no test needs Xcode.
import { describe, expect, test } from "bun:test";
import { join } from "node:path";

const SCRIPT = join(import.meta.dir, "..", "..", "apps", "desktop", "src-tauri", "ios-archive.sh");

function plan(set: Record<string, string>, ...given: string[]): { exitCode: number; printed: string; said: string } {
  const env = { PATH: process.env["PATH"] ?? "/usr/bin:/bin", ...set };
  const result = Bun.spawnSync(["/bin/sh", SCRIPT, "--plan", ...given], { env, stdout: "pipe", stderr: "pipe" });
  return { exitCode: result.exitCode, printed: result.stdout.toString().trim(), said: result.stderr.toString().trim() };
}

const SIGNED = { IOS_SIGN_IDENTITY: "Apple Distribution: Example LLC (ABCDE12345)", IOS_PROFILE: "/nowhere/app.mobileprovision" };

// This script packages iOS on Unix release hosts.
describe.skipIf(process.platform === "win32")("the plan of a run", () => {
  test("M5-3: the build number comes from the version, and with no identity there is no signature", () => {
    expect(plan({}, "0.7.95")).toEqual({ exitCode: 0, printed: "version=0.7.95\nbuild=795\nsigned=no", said: "" });
  });

  test("RLS-011: with an identity and a profile the run signs", () => {
    expect(plan(SIGNED, "0.38.1").printed).toBe("version=0.38.1\nbuild=3801\nsigned=yes");
  });

  test("R-183: with no version it is the version of the commit that is checked out", () => {
    const answer = plan({});
    expect(answer.exitCode).toBe(0);
    expect(answer.printed).toMatch(/^version=0\.\d+\.\d+\nbuild=[1-9]\d*\nsigned=no$/);
  });

  test("LOG-007: the plan prints no identity and no path of a profile", () => {
    const answer = plan(SIGNED, "0.1.0");
    expect(answer.printed + answer.said).not.toContain("Example LLC");
    expect(answer.printed + answer.said).not.toContain("mobileprovision");
  });
});

// This script packages iOS on Unix release hosts.
describe.skipIf(process.platform === "win32")("what is refused before a build", () => {
  test.each([
    [{ IOS_SIGN_IDENTITY: SIGNED.IOS_SIGN_IDENTITY }, "0.1.0", "IOS_PROFILE must name"],
    [{ IOS_PROFILE: SIGNED.IOS_PROFILE }, "0.1.0", "IOS_SIGN_IDENTITY must name"],
    [{ ...SIGNED, IOS_SIGN_IDENTITY: 'a"b' }, "0.1.0", "holds a character"],
    [{ ...SIGNED, IOS_SIGN_IDENTITY: "a\nCODE_SIGNING_ALLOWED = NO" }, "0.1.0", "holds a character"],
    [{ ...SIGNED, IOS_SIGN_IDENTITY: "a$(id)" }, "0.1.0", "holds a character"],
    [{}, "1.2", "not like 0.1.0"],
    [{}, "0.1.0; id", "not like 0.1.0"],
    [{}, "0.7.100", "no build number"],
    [{}, "0.0.0", "no build number"],
  ])("%j with the version %j", (set, version, why) => {
    const answer = plan(set, version);
    expect(answer.exitCode).toBe(1);
    expect(answer.printed).toBe("");
    expect(answer.said).toContain(why);
  });

  test("two versions are refused", () => {
    expect(plan({}, "0.1.0", "0.1.1").exitCode).toBe(1);
  });
});
