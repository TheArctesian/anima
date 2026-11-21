import { existsSync } from "node:fs";
import { checkAlone } from "../src/gates/alone.ts";
import { cpus, loadavg } from "node:os";
import { compileCache } from "../src/gates/cache.ts";
import { formatGateEvent, gateList, GROUPS, keptGoing, missingRequirement, ofGroups, runGates, type Gate, type Group } from "../src/gates/gates.ts";
import { changedGates } from "../src/gates/changed.ts";
import { devPushJobs, EACH_JOB, jobLines, jobsFor } from "../src/gates/jobs.ts";
import { changedPaths, readLayout } from "../src/gates/layout.ts";
import { choose, describeChoice, fastGates, isBusy, macOnly } from "../src/gates/tier.ts";
import { REPO_ROOT } from "../src/repository.ts";

const USAGE = "usage: bun quality/bin/gates.ts [--fast [--whole] | --changed] [--group rust,go,bun,quality] [--gate name,name] [--keep-going]";
/** The last line of a fast run that left tests out. */
let leftOut: string | undefined;

/** The value of `--name value` in the arguments, when it is there. */
function option(name: string): string | undefined {
  const at = process.argv.indexOf(name);
  return at === -1 ? undefined : process.argv[at + 1];
}

function groupsAsked(): Group[] | undefined {
  const asked = option("--group")?.split(",");
  if (asked === undefined) return undefined;
  const unknown = asked.filter((each) => !(GROUPS as readonly string[]).includes(each));
  if (asked.length === 0 || unknown.length > 0) {
    console.error(`gates: no such group: ${unknown.join(", ")}\n${USAGE}`);
    process.exit(2);
  }
  return asked as Group[];
}

// `--alone`: each crate checked by itself (the job rust-lint on GitHub); it runs no gate of the list.
if (process.argv[2] === "--alone") {
  if (process.argv.length > 3) {
    console.error("gates: --alone takes no other argument (usage: bun quality/bin/gates.ts --alone)");
    process.exit(2);
  }
  const crates = readLayout(REPO_ROOT).crates.map((crate) => crate.name);
  const run = (argv: string[]) => Bun.spawnSync(argv, { cwd: REPO_ROOT, stdout: "inherit", stderr: "inherit" }).exitCode ?? 1;
  process.exit(checkAlone(crates, run, (line) => { console.log(line); }));
}

// `--jobs`: the jobs of the workflow `gates` that the changes against `FAST_BASE` need, as lines for
// `$GITHUB_OUTPUT` (the job `base`). With no base here it names each job.
if (process.argv[2] === "--jobs") {
  const paths = changedPaths(REPO_ROOT, process.env["FAST_BASE"] || "origin/dev");
  for (const line of jobLines(paths === undefined ? EACH_JOB : jobsFor(paths))) console.log(line);
  process.exit(0);
}

/** The tree of `commit` here, or undefined when the commit is not here. */
function treeOf(commit: string): string | undefined {
  const done = Bun.spawnSync(["git", "rev-parse", "--verify", "--quiet", `${commit}^{tree}`], { cwd: REPO_ROOT });
  return done.exitCode === 0 ? done.stdout.toString().trim() : undefined;
}

/** Each tree whose gates passed as it is: a run on `dev` or on a train (`gh` signed in), with the run. */
function provedTrees(): Map<string, number> {
  const fields = "databaseId,headBranch,headSha,status,conclusion";
  const listed = Bun.spawnSync(["gh", "run", "list", "--workflow", "gates.yml", "--limit", "400", "--json", fields], { cwd: REPO_ROOT });
  if (listed.exitCode !== 0) return new Map();
  const runs = JSON.parse(listed.stdout.toString()) as { databaseId: number; headBranch: string; headSha: string; status: string; conclusion: string | null }[];
  const asItIs = runs.filter((run) => (run.headBranch === "dev" || run.headBranch.startsWith("train/")) && run.status === "completed" && run.conclusion === "success");
  return new Map(asItIs.flatMap((run) => { const tree = treeOf(run.headSha); return tree === undefined ? [] : [[tree, run.databaseId] as const]; }));
}

// `--dev-push <before>`: the jobs of a push to `dev` from the commit `before` (devPushJobs), as
// lines for `$GITHUB_OUTPUT`; the reason goes to standard error.
if (process.argv[2] === "--dev-push") {
  const before = process.argv[3] ?? "";
  const tree = treeOf("HEAD") ?? "";
  const beforeTree = before === "" ? undefined : treeOf(before);
  const diff = Bun.spawnSync(["git", "diff", "--name-only", "-z", before, "HEAD"], { cwd: REPO_ROOT });
  const changed = beforeTree === undefined || diff.exitCode !== 0 ? undefined : diff.stdout.toString().split("\0").filter((path) => path !== "");
  const push = { tree, proved: provedTrees(), ...(beforeTree !== undefined && changed !== undefined ? { before: { tree: beforeTree, changed } } : {}) };
  const chosen = devPushJobs(push);
  console.error(`jobs of this push to dev: ${chosen.reason}`);
  for (const line of jobLines(chosen.jobs, chosen.provedBy)) console.log(line);
  process.exit(0);
}

/** The gates of the fast tier (`--fast`): chosen by the paths that differ from `FAST_BASE` (default `origin/dev`). */
function fastList(full: Gate[]): Gate[] {
  const base = process.env["FAST_BASE"] || "origin/dev";
  const paths = changedPaths(REPO_ROOT, base);
  if (paths === undefined) {
    console.log(`fast tier: full; ${base} is not a commit here (run \`git fetch origin\`)`);
    return full;
  }
  const layout = readLayout(REPO_ROOT);
  const choice = choose(paths, layout);
  console.log(describeChoice(choice));
  // R-268: a busy machine leaves the broad tests to the run on GitHub. `FAST_BUSY_LOAD` sets the limit.
  const load = loadavg()[0] ?? 0;
  const limit = Number(process.env["FAST_BUSY_LOAD"]) || cpus().length;
  if (choice.tier !== "code" || process.argv.includes("--whole") || !isBusy(load, limit)) return fastGates(choice, full);
  const mac = macOnly(paths);
  if (mac.length > 0) {
    console.log(`fast tier: the machine is busy (load ${load.toFixed(0)}, limit ${limit}), but no test is left out: only a Mac tests ${mac[0] ?? ""}${mac.length > 1 ? ` and ${mac.length - 1} more` : ""}`);
    return fastGates(choice, full);
  }
  const whole = fastGates(choice, full);
  const light = fastGates(choice, full, { paths, packages: layout.packages });
  const gone = whole.map((gate) => gate.name).filter((name) => !light.some((gate) => gate.name === name));
  const narrowed = light.filter((gate) => gate.name === "test" && !whole.some((each) => each.argv.join(" ") === gate.argv.join(" "))).map((gate) => `\`${gate.argv.join(" ")}\``);
  const what = [gone.length > 0 ? `left out ${gone.join(", ")}` : "", narrowed.length > 0 ? `the bun tests are only ${narrowed.join(" and ")}` : ""].filter((part) => part !== "");
  console.log(`fast tier: the machine is busy (load ${load.toFixed(0)}, limit ${limit}): ${what.join("; ") || "nothing is left out for this change"}. \`--whole\` runs each test of the tier`);
  if (what.length === 0) return light;
  leftOut = "fast tier: tests were left out on a busy machine. This run is not the gate: the run of the workflow `gates` on GitHub is (R-254)";
  return light;
}

/**
 * The checks of the changed paths (`--changed`): the documents gate, then the tests and lints of the
 * touched crates and packages. It never runs the full list; the run on GitHub is the gate.
 */
function changedList(full: Gate[]): Gate[] {
  const base = process.env["FAST_BASE"] || "origin/dev";
  const paths = changedPaths(REPO_ROOT, base);
  if (paths === undefined) {
    console.log(`changed: ${base} is not a commit here (run \`git fetch origin\`); only the documents gate runs`);
    return full.filter((gate) => gate.name === "documents");
  }
  const layout = readLayout(REPO_ROOT);
  const choice = choose(paths, layout);
  const run = changedGates(choice, full, layout, paths);
  console.log(describeChoice(choice).replace("fast tier", "changed paths"));
  for (const note of run.notes) console.log(`changed paths: ${note}`);
  leftOut = "changed paths: this run is not the gate. Push the branch: the workflow `gates` on GitHub runs the full list (R-254)";
  return run.gates;
}

const unknown = process.argv.slice(2).filter((each, at, all) => each.startsWith("--") ? !["--fast", "--whole", "--changed", "--group", "--gate", "--keep-going"].includes(each) : !["--group", "--gate"].includes(all[at - 1] ?? ""));
if (unknown.length > 0) {
  console.error(`gates: unknown argument: ${unknown.join(" ")}\n${USAGE}`);
  process.exit(2);
}

const baseRef = process.env["BASE_REF"] || "phase-2";
const cache = compileCache(process.env, existsSync);
console.log(cache.note);

const full = gateList(baseRef);
const groups = groupsAsked();
const tier = process.argv.includes("--changed") ? changedList(full) : process.argv.includes("--fast") ? fastList(full) : full;
// The job bun of a branch that changed only documents runs only the quality gates (`--jobs`).
const documentsOnly = process.env["GATES_DOCUMENTS_ONLY"] === "true";
if (documentsOnly) console.log("gates: the branch changed only documents, so only the quality gates run here");
const grouped = groups === undefined ? tier : ofGroups(tier, documentsOnly ? groups.filter((group) => group === "quality") : groups);
// `--gate`: a job on GitHub that runs a part of one group, for example only the Rust tests.
const named = option("--gate")?.split(",");
const missingName = named?.find((name) => !full.some((gate) => gate.name === name));
if (missingName !== undefined) {
  console.error(`gates: no such gate: ${missingName}\n${USAGE}`);
  process.exit(2);
}
const chosen = named === undefined ? grouped : grouped.filter((gate) => named.includes(gate.name));
// `--keep-going`: a proof run (the Windows job started by hand) shows each failure in one run.
const keepGoing = process.argv.includes("--keep-going");
if (keepGoing) console.log("gates: --keep-going: each gate runs after a failure; the failures are listed at the end. Not for a merge");
const gates = keepGoing ? keptGoing(chosen) : chosen;

const outcome = runGates(
  gates,
  (gate) => {
    const missing = missingRequirement(gate, process.env["PATH"] ?? "", existsSync);
    if (missing !== undefined) throw new Error(missing);
    const result = Bun.spawnSync(gate.argv, { cwd: REPO_ROOT, env: cache.env, stdin: "inherit", stdout: "inherit", stderr: "inherit" });
    return result.exitCode ?? 1;
  },
  (event) => {
    console.log(formatGateEvent(event));
  },
  { keepGoing },
);
if (leftOut !== undefined) console.log(leftOut);
process.exit(outcome.exitCode);
