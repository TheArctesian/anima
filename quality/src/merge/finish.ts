/**
 * The end of one merge into `dev` in this repository: the generated files are written again,
 * each placeholder gets its number (PLN-021), the merge is committed and the document gates run
 * on the commit. I/O: it runs the programs of this repository in `root`.
 */
import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { commitThenCheck } from "./commit.ts";
import { settleMockups } from "./mockups.ts";
import { conflictsForAHand, GENERATED, mergeMessage } from "./rules.ts";

const CHECKS = [["bun", "run", "quality:documents"], ["bun", "quality/bin/numbers.ts", "--check"]];

/** The result of a merge that is finished: the numbers that it gave, or the reason of the failure. */
export type Finished = { numbers: [string, string][] } | { failure: string };

/** Finishes the merge of `source` that is in progress in `root`. With a failure nothing is committed; the caller puts the worktree back. */
export function finishMerge(root: string, source: string, into: string): Finished {
  let said = "";
  const run = (argv: string[]): { ok: boolean; out: string } => {
    const done = Bun.spawnSync(argv, { cwd: root, stdout: "pipe", stderr: "pipe" });
    said = done.stderr.toString();
    process.stderr.write(said);
    return { ok: done.exitCode === 0, out: done.stdout.toString() };
  };
  // The records of the mockups keep the newer comparison, and the index takes the generated form (PLN-019).
  const settled = settleMockups(root);
  if (settled.conflict !== undefined) return { failure: settled.conflict };
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
  return failed === undefined ? { numbers } : { failure: `\`${failed}\` failed on the merge commit${firstFault(said)}` };
}

/** The first line that a failed check printed, so that the reason of a train names the fault; empty when it printed none. */
export function firstFault(said: string): string {
  // `bun run` first prints the command that it runs, after "$ ".
  const line = said.split("\n").map((each) => each.trim()).find((each) => each !== "" && !each.startsWith("$ "));
  return line === undefined ? "" : `: ${line.length > 300 ? `${line.slice(0, 300)}...` : line}`;
}

/**
 * Settles a stopped merge in `root` whose only conflicts are in generated files, the index of the
 * mockups and their records (the merge of a branch with `dev` in a run of the gates). The
 * generated files take the side of HEAD; `bun quality/bin/register.ts` then writes them again.
 * Returns the reason that a person must solve it, or undefined. I/O: git in `root`.
 */
export function settleGenerated(root: string): string | undefined {
  const git = (args: string[]) => Bun.spawnSync(["git", ...args], { cwd: root, stdout: "pipe", stderr: "pipe" });
  const conflicted = git(["diff", "--name-only", "--diff-filter=U"]).stdout.toString().split("\n").filter((path) => path !== "");
  const forAHand = conflictsForAHand(conflicted);
  if (forAHand.length > 0) return `a conflict in ${forAHand.join(", ")}`;
  const settled = settleMockups(root);
  if (settled.conflict !== undefined) return settled.conflict;
  for (const path of GENERATED.filter((each) => conflicted.includes(each))) {
    if (git(["checkout", "--ours", "--", path]).exitCode !== 0 || git(["add", "--", path]).exitCode !== 0) return `${path} could not take the side of HEAD`;
  }
  return undefined;
}
