import { describe, expect, test } from "bun:test";
import { mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { isDocument } from "../../src/gates/tier.ts";
import { carry, type Rules } from "../../src/merge/carry.ts";
import { commitThenCheck } from "../../src/merge/commit.ts";
import { stack } from "../../src/merge/stack.ts";
import type { Tip } from "../../src/merge/train.ts";

/** These tests make repositories with many git commands; a busy machine needs more than the 5 seconds of a test. */
const SLOW = 60_000;

function git(root: string, ...args: string[]): string {
  const done = Bun.spawnSync(["git", ...args], { cwd: root });
  if (done.exitCode !== 0) throw new Error(`git ${args.join(" ")}: ${done.stderr.toString()}`);
  return done.stdout.toString().trim();
}

function commit(root: string, path: string, text: string): string {
  mkdirSync(dirname(join(root, path)), { recursive: true });
  writeFileSync(join(root, path), text);
  git(root, "add", "-A");
  git(root, "commit", "--quiet", "-m", path);
  return git(root, "rev-parse", "HEAD");
}

function clone(origin: string, name: string): string {
  const root = join(dirname(origin), name);
  git(dirname(origin), "clone", "--quiet", origin, root);
  git(root, "config", "user.name", "test");
  git(root, "config", "user.email", "test@anima.invalid");
  return root;
}

/**
 * An origin with `dev` and two branches that passed, a conductor clone that holds the train
 * which the gates tested (the two branches merged on the first `dev`), and a second clone
 * that stands for the sessions which push to `dev` while the gates of the train run.
 */
function world(): { conductor: string; session: string; origin: string; tested: string; held: Tip[]; rules: Rules } {
  const folder = mkdtempSync(join(tmpdir(), "anima-carry-"));
  const origin = join(folder, "origin.git");
  git(folder, "init", "--quiet", "--bare", "--initial-branch=dev", origin);
  const session = clone(origin, "session");
  git(session, "switch", "--quiet", "-c", "dev");
  commit(session, "crates/hub/src/main.rs", "fn main() {}\n");
  commit(session, "docs/plans/hub.md", "# The hub\n");
  git(session, "push", "--quiet", "origin", "dev");
  const held = [["hub/queue", "crates/hub/src/queue.rs", "pub fn claim() {}\n"], ["docs/ruling", "docs/rulings.md", "- R-1\n"]].map(([branch = "", path = "", text = ""]) => {
    git(session, "switch", "--quiet", "-c", branch, "dev");
    const sha = commit(session, path, text);
    git(session, "push", "--quiet", "origin", branch);
    return { branch, sha };
  });
  git(session, "switch", "--quiet", "dev");
  const conductor = clone(origin, "conductor");
  const rules: Rules = { finish: (tip) => commitThenCheck(conductor, `Merge origin/${tip.branch} into dev\n`, () => undefined), isDocument };
  const before = git(conductor, "rev-parse", "HEAD");
  stack(conductor, held, rules.finish);
  const tested = git(conductor, "rev-parse", "HEAD");
  git(conductor, "reset", "--quiet", "--hard", before);
  return { conductor, session, origin, tested, held, rules };
}

/** A session pushes one file to `dev`; the conductor then stands on that `dev`, as at the start of its step. */
function devMoves(at: { conductor: string; session: string }, path: string, text: string): void {
  git(at.session, "pull", "--quiet", "origin", "dev");
  commit(at.session, path, text);
  git(at.session, "push", "--quiet", "origin", "dev");
  git(at.conductor, "pull", "--quiet", "origin", "dev");
}

describe("R-NEW-traincarry a train that passed lands over a dev that moved in documents or by its own branches", () => {
  test("it lands on the new dev with each branch, the pushed document, and the code that the gates tested", () => {
    const at = world();
    devMoves(at, "docs/ideas/spare-computer/idea.md", "# An idea\n");
    expect(carry(at.conductor, { into: "dev", tested: at.tested, held: at.held }, at.rules)).toEqual({ outcome: "landed", refused: [] });
    const dev = git(at.origin, "rev-parse", "dev");
    expect(git(at.origin, "ls-tree", "-r", "--name-only", dev).split("\n")).toEqual(
      expect.arrayContaining(["crates/hub/src/queue.rs", "docs/rulings.md", "docs/ideas/spare-computer/idea.md"]),
    );
    expect(git(at.conductor, "diff", "--name-only", at.tested, dev)).toBe("docs/ideas/spare-computer/idea.md");
    expect(git(at.origin, "rev-list", "--count", "--merges", dev)).toBe("2");
  }, SLOW);

  test("a dev that moved in code is refused, nothing is pushed, and the worktree is as before", () => {
    const at = world();
    devMoves(at, "crates/hub/src/lease.rs", "pub fn renew() {}\n");
    const before = git(at.origin, "rev-parse", "dev");
    expect(carry(at.conductor, { into: "dev", tested: at.tested, held: at.held }, at.rules)).toEqual({ outcome: "different", refused: ["crates/hub/src/lease.rs"] });
    expect(git(at.origin, "rev-parse", "dev")).toBe(before);
    expect(git(at.conductor, "rev-parse", "HEAD")).toBe(before);
    expect(git(at.conductor, "status", "--porcelain")).toBe("");
  }, SLOW);

  test("a document of dev that a branch of the train also changed is refused: the branch does not merge again", () => {
    const at = world();
    devMoves(at, "docs/rulings.md", "- R-2\n");
    const before = git(at.origin, "rev-parse", "dev");
    const carried = carry(at.conductor, { into: "dev", tested: at.tested, held: at.held }, at.rules);
    expect(carried.outcome).toBe("different");
    expect(carried.refused).toEqual(["docs/ruling: a conflict in docs/rulings.md"]);
    expect(git(at.origin, "rev-parse", "dev")).toBe(before);
  }, SLOW);

  test("a dev that moves once more in that moment is not overwritten: the answer is to try again", () => {
    const at = world();
    devMoves(at, "docs/ideas/one/idea.md", "# One\n");
    commit(at.session, "docs/ideas/two/idea.md", "# Two\n");
    git(at.session, "push", "--quiet", "origin", "dev");
    const pushed = git(at.origin, "rev-parse", "dev");
    expect(carry(at.conductor, { into: "dev", tested: at.tested, held: at.held }, at.rules)).toEqual({ outcome: "again", refused: [] });
    expect(git(at.origin, "rev-parse", "dev")).toBe(pushed);
    expect(git(at.conductor, "status", "--porcelain")).toBe("");
  }, SLOW);
});

describe("R-NEW-traincarry a train that passed lands over a dev that a person merged one of its branches into", () => {
  test("a dev that moved by a merge by hand of a branch that the train holds lands: the code is the one tested", () => {
    const at = world();
    git(at.session, "pull", "--quiet", "origin", "dev");
    git(at.session, "merge", "--quiet", "--no-ff", "-m", "Merge origin/hub/queue into dev", "hub/queue");
    git(at.session, "push", "--quiet", "origin", "dev");
    git(at.conductor, "pull", "--quiet", "origin", "dev");
    expect(carry(at.conductor, { into: "dev", tested: at.tested, held: at.held }, at.rules)).toEqual({ outcome: "landed", refused: [] });
    const dev = git(at.origin, "rev-parse", "dev");
    expect(git(at.conductor, "diff", "--name-only", at.tested, dev)).toBe("");
    expect(git(at.origin, "rev-list", "--count", "--merges", dev)).toBe("2");
  }, SLOW);

  test("a merge by hand of a held branch together with other code is refused, and nothing is pushed", () => {
    const at = world();
    git(at.session, "pull", "--quiet", "origin", "dev");
    git(at.session, "merge", "--quiet", "--no-ff", "-m", "Merge origin/hub/queue into dev", "hub/queue");
    git(at.session, "push", "--quiet", "origin", "dev");
    devMoves(at, "crates/hub/src/lease.rs", "pub fn renew() {}\n");
    const before = git(at.origin, "rev-parse", "dev");
    expect(carry(at.conductor, { into: "dev", tested: at.tested, held: at.held }, at.rules)).toEqual({ outcome: "different", refused: ["crates/hub/src/lease.rs"] });
    expect(git(at.origin, "rev-parse", "dev")).toBe(before);
  }, SLOW);
});
