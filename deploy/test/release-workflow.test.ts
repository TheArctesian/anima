// Rules for .github/workflows/release.yml, checked on the parsed file (UPD-007, UPD-008,
// UPD-009, UPD-011, UPD-012, R-005). Nothing here runs the workflow.
import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const ROOT = join(import.meta.dir, "..", "..");
const RAW = readFileSync(join(ROOT, ".github", "workflows", "release.yml"), "utf8");

interface Step {
  name?: string;
  uses?: string;
  if?: string;
  run?: string;
  env?: Record<string, string>;
}
interface Job {
  needs?: string | string[];
  if?: string;
  permissions?: Record<string, string>;
  steps: Step[];
}
interface Workflow {
  on: Record<string, unknown>;
  concurrency: { group: string; "cancel-in-progress": boolean };
  permissions: Record<string, string>;
  jobs: Record<string, Job>;
}

const workflow = Bun.YAML.parse(RAW) as Workflow;
const jobs = workflow.jobs;
const publish = jobs["publish"] as Job;
const allSteps = Object.values(jobs).flatMap((job) => job.steps);

function runText(job: Job): string {
  return job.steps.map((step) => step.run ?? "").join("\n");
}

function needsOf(job: Job): string[] {
  return Array.isArray(job.needs) ? job.needs : job.needs === undefined ? [] : [job.needs];
}

describe("triggers and concurrency (UPD-007, R-005)", () => {
  test("UPD-007 runs only on a push to main and by hand", () => {
    expect(Object.keys(workflow.on).sort()).toEqual(["push", "workflow_dispatch"]);
    expect(workflow.on["push"]).toEqual({ branches: ["main"] });
  });

  test("UPD-007 never uses a pull request trigger", () => {
    expect(RAW).not.toMatch(/pull_request/);
  });

  test("UPD-007 a release in progress is not cancelled", () => {
    expect(workflow.concurrency).toEqual({ group: "release", "cancel-in-progress": false });
  });

  test("R-005 the workflow runs no gate and no test", () => {
    expect(RAW).not.toMatch(/bun run gates|bun test|cargo test|bun run lint/);
  });
});

describe("permissions and pinning", () => {
  test("only the publish job may write", () => {
    expect(workflow.permissions).toEqual({ contents: "read" });
    for (const [name, job] of Object.entries(jobs)) {
      expect(job.permissions?.["contents"] === "write").toBe(name === "publish");
    }
    expect(RAW.match(/contents: write/g)).toHaveLength(1);
  });

  test("every action is pinned to a 40-hex commit SHA", () => {
    const uses = allSteps.flatMap((step) => (step.uses === undefined ? [] : [step.uses]));
    expect(uses.length).toBeGreaterThan(0);
    for (const use of uses) expect(use).toMatch(/^[\w.-]+\/[\w.-]+@[0-9a-f]{40}$/);
  });

  test("no run script interpolates an expression", () => {
    for (const step of allSteps) expect(step.run ?? "").not.toContain("${{");
  });
});

describe("job order (UPD-008, UPD-009)", () => {
  test("UPD-009 publish waits for the version job and both builds", () => {
    expect(needsOf(publish).sort()).toEqual(["build-linux", "build-macos", "version"]);
  });

  test("UPD-009 publish runs after a failed build only when the version job and one build passed", () => {
    const condition = publish.if ?? "";
    expect(condition).toContain("always()");
    expect(condition).toContain("needs.version.result == 'success'");
    expect(condition).toContain("needs.build-macos.result == 'success'");
    expect(condition).toContain("needs.build-linux.result == 'success'");
  });

  test("UPD-009 each artifact is downloaded only when its build succeeded", () => {
    const downloads = publish.steps.filter((step) => step.uses?.startsWith("actions/download-artifact@"));
    expect(downloads.map((step) => step.if).sort()).toEqual([
      "needs.build-linux.result == 'success'",
      "needs.build-macos.result == 'success'",
    ]);
  });

  test("UPD-008 the release is a draft, latest.json is uploaded after the other files, and publishing is last", () => {
    const at = (pattern: RegExp) => publish.steps.findIndex((step) => pattern.test(step.run ?? ""));
    const create = at(/gh release create .*--draft/);
    const files = at(/gh release upload .*\$file/);
    const feed = at(/gh release upload .*latest\.json$/m);
    const release = at(/gh release edit .*--draft=false --latest/);
    expect(create).toBeGreaterThanOrEqual(0);
    expect(files).toBeGreaterThan(create);
    expect(feed).toBeGreaterThan(files);
    expect(release).toBe(feed + 1);
    expect(release).toBe(publish.steps.length - 1);
  });
});

describe("build checks (UPD-011, UPD-012)", () => {
  test("UPD-011 the macOS job verifies the signature and both architectures", () => {
    const text = runText(jobs["build-macos"] as Job);
    expect(text).toContain("codesign --verify --deep --strict");
    expect(text).toContain("lipo -archs");
    expect(text).toContain("arm64 x86_64");
    expect(text).toContain("--target universal-apple-darwin");
  });

  test("UPD-012 the version job stops on the placeholder public key and on an empty private key", () => {
    const text = runText(jobs["version"] as Job);
    expect(text).toContain("REPLACE_WITH_OPERATOR_PUBLIC_KEY");
    expect(text).toContain("plugins.updater.pubkey");
    const step = (jobs["version"] as Job).steps.find((s) => s.env?.["KEY_IS_SET"] !== undefined);
    expect(step?.env?.["KEY_IS_SET"]).toBe("${{ secrets.TAURI_SIGNING_PRIVATE_KEY != '' }}");
  });

  test("UPD-012 builds wait for the version job, and the signing key reaches only a build step", () => {
    for (const name of ["build-macos", "build-linux"]) expect(needsOf(jobs[name] as Job)).toEqual(["version"]);
    for (const [name, job] of Object.entries(jobs)) {
      for (const step of job.steps) {
        const holdsKey = Object.values(step.env ?? {}).some((v) =>
          /secrets\.TAURI_SIGNING_PRIVATE_KEY(_PASSWORD)?\s*\}\}/.test(v),
        );
        expect(holdsKey).toBe(name.startsWith("build-") && (step.run ?? "").includes("tauri build"));
      }
    }
  });

  test("UPD-012 no step echoes a secret", () => {
    expect(RAW).not.toMatch(/echo[^\n]*(secrets\.|\$\{?(TAURI_SIGNING|APPLE_))/);
  });
});
