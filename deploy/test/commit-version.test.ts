// deploy/bin/commit-version: the version of a commit from the number of commits of its
// history (R-183). Each repository here is made in a temporary folder.
import { afterAll, describe, expect, test } from "bun:test";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const COMMAND = join(import.meta.dir, "..", "bin", "commit-version");
const ROOM = mkdtempSync(join(tmpdir(), "commit-version-"));
const GIT_ENV = {
  PATH: process.env["PATH"] ?? "/usr/bin:/bin",
  HOME: ROOM,
  GIT_CONFIG_NOSYSTEM: "1",
  GIT_AUTHOR_NAME: "A Test",
  GIT_AUTHOR_EMAIL: "test@example.invalid",
  GIT_COMMITTER_NAME: "A Test",
  GIT_COMMITTER_EMAIL: "test@example.invalid",
};

afterAll(() => {
  rmSync(ROOM, { recursive: true, force: true });
});

function run(program: string[], folder: string): { exitCode: number; printed: string; said: string } {
  const result = Bun.spawnSync(program, { cwd: folder, env: GIT_ENV, stdout: "pipe", stderr: "pipe" });
  return { exitCode: result.exitCode, printed: result.stdout.toString().trim(), said: result.stderr.toString().trim() };
}

function version(folder: string, ...given: string[]): { exitCode: number; printed: string; said: string } {
  return run(["/bin/sh", COMMAND, ...given], folder);
}

/** A repository with `count` commits on one line; it gives its folder. */
function repositoryWith(count: number): string {
  const folder = mkdtempSync(join(ROOM, "repository-"));
  expect(run(["git", "init", "--quiet", "--initial-branch=main"], folder).exitCode).toBe(0);
  for (let made = 1; made <= count; made += 1) {
    expect(run(["git", "commit", "--quiet", "--allow-empty", "-m", `commit ${String(made)}`], folder).exitCode).toBe(0);
  }
  return folder;
}

describe("the formula (R-183)", () => {
  test.each([
    ["1", "0.0.1"],
    ["99", "0.0.99"],
    ["100", "0.1.0"],
    ["101", "0.1.1"],
    ["3801", "0.38.1"],
    ["9999", "0.99.99"],
    ["10000", "0.100.0"],
  ])("commit %s has the version %s", (count, expected) => {
    expect(version(ROOM, "--count", count)).toEqual({ exitCode: 0, printed: expected, said: "" });
  });

  test.each(["0", "01", "-3", "1.5", "many", ""])("the number %p is refused", (count) => {
    const refused = version(ROOM, "--count", count);
    expect(refused.exitCode).toBe(1);
    expect(refused.printed).toBe("");
  });
});

describe("the version of a commit of a repository", () => {
  const folder = repositoryWith(3);

  test("with no commit named, it is the version of the commit that is checked out", () => {
    expect(version(folder)).toEqual({ exitCode: 0, printed: "0.0.3", said: "" });
  });

  test("an older commit keeps the version that it had", () => {
    expect(version(folder, "HEAD~2").printed).toBe("0.0.1");
    expect(run(["git", "commit", "--quiet", "--allow-empty", "-m", "one more"], folder).exitCode).toBe(0);
    expect(version(folder, "HEAD~3").printed).toBe("0.0.1");
    expect(version(folder).printed).toBe("0.0.4");
  });

  test("a name that is no commit is refused", () => {
    const refused = version(folder, "no-such-branch");
    expect(refused).toEqual({ exitCode: 1, printed: "", said: "commit-version: no such commit" });
  });

  test("a shallow clone is refused, because its count is not the count of the history", () => {
    const shallow = join(ROOM, "shallow");
    expect(run(["git", "clone", "--quiet", "--depth", "1", `file://${folder}`, shallow], ROOM).exitCode).toBe(0);
    const refused = version(shallow);
    expect(refused.exitCode).toBe(1);
    expect(refused.printed).toBe("");
    expect(refused.said).toContain("shallow clone");
  });

  test("a folder that is in no repository is refused", () => {
    const refused = version(mkdtempSync(join(ROOM, "plain-")));
    expect(refused.exitCode).toBe(1);
    expect(refused.printed).toBe("");
  });
});
