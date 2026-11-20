// deploy/bin/phone-build-number: the build number of the phone applications from the release
// version (mobile plan M5-3, RLS-003).
import { describe, expect, test } from "bun:test";
import { join } from "node:path";

const BIN = join(import.meta.dir, "..", "bin");
const ENV = { PATH: process.env["PATH"] ?? "/usr/bin:/bin" };

function run(command: string, ...given: string[]): { exitCode: number; printed: string; said: string } {
  const result = Bun.spawnSync(["/bin/sh", join(BIN, command), ...given], { env: ENV, stdout: "pipe", stderr: "pipe" });
  return { exitCode: result.exitCode, printed: result.stdout.toString().trim(), said: result.stderr.toString().trim() };
}

const number = (version: string) => run("phone-build-number", version);

describe("the rule (RLS-003)", () => {
  test.each([
    ["0.0.1", "1"],
    ["0.0.99", "99"],
    ["0.1.0", "100"],
    ["0.7.95", "795"],
    ["0.38.1", "3801"],
    ["0.123.45", "12345"],
    ["0.99999.99", "9999999"],
    ["1.0.0", "10000000"],
    ["1.2.3", "10000203"],
    ["209.99999.99", "2099999999"],
  ])("the version %s gives the number %s", (version, expected) => {
    expect(number(version)).toEqual({ exitCode: 0, printed: expected, said: "" });
  });

  test("R-201: for the version of a commit the number is the number of commits on the first line", () => {
    for (const count of [1, 99, 100, 101, 795, 3801, 123456, 9999999]) {
      const version = run("commit-version", "--count", String(count));
      expect(version.exitCode).toBe(0);
      expect(number(version.printed).printed).toBe(String(count));
    }
  });

  test("a higher version always gives a higher number", () => {
    const rising = ["0.0.1", "0.0.2", "0.0.99", "0.1.0", "0.1.1", "0.9.99", "0.10.0", "0.99.99", "0.100.0", "0.99999.99", "1.0.0", "1.0.1", "1.99999.99", "2.0.0", "209.99999.99"];
    const numbers = rising.map((version) => Number(number(version).printed));
    for (let at = 1; at < numbers.length; at += 1) {
      expect(numbers[at]).toBeGreaterThan(numbers[at - 1] ?? Number.NaN);
    }
  });

  test("the highest number is a version code that Android takes", () => {
    expect(Number(number("209.99999.99").printed)).toBeLessThanOrEqual(2_100_000_000);
  });
});

describe("what is refused", () => {
  test.each([
    ["0.0.0", "no build number"],
    ["0.7.100", "not like 0.7.95"],
    ["0.100000.0", "not like 0.7.95"],
    ["210.0.0", "more than 209"],
    ["1000.0.0", "not like 0.7.95"],
    ["0.07.5", "not like 0.7.95"],
    ["0.7", "not like 0.7.95"],
    ["0.7.95.1", "not like 0.7.95"],
    ["v0.7.95", "not like 0.7.95"],
    ["0.7.95-beta", "not like 0.7.95"],
    ["", "not like 0.7.95"],
    ["0.7.95; id", "not like 0.7.95"],
  ])("the version %j is refused", (version, why) => {
    const answer = number(version);
    expect(answer.exitCode).toBe(1);
    expect(answer.printed).toBe("");
    expect(answer.said).toContain(why);
  });

  test("no version, or two, is refused", () => {
    expect(run("phone-build-number").exitCode).toBe(1);
    expect(run("phone-build-number", "0.7.95", "0.7.96").exitCode).toBe(1);
  });
});
