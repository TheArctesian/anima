// The conductor of the merge train (R-252): `bun quality/bin/train.ts`, in a clean checkout that
// is on `dev`, with `gh` signed in. The workflow `merge-to-dev` runs it; a session can run it too.
// `--plan` only prints. One call does one step and ends: it settles the train that exists (waits, lands it, splits
// it), and if no train is on the way it builds the next one and starts its gates. It moves `dev`
// only forward, to a commit whose gates passed, and never touches `main`.
import { appendFileSync } from "node:fs";
import { finishMerge } from "../src/merge/finish.ts";
import { members, stack } from "../src/merge/stack.ts";
import { batch, firstHalf, TRAIN, trainName, verdict, type GatesRun, type Tip } from "../src/merge/train.ts";
import { REPO_ROOT } from "../src/repository.ts";

const INTO = "dev";
const NOTES = "refs/notes/train";
/** A run that was just started is not in the list of GitHub at once: a train this young waits. */
const YOUNG_SECONDS = 180;

function run(argv: string[]): { ok: boolean; out: string } {
  const done = Bun.spawnSync(argv, { cwd: REPO_ROOT, stdout: "pipe", stderr: "pipe" });
  return { ok: done.exitCode === 0, out: done.stdout.toString().trim() };
}

function say(text: string): void {
  console.log(`train: ${text}`);
  const summary = process.env["GITHUB_STEP_SUMMARY"];
  if (summary !== undefined) appendFileSync(summary, `${text}\n\n`);
}

function must(argv: string[]): string {
  const done = run(argv);
  if (!done.ok) {
    console.error(`train: \`${argv.join(" ")}\` failed`);
    process.exit(4);
  }
  return done.out;
}

function tips(): Tip[] {
  const lines = must(["git", "for-each-ref", "--format=%(refname:lstrip=3)%09%(objectname)", "refs/remotes/origin"]).split("\n");
  return lines.map((line) => line.split("\t")).flatMap(([branch, sha]) => (branch === undefined || sha === undefined ? [] : [{ branch, sha }]));
}

function gatesRuns(): GatesRun[] {
  const fields = "databaseId,headBranch,headSha,event,status,conclusion,updatedAt";
  return JSON.parse(must(["gh", "run", "list", "--workflow", "gates.yml", "--limit", "400", "--json", fields])) as GatesRun[];
}

/** The tips that failed alone in a train, with the time of that failure (the first line of the note). */
function refused(): Map<string, string> {
  run(["git", "fetch", "--quiet", "origin", `+${NOTES}:${NOTES}`]);
  const noted = run(["git", "notes", "--ref", NOTES, "list"]).out.split("\n").map((line) => line.split(" ")[1] ?? "");
  return new Map(noted.filter((sha) => sha !== "").map((sha) => [sha, run(["git", "notes", "--ref", NOTES, "show", sha]).out.split("\n")[0] ?? ""]));
}

function refuse(tip: Tip, now: Date): void {
  const text = `${now.toISOString()}\nThe gates failed for a train that held only ${tip.branch}, merged with dev.`;
  must(["git", "notes", "--ref", NOTES, "add", "--force", "-m", text, tip.sha]);
  must(["git", "push", "--quiet", "origin", NOTES]);
  say(`${tip.branch} (${tip.sha.slice(0, 8)}) failed alone as merged with dev. It stays out until it moves or its own gates pass again.`);
}

function remove(train: Tip): void {
  run(["git", "push", "--quiet", "origin", `:refs/heads/${train.branch}`]);
}

function inDev(sha: string): boolean {
  return run(["git", "merge-base", "--is-ancestor", sha, `origin/${INTO}`]).ok;
}

/** Moves `dev` to the commit of the train when `dev` is still the commit that the train stands on. */
function land(train: Tip, base: string): void {
  const held = members(REPO_ROOT, base, train.sha).map((tip) => tip.branch);
  const moved = base !== must(["git", "rev-parse", `origin/${INTO}`]);
  if (!moved && run(["git", "push", "--quiet", "origin", `${train.sha}:refs/heads/${INTO}`]).ok) {
    must(["git", "merge", "--quiet", "--ff-only", train.sha]);
    say(`landed ${held.length} branch(es) in one step, ${INTO} is ${train.sha.slice(0, 8)}: ${held.join(", ")}`);
  } else say(`${INTO} moved after the train was made; the gates of the train do not count. The branches go into the next train: ${held.join(", ")}`);
  remove(train);
}

/**
 * Settles a train that exists. Returns "wait" when it is still on the way, the branches that the
 * next train must hold after a failure, or undefined when the next train is built by the rule.
 */
function settle(train: Tip, runs: GatesRun[], now: Date): Tip[] | "wait" | undefined {
  const base = must(["git", "merge-base", train.sha, `origin/${INTO}`]);
  const stale = base !== must(["git", "rev-parse", `origin/${INTO}`]);
  const what = verdict(train.sha, runs);
  if (what === "land") return void land(train, base);
  if (what === "wait" && !stale) return "wait";
  if (what === "start-gates" && !stale) {
    const age = now.getTime() / 1000 - Number(must(["git", "log", "-1", "--format=%ct", train.sha]));
    if (age > YOUNG_SECONDS) must(["gh", "workflow", "run", "gates.yml", "--ref", train.branch]);
    return "wait";
  }
  const held = members(REPO_ROOT, base, train.sha);
  for (const each of runs.filter((one) => one.headSha === train.sha && one.status !== "completed")) run(["gh", "run", "cancel", String(each.databaseId)]);
  remove(train);
  if (what !== "split") return void say(`the train ${train.branch} is given up (${stale ? `${INTO} moved` : "its gates were cancelled"}); its branches go into the next train`);
  const [only] = held;
  if (held.length === 1 && only !== undefined) return void refuse(only, now);
  say(`the gates failed for a train of ${held.length} branches; the next train holds the first half`);
  return firstHalf(held);
}

/** Builds a train of `wanted` on the local `dev`, pushes it as a train branch and starts its gates. */
function build(wanted: Tip[], now: Date): void {
  if (wanted.length === 0) return say("no branch waits: each is in dev, or its own gates did not pass yet");
  const base = must(["git", "rev-parse", "HEAD"]);
  const made = stack(REPO_ROOT, wanted, (tip) => {
    const finished = finishMerge(REPO_ROOT, `origin/${tip.branch}`, INTO);
    return "failure" in finished ? finished.failure : undefined;
  });
  for (const each of made.leftOut) say(`${each.branch} is left out: ${each.reason}. Merge origin/${INTO} into the branch and solve it there.`);
  if (made.merged.length === 0) return say("no branch of the batch merges with dev: no train");
  const name = trainName(base, now);
  must(["git", "push", "--quiet", "origin", `HEAD:refs/heads/${name}`]);
  must(["gh", "workflow", "run", "gates.yml", "--ref", name]);
  say(`the train ${name} holds ${made.merged.length} branch(es) on ${INTO} ${base.slice(0, 8)}, and its gates are started: ${made.merged.map((tip) => tip.branch).join(", ")}`);
}

const now = new Date();
// `--plan` changes nothing, in any worktree: it prints each train with its verdict and the batch of the next train.
if (process.argv.includes("--plan")) {
  must(["git", "fetch", "--quiet", "--prune", "origin"]);
  const listed = gatesRuns();
  for (const train of tips().filter((tip) => tip.branch.startsWith(TRAIN))) say(`${train.branch}: ${verdict(train.sha, listed)}`);
  say(`the next train would hold: ${batch(tips(), listed, refused(), inDev).map((tip) => tip.branch).join(", ") || "nothing"}`);
  process.exit(0);
}
if (must(["git", "rev-parse", "--abbrev-ref", "HEAD"]) !== INTO) must(["git", "switch", "--quiet", INTO]);
if (must(["git", "status", "--porcelain"]) !== "") {
  console.error("train: this worktree has changes; use a clean worktree");
  process.exit(2);
}
must(["git", "fetch", "--quiet", "--prune", "origin"]);
must(["git", "merge", "--quiet", "--ff-only", `origin/${INTO}`]);
const runs = gatesRuns();
let next: Tip[] | undefined;
for (const train of tips().filter((tip) => tip.branch.startsWith(TRAIN))) {
  const settled = settle(train, runs, now);
  if (settled === "wait") {
    say(`the train ${train.branch} is on the way; nothing to do until its gates end`);
    process.exit(0);
  }
  next ??= settled;
}
const current = new Map(tips().map((tip) => [tip.branch, tip.sha]));
const still = next?.filter((tip) => current.get(tip.branch) === tip.sha && !inDev(tip.sha));
build(still !== undefined && still.length > 0 ? still : batch(tips(), runs, refused(), inDev), now);
