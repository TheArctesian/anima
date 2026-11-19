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
const gateJobs = jobs.filter(([name]) => name !== "passed" && name !== "base");

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

  test("PRO-044 no pull request starts it, and no job runs on a self-hosted runner", () => {
    expect(Object.keys(workflow.on).sort()).toEqual(["push", "workflow_call", "workflow_dispatch"]);
    expect(workflow.on["push"]).toEqual({ "branches-ignore": ["main"] });
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

  test("R-NEW-automerge each job of the gates tests the branch as merged with the dev that the job `base` read", () => {
    expect(workflow.jobs["base"]?.steps.some((step) => step.with?.["name"] === "tested-base")).toBe(true);
    for (const [name, job] of gateJobs) {
      expect(job.needs, name).toEqual("base" as unknown as string[]);
      const merge = job.steps.find((step) => step.run?.includes("gates-merge.sh"));
      expect(merge?.env?.["DEV_SHA"], name).toBe("${{ needs.base.outputs.dev_sha }}");
      const checkout = job.steps.find((step) => step.uses?.startsWith("actions/checkout@"));
      expect(checkout?.with?.["fetch-depth"], name).toBe(0);
      const order = job.steps.map((step) => step.run ?? "");
      expect(order.findIndex((run) => run.includes("gates-merge.sh")), name).toBeLessThan(
        order.findIndex((run) => run.includes("quality/bin/gates.ts")),
      );
    }
  });

  test("the job `passed` waits for each job and fails when one did not pass", () => {
    const passed = workflow.jobs["passed"];
    expect(passed?.needs?.sort()).toEqual(["base", ...gateJobs.map(([name]) => name)].sort());
    expect(passed?.if).toBe("always()");
  });

  test("a workflow that calls the gates reads the cache and does not write it", () => {
    for (const step of steps.filter((each) => each.uses?.startsWith("Swatinem/rust-cache@"))) {
      expect(step.with?.["save-if"]).toBe("${{ inputs.ref == '' }}");
    }
  });
});

describe("the workflow that merges into dev (merge-to-dev.yml; R-NEW-automerge)", () => {
  const raw = readFileSync(join(ROOT, ".github", "workflows", "merge-to-dev.yml"), "utf8");
  const merge = (Bun.YAML.parse(raw) as Workflow).jobs["merge"];
  const script = readFileSync(join(ROOT, ".github", "workflows", "merge-to-dev.sh"), "utf8");

  test("it takes only a push of a branch of this repository whose gates passed, and never dev or main", () => {
    for (const part of [
      "github.event.workflow_run.conclusion == 'success'",
      "github.event.workflow_run.event == 'push'",
      "github.event.workflow_run.head_repository.full_name == github.repository",
      "github.event.workflow_run.head_branch != 'dev'",
      "github.event.workflow_run.head_branch != 'main'",
    ]) {
      expect(merge?.if).toContain(part);
    }
    expect(merge?.["runs-on"]).toBe("ubuntu-24.04");
    expect(merge?.steps.find((each) => each.uses?.startsWith("actions/checkout@"))?.with?.["ref"]).toBe("dev");
    expect(raw).not.toContain("refs/heads/main");
    expect(script).not.toContain("main");
  });

  test("it reads the rule of dev first, and merges only when dev is the commit that the gates tested", () => {
    const rule = script.indexOf("quality/merge-rule.json");
    const tested = script.indexOf('if [ "$tested" != "$dev_now" ]');
    const merged = script.indexOf("bun quality/bin/merge.ts");
    expect(rule).toBeGreaterThan(-1);
    expect(tested).toBeGreaterThan(rule);
    expect(merged).toBeGreaterThan(tested);
    expect(script).toContain('gh run rerun "$RUN_ID"');
  });

  test("each action is pinned to a commit, and no run script holds an expression", () => {
    for (const step of merge?.steps ?? []) {
      if (step.uses !== undefined) expect(step.uses).toMatch(/^[\w.-]+\/[\w.-]+@[0-9a-f]{40}$/);
      expect(step.run ?? "").not.toContain("${{");
    }
  });
});
