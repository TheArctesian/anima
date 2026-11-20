/**
 * The end of one merge into `dev` in this repository: the generated files are written again,
 * each placeholder gets its number (PLN-021), the merge is committed and the document gates run
 * on the commit. I/O: it runs the programs of this repository in `root`.
 */
import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { commitThenCheck } from "./commit.ts";
import { GENERATED, mergeMessage } from "./rules.ts";

const CHECKS = [["bun", "run", "quality:documents"], ["bun", "quality/bin/numbers.ts", "--check"]];

/** The result of a merge that is finished: the numbers that it gave, or the reason of the failure. */
export type Finished = { numbers: [string, string][] } | { failure: string };

/** Finishes the merge of `source` that is in progress in `root`. With a failure nothing is committed; the caller puts the worktree back. */
export function finishMerge(root: string, source: string, into: string): Finished {
  const run = (argv: string[]): { ok: boolean; out: string } => {
    const done = Bun.spawnSync(argv, { cwd: root, stdout: "pipe", stderr: "inherit" });
    return { ok: done.exitCode === 0, out: done.stdout.toString() };
  };
  // A generated file is never merged: take the one of `dev`, then write it again from the documents.
  for (const path of GENERATED) {
    const ours = run(["git", "show", `HEAD:${path}`]);
    if (!ours.ok) return { failure: `git show HEAD:${path} failed` };
    writeFileSync(join(root, path), ours.out);
  }
  if (!run(["bun", "install", "--frozen-lockfile"]).ok) return { failure: "`bun install --frozen-lockfile` failed" };
  const assigned = run(["bun", "quality/bin/numbers.ts", "--assign"]);
  if (!assigned.ok) return { failure: "`bun quality/bin/numbers.ts --assign` failed" };
  const numbers = assigned.out
    .split("\n")
    .map((line) => line.split(" -> "))
    .filter((pair): pair is [string, string] => pair.length === 2);
  if (!run(["bun", "run", "quality:register"]).ok) return { failure: "`bun run quality:register` failed" };
  const failed = commitThenCheck(root, mergeMessage(source, into, numbers), () => CHECKS.find((argv) => !run(argv).ok)?.join(" "));
  return failed === undefined ? { numbers } : { failure: `\`${failed}\` failed on the merge commit` };
}
