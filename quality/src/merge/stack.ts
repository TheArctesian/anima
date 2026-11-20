/** The git work of a train: merges of branches one on the other in a worktree, and what a train holds. I/O. */
import { undoMerge } from "./commit.ts";
import { conflictsForAHand } from "./rules.ts";
import { mergedBranch, type Tip } from "./train.ts";

function git(root: string, args: string[]): { ok: boolean; out: string } {
  const done = Bun.spawnSync(["git", ...args], { cwd: root, stdout: "pipe", stderr: "pipe" });
  return { ok: done.exitCode === 0, out: done.stdout.toString().trim() };
}

/** A branch that a train left out, and why. */
export interface LeftOut {
  branch: string;
  reason: string;
}

/**
 * Merges each tip into the HEAD of `root`, in the order given. `finish` ends one merge that is
 * in progress (it commits) and returns the reason of a failure. A branch with a conflict for a
 * hand, or whose `finish` failed, is left out and the worktree is put back; the next one goes on.
 */
export function stack(root: string, tips: Tip[], finish: (tip: Tip) => string | undefined): { merged: Tip[]; leftOut: LeftOut[] } {
  const merged: Tip[] = [];
  const leftOut: LeftOut[] = [];
  for (const tip of tips) {
    const before = git(root, ["rev-parse", "HEAD"]).out;
    git(root, ["merge", "--no-ff", "--no-commit", tip.sha]);
    const conflicted = git(root, ["diff", "--name-only", "--diff-filter=U"]).out.split("\n").filter((path) => path !== "");
    const forAHand = conflictsForAHand(conflicted);
    const reason = forAHand.length > 0 ? `a conflict in ${forAHand.join(", ")}` : finish(tip);
    if (reason === undefined && git(root, ["rev-parse", "HEAD"]).out !== before) merged.push(tip);
    else {
      undoMerge(root, before);
      leftOut.push({ branch: tip.branch, reason: reason ?? "the merge made no commit" });
    }
  }
  return { merged, leftOut };
}

/** The branches that the commit `train` holds over `base`, in the order of their merges. */
export function members(root: string, base: string, train: string): Tip[] {
  const lines = git(root, ["log", "--first-parent", "--reverse", "--format=%P%x09%s", `${base}..${train}`]).out.split("\n");
  return lines.flatMap((line) => {
    const [parents = "", subject = ""] = line.split("\t");
    const branch = mergedBranch(subject);
    const sha = parents.split(" ")[1];
    return branch === undefined || sha === undefined ? [] : [{ branch, sha }];
  });
}
