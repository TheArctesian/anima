import { existsSync } from "node:fs";
import { compileCache } from "../src/gates/cache.ts";
import { formatGateEvent, gateList, GROUPS, missingRequirement, ofGroups, runGates, type Gate, type Group } from "../src/gates/gates.ts";
import { changedPaths, readLayout } from "../src/gates/layout.ts";
import { choose, describeChoice, fastGates } from "../src/gates/tier.ts";
import { REPO_ROOT } from "../src/repository.ts";

const USAGE = "usage: bun quality/bin/gates.ts [--fast] [--group rust,go,bun,quality]";

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

/** The gates of the fast tier (`--fast`): chosen by the paths that differ from `FAST_BASE` (default `origin/dev`). */
function fastList(full: Gate[]): Gate[] {
  const base = process.env["FAST_BASE"] || "origin/dev";
  const paths = changedPaths(REPO_ROOT, base);
  if (paths === undefined) {
    console.log(`fast tier: full; ${base} is not a commit here (run \`git fetch origin\`)`);
    return full;
  }
  const choice = choose(paths, readLayout(REPO_ROOT));
  console.log(describeChoice(choice));
  return fastGates(choice, full);
}

const unknown = process.argv.slice(2).filter((each, at, all) => each.startsWith("--") ? !["--fast", "--group"].includes(each) : all[at - 1] !== "--group");
if (unknown.length > 0) {
  console.error(`gates: unknown argument: ${unknown.join(" ")}\n${USAGE}`);
  process.exit(2);
}

const baseRef = process.env["BASE_REF"] || "phase-2";
const cache = compileCache(process.env, existsSync);
console.log(cache.note);

const full = gateList(baseRef);
const groups = groupsAsked();
const tier = process.argv.includes("--fast") ? fastList(full) : full;
const gates = groups === undefined ? tier : ofGroups(tier, groups);

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
);
process.exit(outcome.exitCode);
