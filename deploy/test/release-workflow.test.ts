// Rules for .github/workflows/release.yml, checked on the parsed file (R-179, R-180; BLD-041,
// BLD-045, UPD-007, UPD-008), and the refusals of apps/desktop/src-tauri/check-mach-o.sh
// (UPD-011). Nothing here runs the workflow or reads a secret.
import { describe, expect, test } from "bun:test";
import { chmodSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const ROOT = join(import.meta.dir, "..", "..");
const RAW = readFileSync(join(ROOT, ".github", "workflows", "release.yml"), "utf8");
const MACH_O_CHECK = join(ROOT, "apps", "desktop", "src-tauri", "check-mach-o.sh");

interface Step {
  name?: string;
  uses?: string;
  if?: string;
  run?: string;
  env?: Record<string, string>;
  with?: Record<string, string | boolean | number>;
}
interface Job {
  environment?: string;
  needs?: string | string[];
  if?: string;
  permissions?: Record<string, string>;
  env?: Record<string, string>;
  steps: Step[];
}
interface Workflow {
  on: Record<string, { tags?: string[]; inputs?: Record<string, unknown> }>;
  concurrency: { group: string; "cancel-in-progress": boolean };
  permissions: Record<string, string>;
  jobs: Record<string, Job>;
}

const workflow = Bun.YAML.parse(RAW) as Workflow;
const jobs = workflow.jobs;
const allSteps = Object.values(jobs).flatMap((job) => job.steps);

/** The environment of each job that has one, and the secrets that only that job may name. */
const HOLDERS: Record<string, { environment: string; secrets: string[] }> = {
  macos: {
    environment: "release",
    secrets: ["APPLE_CERTIFICATE", "APPLE_CERTIFICATE_PASSWORD", "APPLE_ID", "APPLE_PASSWORD", "APPLE_TEAM_ID"],
  },
  sign: { environment: "release-sign", secrets: ["TAURI_SIGNING_PRIVATE_KEY", "TAURI_SIGNING_PRIVATE_KEY_PASSWORD"] },
  publish: { environment: "release-publish", secrets: ["R2_ACCESS_KEY_ID", "R2_SECRET_ACCESS_KEY"] },
};

function job(name: string): Job {
  const found = jobs[name];
  if (found === undefined) throw new Error(`the workflow has no job ${name}`);
  return found;
}

function needsOf(name: string): string[] {
  const needs = job(name).needs;
  return Array.isArray(needs) ? [...needs] : needs === undefined ? [] : [needs];
}

/** Each job that `name` waits for, directly or through a job that it needs. */
function waitsFor(name: string): Set<string> {
  const all = new Set<string>();
  const queue = needsOf(name);
  for (let next = queue.pop(); next !== undefined; next = queue.pop()) {
    if (all.has(next)) continue;
    all.add(next);
    queue.push(...needsOf(next));
  }
  return all;
}

/** The names of the secrets that the text of one job names. */
function secretsNamed(found: Job): string[] {
  const names = [...JSON.stringify(found).matchAll(/secrets\.([A-Z0-9_]+)/g)].map((match) => match[1] ?? "");
  return [...new Set(names)].sort();
}

function runText(found: Job): string {
  return found.steps.map((step) => step.run ?? "").join("\n");
}

describe("what starts the workflow (UPD-007, OD-W26)", () => {
  test("only a version tag and a run by hand start it", () => {
    expect(Object.keys(workflow.on).sort()).toEqual(["push", "workflow_dispatch"]);
    expect(workflow.on["push"]).toEqual({ tags: ["v[0-9]+.[0-9]+.[0-9]+"] });
  });

  test("no pull request starts it", () => {
    expect(RAW).not.toMatch(/pull_request/);
  });

  test("a release in progress is not cancelled, and two do not run together", () => {
    expect(workflow.concurrency).toEqual({ group: "release", "cancel-in-progress": false });
  });

  test("BLD-041 the first job refuses a commit that is not on main, and each other job waits for it", () => {
    expect(runText(job("commit"))).toContain('git merge-base --is-ancestor "$sha" refs/remotes/origin/main');
    for (const name of Object.keys(jobs)) {
      if (name !== "commit") expect(waitsFor(name).has("commit")).toBe(true);
    }
  });

  test("R-183 a tag must give the version of its commit, and the job has the full history for the count", () => {
    const checkout = job("commit").steps.find((step) => step.uses?.startsWith("actions/checkout@"));
    expect(checkout?.with?.["fetch-depth"]).toBe(0);
    const text = runText(job("commit"));
    expect(text).toContain('of_commit=$(sh deploy/bin/commit-version "$sha")');
    expect(text).toMatch(/\[ "\$EVENT" = push \] && \[ "\$version" != "\$of_commit" \]; then\n\s+echo "::error::[^\n]+\n\s+exit 1/);
  });

  test("each job that reads the repository reads the commit that the first job checked", () => {
    for (const [name, found] of Object.entries(jobs)) {
      for (const step of found.steps.filter((one) => one.uses?.startsWith("actions/checkout@"))) {
        expect(step.with?.["persist-credentials"]).toBe(false);
        if (name !== "commit") expect(step.with?.["ref"]).toBe("${{ needs.commit.outputs.sha }}");
      }
    }
  });
});

describe("permissions and pinning", () => {
  test("no job may write to the repository", () => {
    expect(workflow.permissions).toEqual({});
    for (const found of Object.values(jobs)) {
      const granted = JSON.stringify(found.permissions);
      expect(granted === "{}" || granted === '{"contents":"read"}').toBe(true);
    }
    expect(RAW).not.toMatch(/:\s*write\b/);
  });

  test("every action is pinned to a full commit hash", () => {
    const uses = allSteps.flatMap((step) => (step.uses === undefined ? [] : [step.uses]));
    expect(uses.length).toBeGreaterThan(0);
    for (const use of uses) expect(use).toMatch(/^[\w.-]+\/[\w.-]+@[0-9a-f]{40}$/);
  });

  test("no run script holds an expression", () => {
    for (const step of allSteps) expect(step.run ?? "").not.toContain("${{");
  });
});

describe("which job holds which secret (R-180, BLD-045)", () => {
  test("only the three jobs with a key have an environment, each its own", () => {
    for (const [name, found] of Object.entries(jobs)) {
      expect(found.environment).toBe(HOLDERS[name]?.environment);
    }
  });

  test("a job names only the secrets of its own work", () => {
    for (const [name, found] of Object.entries(jobs)) {
      expect(secretsNamed(found)).toEqual([...(HOLDERS[name]?.secrets ?? [])].sort());
    }
  });

  test("a secret is in the environment of one step, never of a whole job", () => {
    for (const found of Object.values(jobs)) {
      expect(JSON.stringify(found.env ?? {})).not.toContain("secrets.");
    }
  });

  test("the jobs that hold the updater key or the bucket token read no repository and compile nothing", () => {
    for (const name of ["sign", "publish", "trial"]) {
      const found = job(name);
      const actions = found.steps.flatMap((step) => (step.uses === undefined ? [] : [step.uses.split("@")[0]]));
      for (const action of actions) {
        expect(["actions/download-artifact", "actions/upload-artifact"]).toContain(action ?? "");
      }
      expect(runText(found)).not.toMatch(/(^|[\s;&|(])(cargo|bun|bunx|go|npm|git|rustc|make) /);
    }
  });

  test("BLD-046 the sign job compares each file with the digest of its build job before the key step", () => {
    const steps = job("sign").steps;
    const compare = steps.findIndex((step) => (step.run ?? "").includes("sha256sum --check --strict"));
    const sign = steps.findIndex((step) => JSON.stringify(step.env ?? {}).includes("secrets.TAURI_SIGNING_PRIVATE_KEY"));
    expect(compare).toBeGreaterThanOrEqual(0);
    expect(sign).toBeGreaterThan(compare);
    expect(steps[compare]?.env).toMatchObject({
      TOOL_SHA256: "${{ needs.tool.outputs.sha256 }}",
      MACOS_SHA256: "${{ needs.macos.outputs.updater_sha256 }}",
      LINUX_SHA256: "${{ needs.linux.outputs.updater_sha256 }}",
    });
  });
});

describe("the order to a published release (UPD-008, BLD-042)", () => {
  test("the sign job waits for the gates, the tool and both builds", () => {
    expect(needsOf("sign").sort()).toEqual(["commit", "gates", "linux", "macos", "tool"]);
  });

  test("the gates job has the tools and the history that the gates need on a machine of GitHub", () => {
    const install = runText(job("gates"));
    for (const tool of ["postgresql@17", "cargo-nextest", "shellcheck"]) {
      expect(install).toMatch(new RegExp(`brew install [^\\n]*${tool}`));
    }
    const checkout = job("gates").steps.find((step) => step.uses?.startsWith("actions/checkout@"));
    expect(checkout?.with?.["fetch-depth"]).toBe(0);
    expect(job("gates").steps.some((step) => step.uses?.startsWith("actions/setup-go@"))).toBe(true);
    expect(job("gates").steps.some((step) => step.uses?.startsWith("oven-sh/setup-bun@"))).toBe(true);
  });

  test("publish runs only for a tag and only after every other job passed", () => {
    expect(job("publish").if).toBe("github.event_name == 'push' && startsWith(github.ref, 'refs/tags/v')");
    expect(job("publish").if).not.toMatch(/always\(\)|cancelled\(\)|failure\(\)/);
    expect([...waitsFor("publish")].sort()).toEqual(["commit", "gates", "linux", "macos", "sign", "tool"]);
  });

  test("a run by hand ends in the trial job, which has no token and publishes nothing", () => {
    expect(job("trial").if).toBe("github.event_name == 'workflow_dispatch'");
    expect(runText(job("trial"))).not.toContain("release publish");
    expect(secretsNamed(job("trial"))).toEqual([]);
  });

  test("publish checks both targets before it writes one, and SHA256SUMS comes from the job outputs", () => {
    const text = runText(job("publish"));
    const lastCheck = text.lastIndexOf("release check");
    const firstPublish = text.indexOf("release publish");
    expect(lastCheck).toBeGreaterThanOrEqual(0);
    expect(firstPublish).toBeGreaterThan(lastCheck);
    expect(text).toContain('echo "$LINUX_SHA256  Anima_${VERSION}_amd64.AppImage" > linux/SHA256SUMS');
    expect(text).toContain('echo "$MACOS_SHA256  Anima.app.tar.gz"');
  });

  test("the macOS job checks the Mach-O files before the notary step, and makes the updater file after the staple", () => {
    const text = runText(job("macos"));
    const at = (part: string): number => text.indexOf(part);
    expect(at("trial-bundle.sh")).toBeGreaterThanOrEqual(0);
    expect(at("check-mach-o.sh")).toBeGreaterThan(at("trial-bundle.sh"));
    expect(at("notarytool submit")).toBeGreaterThan(at("check-mach-o.sh"));
    expect(at("stapler validate")).toBeGreaterThan(at("notarytool submit"));
    expect(at("tar -czf out/Anima.app.tar.gz")).toBeGreaterThan(at("stapler validate"));
  });
});

const BUNDLE_SCRIPT = join(ROOT, "apps", "desktop", "src-tauri", "trial-bundle.sh");
const UPDATER_OFF = "ANIMA_LOCAL_BUILD=1 VITE_ANIMA_LOCAL_BUILD=1";
const UPDATER_ON = "ANIMA_LOCAL_BUILD=unset VITE_ANIMA_LOCAL_BUILD=unset";

/** The marks of a local build that `trial-bundle.sh` gives a build with the variables `set`; it builds nothing. */
function marks(set: Record<string, string>): { exitCode: number; printed: string } {
  const env = { PATH: process.env["PATH"] ?? "/usr/bin:/bin", ...set };
  const result = Bun.spawnSync(["/bin/sh", BUNDLE_SCRIPT, "--marks", "0.2.2"], { env, stdout: "pipe", stderr: "pipe" });
  return { exitCode: result.exitCode, printed: result.stdout.toString().trim() };
}

describe("the updater of a built application (UPD-001, DEF-001)", () => {
  const identity = { SIGN_IDENTITY: "Developer ID Application: A Test (ABCDE12345)" };

  test("a build of the workflow has the updater on, also when the caller's environment has a mark", () => {
    expect(marks({ ANIMA_RELEASE_BUILD: "1", ...identity })).toEqual({ exitCode: 0, printed: UPDATER_ON });
    const marked = { ANIMA_RELEASE_BUILD: "1", ANIMA_LOCAL_BUILD: "", VITE_ANIMA_LOCAL_BUILD: "1", ...identity };
    expect(marks(marked)).toEqual({ exitCode: 0, printed: UPDATER_ON });
  });

  test("a build by hand has the updater off, with or without a signing identity", () => {
    expect(marks({})).toEqual({ exitCode: 0, printed: UPDATER_OFF });
    expect(marks(identity)).toEqual({ exitCode: 0, printed: UPDATER_OFF });
    expect(marks({ ANIMA_RELEASE_BUILD: "0", ...identity })).toEqual({ exitCode: 0, printed: UPDATER_OFF });
  });

  test.skipIf(process.platform !== "darwin")("a release build on macOS with no signing identity is refused", () => {
    expect(marks({ ANIMA_RELEASE_BUILD: "1" }).exitCode).toBe(1);
  });

  test("both build jobs of the workflow ask for a release build, for a tag and for a trial run alike", () => {
    for (const name of ["macos", "linux"]) {
      const build = job(name).steps.filter((step) => (step.run ?? "").includes("trial-bundle.sh"));
      expect(build).toHaveLength(1);
      expect(build[0]?.env?.["ANIMA_RELEASE_BUILD"]).toBe("1");
      expect(build[0]?.if).toBeUndefined();
      expect(job(name).if).toBeUndefined();
    }
    expect(RAW).not.toMatch(/ANIMA_LOCAL_BUILD/);
  });
});

/** A folder shaped as an application whose programs are copies of one universal file with an ad hoc signature. */
function adHocApp(programs: string[]): { folder: string; app: string } {
  const folder = mkdtempSync(join(tmpdir(), "anima-mach-o-"));
  const app = join(folder, "Anima.ing.app");
  const macos = join(app, "Contents", "MacOS");
  mkdirSync(macos, { recursive: true });
  const source = join(folder, "main.c");
  writeFileSync(source, "int main(void) { return 0; }\n");
  const built = join(folder, "program");
  const run = (argv: string[]): void => {
    const result = Bun.spawnSync(argv, { stdout: "ignore", stderr: "pipe" });
    if (result.exitCode !== 0) throw new Error(`${argv[0] ?? ""} failed: ${result.stderr.toString()}`);
  };
  run(["/usr/bin/cc", "-arch", "arm64", "-arch", "x86_64", "-o", built, source]);
  for (const program of programs) {
    writeFileSync(join(macos, program), readFileSync(built));
    chmodSync(join(macos, program), 0o755);
    run(["/usr/bin/codesign", "--force", "--sign", "-", join(macos, program)]);
  }
  return { folder, app };
}

function machOCheck(app: string): { exitCode: number; errors: string } {
  const result = Bun.spawnSync(["/bin/sh", MACH_O_CHECK, app, "ABCDE12345"], { stdout: "ignore", stderr: "pipe" });
  return { exitCode: result.exitCode, errors: result.stderr.toString() };
}

const PROGRAMS = ["anima", "anima-database", "anima-desktop", "anima-gateway", "anima-hub", "anima-link", "anima-local-hub"];

// The checked programs are of macOS (codesign, lipo); the pinning gate is skipped elsewhere in the same way.
describe.skipIf(process.platform !== "darwin")("check-mach-o.sh (UPD-011)", () => {
  test("an application whose programs have an ad hoc signature is refused", () => {
    const { folder, app } = adHocApp(PROGRAMS);
    try {
      const result = machOCheck(app);
      expect(result.exitCode).toBe(1);
      expect(result.errors).toContain("check-mach-o:");
    } finally {
      rmSync(folder, { recursive: true, force: true });
    }
  });

  test("an application with a program that a release does not carry is refused by its name", () => {
    const { folder, app } = adHocApp([...PROGRAMS, "extra-program"]);
    try {
      const result = machOCheck(app);
      expect(result.exitCode).toBe(1);
      expect(result.errors).toContain("Contents/MacOS holds");
    } finally {
      rmSync(folder, { recursive: true, force: true });
    }
  });
});
