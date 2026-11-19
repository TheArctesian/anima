// deploy/bin/affected-applications and deploy/bin/release-decision: which push to `main` is a
// release of the desktop application (R-184, R-197, R-199, R-200). Each repository here is made
// in a temporary folder; the commands under test are the files of this repository, and the map
// is deploy/release/applications.txt.
import { afterAll, describe, expect, test } from "bun:test";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";

const BIN = join(import.meta.dir, "..", "bin");
const ROOM = mkdtempSync(join(tmpdir(), "release-decision-"));
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

interface Ran {
  exitCode: number;
  printed: string;
  said: string;
}

function run(program: string[], folder: string): Ran {
  const result = Bun.spawnSync(program, { cwd: folder, env: GIT_ENV, stdout: "pipe", stderr: "pipe" });
  return { exitCode: result.exitCode, printed: result.stdout.toString().trim(), said: result.stderr.toString().trim() };
}

function git(folder: string, ...given: string[]): string {
  const ran = run(["git", ...given], folder);
  expect(ran).toMatchObject({ exitCode: 0 });
  return ran.printed;
}

/** A commit on the branch that is checked out that changes `paths`; it gives the commit. */
function change(folder: string, ...paths: string[]): string {
  for (const path of paths) {
    mkdirSync(dirname(join(folder, path)), { recursive: true });
    writeFileSync(join(folder, path), `${String(Math.random())}\n`);
  }
  git(folder, "add", "--all");
  git(folder, "commit", "--quiet", "--allow-empty", "-m", `change ${paths.join(" ")}`);
  return git(folder, "rev-parse", "HEAD");
}

function repository(): string {
  const folder = mkdtempSync(join(ROOM, "repository-"));
  git(folder, "init", "--quiet", "--initial-branch=main");
  change(folder, "apps/client/src/first.ts");
  return folder;
}

const affected = (folder: string, from: string, to: string): Ran => run(["/bin/sh", join(BIN, "affected-applications"), from, to], folder);
const decision = (folder: string, ...given: string[]): Ran => run(["/bin/sh", join(BIN, "release-decision"), ...given], folder);

describe("which change affects the desktop application (R-184, R-199)", () => {
  const folder = repository();
  const start = git(folder, "rev-parse", "HEAD");

  test.each([
    ["a document", ["docs/plans/a-plan.md"]],
    ["the rules for agents and the README", ["AGENTS.md", "README.md"]],
    ["the quality gates", ["quality/bin/limits.ts"]],
    ["the files of the server", ["deploy/compose.yaml", "deploy/test/some.test.ts"]],
    ["the test device, which no application carries", ["crates/rdp-test-device/src/main.rs"]],
  ])("%s releases nothing", (_name, paths) => {
    const to = change(folder, ...paths);
    expect(affected(folder, start, to)).toEqual({ exitCode: 0, printed: "", said: "" });
  });

  test.each([
    ["the client", ["apps/client/src/lib/devices/list.ts"]],
    ["only a test inside the client (R-199)", ["apps/client/src/lib/devices/list.test.ts"]],
    ["the shell", ["apps/desktop/src-tauri/src/desktop.rs"]],
    ["a program that the application carries", ["crates/anima-cli/src/main.rs"]],
    ["the link", ["link/tunnel/tunnel.go"]],
    ["the lock file", ["Cargo.lock"]],
    ["the release workflow", [".github/workflows/release.yml"]],
    ["the version command, which is below a folder that is out", ["deploy/bin/commit-version"]],
    ["the map itself", ["deploy/release/applications.txt"]],
    ["a folder that the map does not know", ["newthing/file.txt"]],
    ["a document inside the client, which the client can show", ["apps/client/static/help.md"]],
  ])("%s releases it", (_name, paths) => {
    const from = git(folder, "rev-parse", "HEAD");
    const to = change(folder, ...paths);
    expect(affected(folder, from, to)).toEqual({ exitCode: 0, printed: "desktop", said: "" });
  });

  test("one path of the application among documents is enough", () => {
    const from = git(folder, "rev-parse", "HEAD");
    const to = change(folder, "docs/a.md", "docs/b.md", "apps/client/src/second.ts");
    expect(affected(folder, from, to).printed).toBe("desktop");
  });

  test("a file that moves from the documents into the application releases it", () => {
    change(folder, "docs/moved.md");
    const from = git(folder, "rev-parse", "HEAD");
    git(folder, "mv", "docs/moved.md", "apps/client/src/moved.md");
    git(folder, "commit", "--quiet", "-m", "move");
    expect(affected(folder, from, "HEAD").printed).toBe("desktop");
  });

  test("a name that is no commit is refused, with no answer", () => {
    expect(affected(folder, "no-such", "HEAD")).toMatchObject({ exitCode: 1, printed: "" });
  });
});

describe("is this commit of main a release (R-184, R-200)", () => {
  test("the first release: a feed that does not exist releases, also after a change of documents only", () => {
    const folder = repository();
    change(folder, "docs/a.md");
    expect(decision(folder, "HEAD", "desktop", "none", "none").printed).toBe("release 0.0.2");
    expect(decision(folder, "HEAD", "desktop", "0.0.1", "none").printed).toBe("release 0.0.2");
  });

  test("documents after the published version release nothing; a later change of the application releases", () => {
    const folder = repository();
    change(folder, "docs/a.md");
    change(folder, "docs/b.md");
    expect(decision(folder, "HEAD", "desktop", "0.0.1", "0.0.1")).toEqual({
      exitCode: 0,
      printed: "nothing: no change of desktop after the published 0.0.1",
      said: "",
    });
    change(folder, "apps/client/src/second.ts");
    expect(decision(folder, "HEAD", "desktop", "0.0.1", "0.0.1").printed).toBe("release 0.0.4");
  });

  test("a change is not lost when the run of its own push never published", () => {
    const folder = repository();
    change(folder, "apps/client/src/second.ts");
    const before = change(folder, "docs/only-a-document.md");
    const pushed = change(folder, "docs/one-more.md");
    expect(decision(folder, "--before", before, pushed, "desktop", "0.0.1", "0.0.1").printed).toBe("release 0.0.4");
  });

  test("the feed that is behind decides: one target that missed a release gets it", () => {
    const folder = repository();
    change(folder, "apps/client/src/second.ts");
    change(folder, "docs/a.md");
    expect(decision(folder, "HEAD", "desktop", "0.0.2", "0.0.1").printed).toBe("release 0.0.3");
    expect(decision(folder, "HEAD", "desktop", "0.0.2", "0.0.2").printed).toBe("nothing: no change of desktop after the published 0.0.2");
  });

  test("BLD-002 a commit whose version is not more than a published version releases nothing", () => {
    const folder = repository();
    change(folder, "apps/client/src/second.ts");
    const old = git(folder, "rev-parse", "HEAD");
    change(folder, "apps/client/src/third.ts");
    expect(decision(folder, old, "desktop", "0.0.3", "0.0.3").printed).toBe("nothing: the version 0.0.2 of this commit is not more than the published 0.0.3");
    expect(decision(folder, old, "desktop", "0.0.2", "none").printed).toBe("nothing: the version 0.0.2 of this commit is not more than the published 0.0.2");
    expect(decision(folder, "HEAD", "desktop", "1.0.0").printed).toBe("nothing: the version 0.0.3 of this commit is not more than the published 1.0.0");
  });

  test("the published 0.2.1 of the old numbers is below a first line of 800 commits", () => {
    const folder = repository();
    for (let made = 2; made <= 205; made += 1) git(folder, "commit", "--quiet", "--allow-empty", "-m", `commit ${String(made)}`);
    change(folder, "apps/client/src/second.ts");
    expect(decision(folder, "HEAD", "desktop", "0.2.1", "0.2.1", "none").printed).toBe("release 0.2.6");
    expect(decision(folder, "HEAD", "desktop", "0.2.1", "0.2.1").printed).toBe("release 0.2.6");
  });

  test("R-200 dev comes into main by a merge commit: one release, with the next number of main", () => {
    const folder = repository();
    const tip = git(folder, "rev-parse", "HEAD");
    git(folder, "switch", "--quiet", "-c", "dev");
    change(folder, "apps/client/src/a.ts");
    change(folder, "apps/client/src/b.ts");
    change(folder, "docs/c.md");
    git(folder, "switch", "--quiet", "main");
    git(folder, "merge", "--quiet", "--no-ff", "-m", "dev into main", "dev");
    expect(decision(folder, "--before", tip, "HEAD", "desktop", "0.0.1", "0.0.1").printed).toBe("release 0.0.2");
  });

  test("R-200 a fast-forward of main to a branch is refused: the first line is then not the line of main", () => {
    const folder = repository();
    git(folder, "switch", "--quiet", "-c", "dev");
    change(folder, "apps/client/src/a.ts");
    git(folder, "switch", "--quiet", "main");
    const tip = change(folder, "docs/on-main.md");
    git(folder, "switch", "--quiet", "dev");
    git(folder, "merge", "--quiet", "--no-ff", "-m", "main into dev", "main");
    git(folder, "switch", "--quiet", "main");
    git(folder, "merge", "--quiet", "--ff-only", "dev");
    const refused = decision(folder, "--before", tip, "HEAD", "desktop", "none");
    expect(refused).toMatchObject({ exitCode: 1, printed: "" });
    expect(refused.said).toContain("did not move by a commit whose first parent is its tip before");
  });

  test("a tip before the push that is not in the history is refused (a push by force)", () => {
    const folder = repository();
    const refused = decision(folder, "--before", "1234567890123456789012345678901234567890", "HEAD", "desktop", "none");
    expect(refused).toMatchObject({ exitCode: 1, printed: "" });
  });

  test("the first push of a branch gives zeros as the tip before, and is not refused for that", () => {
    const folder = repository();
    expect(decision(folder, "--before", "0".repeat(40), "HEAD", "desktop", "none").printed).toBe("release 0.0.1");
  });

  test.each([
    ["an application that the map does not have", ["HEAD", "phone", "none"]],
    ["a published version that is no version", ["HEAD", "desktop", "latest"]],
    ["no published version at all", ["HEAD", "desktop"]],
    ["a name that is no commit", ["no-such", "desktop", "none"]],
  ])("%s is refused, with no answer", (_name, given) => {
    expect(decision(repository(), ...given)).toMatchObject({ exitCode: 1, printed: "" });
  });
});
