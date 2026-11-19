// Rules for .github/workflows/gates.yml, checked on the parsed file: the gates on GitHub hold no
// secret, can write nothing, give a pull request of a fork nothing, and run the list of
// quality/src/gates/gates.ts. Nothing here runs the workflow.
import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { gateList } from "../../quality/src/gates/gates.ts";

const ROOT = join(import.meta.dir, "..", "..");
const RAW = readFileSync(join(ROOT, ".github", "workflows", "gates.yml"), "utf8");

interface Step {
  name?: string;
  uses?: string;
  run?: string;
  env?: Record<string, string>;
  with?: Record<string, string | boolean | number>;
}
interface Job {
  "runs-on": string;
  if?: string;
  needs?: string[];
  environment?: string;
  permissions?: Record<string, string>;
  steps: Step[];
}
interface Workflow {
  on: Record<string, unknown>;
  permissions: Record<string, string>;
  jobs: Record<string, Job>;
}

const workflow = Bun.YAML.parse(RAW) as Workflow;
const jobs = Object.entries(workflow.jobs);
const steps = jobs.flatMap(([, job]) => job.steps);
const gateJobs = jobs.filter(([name]) => name !== "all");

/** The gates that the `run` scripts of the workflow select, in the way `quality/bin/gates.ts` selects them. */
function gatesThatRun(): string[] {
  const full = gateList("x");
  return steps.flatMap((step) => {
    const command = /quality\/bin\/gates\.ts(.*)/.exec(step.run ?? "")?.[1];
    if (command === undefined) return [];
    const groups = /--group (\S+)/.exec(command)?.[1]?.split(",");
    const named = /--gate (\S+)/.exec(command)?.[1]?.split(",");
    return full
      .filter((gate) => groups === undefined || groups.includes(gate.group))
      .filter((gate) => named === undefined || named.includes(gate.name))
      .map((gate) => gate.name);
  });
}

describe("the gates workflow (gates.yml)", () => {
  test("CQ-010 the jobs together run each gate of the list one time", () => {
    const all = gateList("x").map((gate) => gate.name);
    expect(gatesThatRun().sort()).toEqual([...all].sort());
  });

  test("it holds no secret, no environment and no token that can write", () => {
    expect(RAW).not.toContain("secrets.");
    expect(RAW).not.toContain("secrets:");
    expect(workflow.permissions).toEqual({});
    for (const [name, job] of jobs) {
      expect(job.environment, name).toBeUndefined();
      expect(Object.values(job.permissions ?? {}).filter((value) => value !== "read"), name).toEqual([]);
      expect(Object.keys(job.permissions ?? {}).filter((key) => key !== "contents"), name).toEqual([]);
    }
    for (const step of steps.filter((each) => each.uses?.startsWith("actions/checkout@"))) {
      expect(step.with?.["persist-credentials"]).toBe(false);
    }
  });

  test("PRO-044 a pull request of a fork gets a read-only run: never pull_request_target, never a self-hosted runner", () => {
    expect(Object.keys(workflow.on).sort()).toEqual(["pull_request", "push", "workflow_call"]);
    for (const [name, job] of jobs) {
      expect(job["runs-on"], name).toBe("ubuntu-24.04");
    }
  });

  test("each action is pinned to a commit, and no run script holds an expression", () => {
    for (const step of steps) {
      if (step.uses !== undefined) expect(step.uses).toMatch(/^[\w.-]+\/[\w.-]+@[0-9a-f]{40}$/);
      expect(step.run ?? "").not.toContain("${{");
    }
  });

  test("a branch of this repository is checked by its push only, and `main` by the release workflow", () => {
    expect(workflow.on["push"]).toEqual({ "branches-ignore": ["main"] });
    expect(workflow.on["pull_request"]).toEqual({ branches: ["dev"] });
    for (const [name, job] of gateJobs) {
      expect(job.if, name).toContain("github.event.pull_request.head.repo.full_name != github.repository");
    }
  });

  test("the job `all` waits for each job of the gates and fails when one did not pass", () => {
    const all = workflow.jobs["all"];
    expect(all?.needs?.sort()).toEqual(gateJobs.map(([name]) => name).sort());
    expect(all?.if).toStartWith("always() && ");
  });

  test("a cache is written by a push only, so a pull request cannot fill the cache of a branch", () => {
    for (const step of steps.filter((each) => each.uses?.startsWith("Swatinem/rust-cache@"))) {
      expect(step.with?.["save-if"]).toBe("${{ github.event_name == 'push' }}");
    }
  });
});
