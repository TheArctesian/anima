/**
 * The end of a merge into `dev`: first the merge commit, then the checks, on the commit.
 * The order has a reason. The mockup check (PLN-019) counts the layout files that are different
 * between the commit that a row records and HEAD. While a merge is not committed, HEAD is still
 * the old `dev`, so each layout file of the branch counts as a change after its own comparison.
 * On the merge commit only a change that `dev` brought to the same screen counts, which is the
 * case that needs a new comparison.
 */

function git(root: string, args: string[]): { ok: boolean; out: string } {
  const done = Bun.spawnSync(["git", ...args], { cwd: root, stdout: "pipe", stderr: "pipe" });
  return { ok: done.exitCode === 0, out: done.stdout.toString().trim() };
}

/** Puts the worktree back to `before`: the merge, committed or not, is gone and no file of it stays. */
export function undoMerge(root: string, before: string): void {
  git(root, ["merge", "--abort"]);
  git(root, ["reset", "--hard", "--quiet", before]);
  git(root, ["clean", "-fd", "--quiet"]);
}

/**
 * Commits the merge that is in progress in `root`, then runs `check` on the commit. `check`
 * returns the reason of a failure. With a failure the commit is removed (HEAD is `before` again)
 * and the reason is returned; with none the commit stays and undefined is returned.
 */
export function commitThenCheck(root: string, message: string, check: () => string | undefined): string | undefined {
  const before = git(root, ["rev-parse", "HEAD"]).out;
  const failure = git(root, ["add", "-A"]).ok && git(root, ["commit", "--quiet", "-m", message]).ok ? check() : "the merge commit failed";
  if (failure !== undefined) undoMerge(root, before);
  return failure;
}
