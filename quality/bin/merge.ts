// Merges one branch into `dev`: `bun quality/bin/merge.ts <branch> [--push]`, in a clean worktree
// that is on `dev`. It writes the generated files again in place of a merge of them, gives each
// placeholder its number (PLN-021), makes the merge commit and runs the document gates on that
// commit (`src/merge/commit.ts` says why in that order); if they fail, the commit is removed.
// Without `--push` it pushes nothing and prints the command. It never pushes to `main`.
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { undoMerge } from "../src/merge/commit.ts";
import { finishMerge } from "../src/merge/finish.ts";
import { conflictsForAHand, gatesRefusal, GENERATED, parseMergeRule, type CheckRun } from "../src/merge/rules.ts";
import { REPO_ROOT } from "../src/repository.ts";

const INTO = "dev";

function run(argv: string[], quiet = false): { ok: boolean; out: string } {
  const done = Bun.spawnSync(argv, { cwd: REPO_ROOT, stdout: "pipe", stderr: quiet ? "pipe" : "inherit" });
  return { ok: done.exitCode === 0, out: done.stdout.toString() };
}

function stop(code: number, message: string): never {
  console.error(`merge: ${message}`);
  process.exit(code);
}

/** Runs a step of the merge; a step that fails undoes the merge and ends the program. */
function step(argv: string[]): string {
  const done = run(argv);
  if (!done.ok) {
    undoMerge(REPO_ROOT, before);
    stop(4, `\`${argv.join(" ")}\` failed; the merge is undone and nothing is changed`);
  }
  return done.out;
}

/** With the rule `fullGates: github`, the gates of the tip of the branch must have passed on GitHub. */
function requireGatesOnGitHub(commit: string): void {
  const repository = process.env["GITHUB_REPOSITORY"] ?? run(["gh", "repo", "view", "--json", "nameWithOwner", "--jq", ".nameWithOwner"], true).out.trim();
  const answer = run(["gh", "api", `repos/${repository}/commits/${commit}/check-runs`, "--jq", ".check_runs"], true);
  if (!answer.ok) stop(5, "the check runs of the commit cannot be read (is `gh` signed in?)");
  const refusal = gatesRefusal(JSON.parse(answer.out) as CheckRun[]);
  if (refusal !== undefined) stop(5, `${refusal}; the rule is fullGates "github" (quality/merge-rule.json)`);
}

const branch = process.argv.slice(2).find((each) => !each.startsWith("--"));
const push = process.argv.includes("--push");
if (branch === undefined) stop(2, "usage: bun quality/bin/merge.ts <branch> [--push]");
const rule = parseMergeRule(readFileSync(join(REPO_ROOT, "quality", "merge-rule.json"), "utf8"));
if (typeof rule === "string") stop(2, `quality/merge-rule.json: ${rule}`);

if (run(["git", "rev-parse", "--abbrev-ref", "HEAD"]).out.trim() !== INTO) stop(2, `this worktree is not on \`${INTO}\``);
if (run(["git", "status", "--porcelain"]).out.trim() !== "") stop(2, "this worktree has changes; use a clean worktree");
if (!run(["git", "fetch", "origin"]).ok) stop(2, "git fetch failed");
if (!run(["git", "merge", "--ff-only", `origin/${INTO}`]).ok) stop(2, `local \`${INTO}\` is not behind \`origin/${INTO}\`; it holds commits that are not pushed`);
const source = `origin/${branch.replace(/^origin\//, "")}`;
const tip = run(["git", "rev-parse", "--verify", `${source}^{commit}`], true);
if (!tip.ok) stop(2, `${source} is not a branch on origin`);
if (rule.fullGates === "github") requireGatesOnGitHub(tip.out.trim());

const before = run(["git", "rev-parse", "HEAD"]).out.trim();
run(["git", "merge", "--no-ff", "--no-commit", source], true);
const conflicted = run(["git", "diff", "--name-only", "--diff-filter=U"]).out.split("\n").filter((path) => path !== "");
const forAHand = conflictsForAHand(conflicted);
if (forAHand.length > 0) {
  run(["git", "merge", "--abort"], true);
  stop(3, `a conflict that the script does not solve:\n${forAHand.join("\n")}\nMerge origin/${INTO} into the branch, solve it there, and run this again.`);
}
const finished = finishMerge(REPO_ROOT, source, INTO);
if ("failure" in finished) {
  undoMerge(REPO_ROOT, before);
  stop(4, `${finished.failure}; the merge is undone and nothing is changed`);
}
const { numbers } = finished;
console.log(`merge: ${source} is merged into local ${INTO} at ${run(["git", "rev-parse", "--short", "HEAD"]).out.trim()}`);
for (const [placeholder, number] of numbers) console.log(`merge: ${placeholder} is ${number}`);
console.log(`merge: the generated files are written again: ${GENERATED.join(", ")}`);
if (push) step(["git", "push", "origin", `HEAD:refs/heads/${INTO}`]);
else console.log(`merge: not pushed. To push: git push origin HEAD:refs/heads/${INTO}`);
