// Rules for .github/workflows/gates.yml, checked on the parsed file: the gates on GitHub hold no
// secret, can write nothing, give a pull request of a fork nothing, and run the list of
// quality/src/gates/gates.ts. Nothing here runs the workflow.
import { describe, expect, test } from "bun:test";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
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
  uses?: string;
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
const jobs = Object.entries(workflow.jobs).map(([name, job]) => [name, { ...job, steps: job.steps ?? [] }] as const);
const steps = jobs.flatMap(([, job]) => job.steps);
const gateJobs = jobs.filter(([name]) => name !== "passed" && name !== "base" && name !== "windows");

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
    expect(workflow.on["push"]).toEqual({ "branches-ignore": ["main", "train/**"] });
    for (const [name, job] of jobs) {
      if (name === "windows") expect(job.uses).toBe("./.github/workflows/windows-trial.yml");
      else expect(job["runs-on"], name).toBe("ubuntu-24.04");
    }
  });

  test("each action is pinned to a commit, and no run script holds an expression", () => {
    for (const step of steps) {
      if (step.uses !== undefined) expect(step.uses).toMatch(/^[\w.-]+\/[\w.-]+@[0-9a-f]{40}$/);
      expect(step.run ?? "").not.toContain("${{");
    }
  });

  test("R-252 each job of the gates tests the branch as merged with the dev that the job `base` read", () => {
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
    expect(passed?.needs?.sort()).toEqual(["base", "windows", ...gateJobs.map(([name]) => name)].sort());
    expect(passed?.if).toBe("always()");
  });

  test("a workflow that calls the gates reads the cache and does not write it", () => {
    for (const step of steps.filter((each) => each.uses?.startsWith("Swatinem/rust-cache@"))) {
      expect(step.with?.["save-if"]).toBe("${{ inputs.ref == '' }}");
    }
  });
});

describe("the workflow that merges into dev (merge-to-dev.yml; R-252)", () => {
  const raw = readFileSync(join(ROOT, ".github", "workflows", "merge-to-dev.yml"), "utf8");
  const merge = (Bun.YAML.parse(raw) as Workflow).jobs["merge"];
  const script = readFileSync(join(ROOT, ".github", "workflows", "merge-to-dev.sh"), "utf8");

  test("for a push it takes only a branch of this repository whose gates passed, and never dev, main or a fork", () => {
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

  test("the end of the gates of a train starts it too, with each conclusion, so that a failed train is split", () => {
    expect(merge?.if).toContain("(github.event.workflow_run.event == 'workflow_dispatch' && startsWith(github.event.workflow_run.head_branch, 'train/'))");
    expect(merge?.if).toContain("!startsWith(github.event.workflow_run.head_branch, 'train/')");
  });

  test("it reads the rule of dev first, then runs one step of the train; it starts no gates of a branch and none of dev", () => {
    const rule = script.indexOf("quality/merge-rule.json");
    const train = script.indexOf("bun quality/bin/train.ts");
    expect(rule).toBeGreaterThan(-1);
    expect(train).toBeGreaterThan(rule);
    expect(script).not.toContain("gh run rerun");
    expect(script.match(/gh workflow run.*/g)).toEqual(["gh workflow run merge-to-dev.yml"]);
  });

  test("each action is pinned to a commit, and no run script holds an expression", () => {
    for (const step of merge?.steps ?? []) {
      if (step.uses !== undefined) expect(step.uses).toMatch(/^[\w.-]+\/[\w.-]+@[0-9a-f]{40}$/);
      expect(step.run ?? "").not.toContain("${{");
    }
  });
});

describe("while a train is on the way, nothing else wakes it: the script stays, then starts the workflow again", () => {
  /** Runs the script with stand-ins for bun, gh, git and sleep; `trains` says, for each step, if a train exists after it. */
  function steps(trains: boolean[], seconds: number, rule = '{"mergeWithoutCheck": true, "fullGates": "github"}'): string[] {
    const dir = mkdtempSync(join(tmpdir(), "merge-to-dev-"));
    mkdirSync(join(dir, "bin"));
    mkdirSync(join(dir, "quality"));
    writeFileSync(join(dir, "quality", "merge-rule.json"), rule);
    writeFileSync(join(dir, "trains"), trains.map((each) => (each ? "sha\trefs/heads/train/a" : "")).join("\n"));
    const log = `echo "$(basename "$0") $*" >> "${dir}/log"`;
    const stand: Record<string, string> = {
      bun: log,
      gh: log,
      sleep: log,
      git: `[ "$1" = ls-remote ] || exit 0\nn=$(grep -c '^bun' "${dir}/log")\nsed -n "\${n}p" "${dir}/trains"`,
    };
    for (const [name, body] of Object.entries(stand)) writeFileSync(join(dir, "bin", name), `#!/bin/sh\n${body}\n`, { mode: 0o755 });
    const env = { PATH: `${join(dir, "bin")}:/usr/bin:/bin`, TRAIN_WATCH_SECONDS: String(seconds), TRAIN_WATCH_PAUSE: "0" };
    const done = Bun.spawnSync(["/bin/sh", join(ROOT, ".github", "workflows", "merge-to-dev.sh")], { cwd: dir, env });
    expect(done.exitCode).toBe(0);
    const said = existsSync(join(dir, "log")) ? readFileSync(join(dir, "log"), "utf8").trim().split("\n") : [];
    rmSync(dir, { recursive: true });
    return said.filter((line) => !line.startsWith("sleep"));
  }
  const step = "bun quality/bin/train.ts";

  test("no train after the step: one step, and nothing is started", () => {
    expect(steps([false], 600)).toEqual([step]);
  });

  test("a train on the way: it does a step again until no train is left, so the train lands with no event", () => {
    expect(steps([true, true, false], 600)).toEqual([step, step, step]);
  });

  test("the train is still on the way when the time ends: it starts the workflow again, one time", () => {
    expect(steps([true], 0)).toEqual([step, "gh workflow run merge-to-dev.yml"]);
  });

  test("the rule of dev is off: no step and no start", () => {
    expect(steps([true], 600, '{"mergeWithoutCheck": false, "fullGates": "github"}')).toEqual([]);
  });
});
