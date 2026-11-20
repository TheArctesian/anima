import { spawnSync } from "node:child_process";

/**
 * The number of layout files (Svelte or CSS) below a screen folder that are different between
 * `commit` and HEAD, or undefined when the repository does not have the commit. The folder is a
 * literal path: `[id]` in a route is a name, not a pattern. HEAD is the commit, not the files of
 * the worktree: a merge that is not committed is not in it (see `src/merge/commit.ts`).
 */
export function layoutChangedSince(root: string, screen: string, commit: string): number | undefined {
  const changed = spawnSync("git", ["diff", "--name-only", `${commit}..HEAD`, "--", `:(literal)${screen}`], { cwd: root, encoding: "utf8" });
  if (changed.status !== 0) return undefined;
  return changed.stdout.split("\n").filter((path) => path.endsWith(".svelte") || path.endsWith(".css")).length;
}
