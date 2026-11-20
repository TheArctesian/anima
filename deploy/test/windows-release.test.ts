import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

interface Step { name?: string; uses?: string; run?: string; if?: string; with?: Record<string, unknown> }
interface Workflow {
  on: Record<string, unknown>;
  permissions: Record<string, string>;
  jobs: Record<string, { permissions: Record<string, string>; environment?: string; steps: Step[] }>;
}
const root = join(import.meta.dir, "..", "..");
const raw = readFileSync(join(root, ".github/workflows/windows-trial.yml"), "utf8");
const workflow = Bun.YAML.parse(raw) as Workflow;

test("Windows dev trials are secret-free, read-only and never publish", () => {
  expect(workflow.on).toEqual({ push: { branches: ["dev"] }, workflow_dispatch: null });
  expect(workflow.permissions).toEqual({});
  expect(raw).not.toMatch(/secrets\.|release publish|:\s*write\b/);
  for (const job of Object.values(workflow.jobs)) {
    expect(job.environment).toBeUndefined();
    expect(job.permissions).toEqual({ contents: "read" });
    for (const step of job.steps) {
      expect(step.run ?? "").not.toContain("${{");
      if (step.uses !== undefined) expect(step.uses).toMatch(/^[\w.-]+\/[\w.-]+@[0-9a-f]{40}$/);
    }
  }
});

test("a downloadable trial must pass native gates and installation checks with updates disabled", () => {
  const steps = workflow.jobs["windows"]?.steps ?? [];
  const build = steps.findIndex((step) => step.run?.includes("windows-bundle.ps1"));
  const gates = steps.findIndex((step) => step.run?.includes("bun run gates"));
  const install = steps.findIndex((step) => step.run?.includes("deploy/windows/install.ps1"));
  const artifact = steps.findIndex((step) => step.with?.["name"] === "windows-trial");
  expect(build).toBeGreaterThanOrEqual(0);
  expect(gates).toBeGreaterThanOrEqual(build);
  expect(install).toBeGreaterThan(gates);
  expect(artifact).toBeGreaterThan(install);
  expect(steps[build]?.run).not.toContain("-Release");
  expect(steps[artifact]?.if).toBeUndefined();
  expect(steps[artifact]?.with?.["if-no-files-found"]).toBe("error");
});
