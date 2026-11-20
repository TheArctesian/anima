import { describe, expect, test } from "bun:test";
import { mkdirSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { checkMockups, MOCKUPS } from "../../src/documents/mockups.ts";
import { layoutChangedSince } from "../../src/documents/screens.ts";
import { commitThenCheck } from "../../src/merge/commit.ts";

const SCREEN = "apps/client/src/lib/containers";
const PAGE = `${SCREEN}/Containers.svelte`;

function git(root: string, ...args: string[]): string {
  const done = Bun.spawnSync(["git", "-c", "user.name=test", "-c", "user.email=test@anima.invalid", ...args], { cwd: root });
  if (done.exitCode !== 0) throw new Error(`git ${args.join(" ")}: ${done.stderr.toString()}`);
  return done.stdout.toString().trim();
}

function commit(root: string, path: string, text: string): string {
  mkdirSync(dirname(join(root, path)), { recursive: true });
  writeFileSync(join(root, path), text);
  git(root, "add", "-A");
  git(root, "commit", "--quiet", "-m", path);
  return git(root, "rev-parse", "--short=8", "HEAD");
}

function index(compared: string): string {
  const row = `| [containers/index.html](containers/index.html) | plan | [containers](../../${SCREEN}) | Built | \`${compared}\` | none |`;
  return `# Mockups\n\n## Index\n\n| Mockup | Owner | Screens | State | Compared | Differences |\n| --- | --- | --- | --- | --- | --- |\n${row}\n`;
}

/** A repository on `dev`, with a branch that changed the screen and then recorded that commit as compared. */
function repository(): string {
  const root = mkdtempSync(join(tmpdir(), "anima-merge-"));
  git(root, "init", "--quiet", "--initial-branch=dev");
  git(root, "config", "user.name", "test");
  git(root, "config", "user.email", "test@anima.invalid");
  const first = commit(root, PAGE, "<p>one</p>\n");
  commit(root, MOCKUPS, index(first));
  git(root, "switch", "--quiet", "-c", "ui/containers");
  const built = commit(root, PAGE, "<p>two</p>\n");
  commit(root, MOCKUPS, index(built));
  git(root, "switch", "--quiet", "dev");
  return root;
}

/** The mockup check of the documents gate, on the files of the worktree and the commits of `root`. */
function mockupCheck(root: string): string | undefined {
  const text = readFileSync(join(root, MOCKUPS), "utf8");
  const found = checkMockups([{ path: MOCKUPS, text }], (screen, at) => layoutChangedSince(root, screen, at));
  return found[0]?.signal;
}

function startMerge(root: string): void {
  git(root, "merge", "--no-ff", "--no-commit", "ui/containers");
}

describe("PLN-019 the document gates run on the merge commit", () => {
  test("a merge that is not committed counts the files of the branch as changed after its own comparison", () => {
    const root = repository();
    startMerge(root);
    expect(mockupCheck(root)).toBe("screen-changed-after-mockup");
  }, 60_000);

  test("a branch that recorded its comparison merges, and the commit stays", () => {
    const root = repository();
    const before = git(root, "rev-parse", "HEAD");
    startMerge(root);
    expect(commitThenCheck(root, "Merge ui/containers into dev\n", () => mockupCheck(root))).toBeUndefined();
    expect(git(root, "rev-parse", "HEAD^1")).toBe(before);
    expect(git(root, "log", "-1", "--format=%s")).toBe("Merge ui/containers into dev");
  }, 60_000);

  test("a screen that dev also changed fails on the commit, and the commit is removed", () => {
    const root = repository();
    const before = commit(root, `${SCREEN}/Row.svelte`, "<li>row</li>\n");
    startMerge(root);
    const failure = commitThenCheck(root, "Merge ui/containers into dev\n", () => mockupCheck(root));
    expect(failure).toBe("screen-changed-after-mockup");
    expect(git(root, "rev-parse", "--short=8", "HEAD")).toBe(before);
    expect(git(root, "status", "--porcelain")).toBe("");
    expect(Bun.spawnSync(["git", "rev-parse", "--verify", "--quiet", "MERGE_HEAD"], { cwd: root }).exitCode).not.toBe(0);
  }, 60_000);
});
