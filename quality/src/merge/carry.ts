/**
 * The carry-over of a train (R-NEW-traincarry): the gates passed for the commit `tested`, and `dev`
 * moved after it, in documents or by merges of branches that the train holds. The branches that
 * are not in the new `dev` yet are merged again on it, and that result goes to `dev` only when
 * each file that is not a document is the same as in `tested`. So the code that lands is the code
 * that the gates tested, however `dev` moved. I/O: git in `root`.
 */
import { undoMerge } from "./commit.ts";
import { stack } from "./stack.ts";
import type { Tip } from "./train.ts";

function git(root: string, args: string[]): { ok: boolean; out: string } {
  const done = Bun.spawnSync(["git", ...args], { cwd: root, stdout: "pipe", stderr: "pipe" });
  return { ok: done.exitCode === 0, out: done.stdout.toString().trim() };
}

/** The paths that are different between two commits. */
export function changed(root: string, from: string, to: string): string[] {
  return git(root, ["diff", "--name-only", from, to]).out.split("\n").filter((path) => path !== "");
}

/** How one branch is merged, and which path is a document. */
export interface Rules {
  /** Ends one merge that is in progress (it commits); returns the reason of a failure. */
  finish: (tip: Tip) => string | undefined;
  isDocument: (path: string) => boolean;
}

/** A train whose gates passed: the branch that it lands in, the commit that the gates tested, and its branches in their order. */
export interface Passed {
  into: string;
  tested: string;
  held: Tip[];
}

/** The result of a carry-over, and for "different" what refused it: each branch that did not merge again and each path that is not the one of `tested`. */
export interface Carried {
  outcome: "landed" | "again" | "different";
  refused: string[];
}

/**
 * "landed": `into` on origin is the train made again. "again": the result was right, but
 * `into` moved once more in that moment; nothing is pushed and a later call can try again.
 * "different": a branch did not merge again, or a file that is not a document is not the one
 * of `tested`; nothing is pushed. In each case but "landed" the worktree is back at its start.
 */
export function carry(root: string, { into, tested, held }: Passed, rules: Rules): Carried {
  const start = git(root, ["rev-parse", "HEAD"]).out;
  // A branch that a person merged into `dev` while the gates ran is there already.
  const waiting = held.filter((tip) => !git(root, ["merge-base", "--is-ancestor", tip.sha, "HEAD"]).ok);
  const made = stack(root, waiting, rules.finish);
  const refused = [
    ...made.leftOut.map((each) => `${each.branch}: ${each.reason}`),
    ...changed(root, tested, "HEAD").filter((path) => !rules.isDocument(path)),
  ];
  if (refused.length === 0 && git(root, ["push", "--quiet", "origin", `HEAD:refs/heads/${into}`]).ok) return { outcome: "landed", refused };
  undoMerge(root, start);
  return { outcome: refused.length === 0 ? "again" : "different", refused };
}
