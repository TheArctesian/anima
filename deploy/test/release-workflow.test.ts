// Rules for .github/workflows/release.yml, checked on the parsed file (R-179, R-180; BLD-041,
// BLD-045, UPD-007, UPD-008). Nothing here runs the workflow or reads a secret.
import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const ROOT = join(import.meta.dir, "..", "..");
const RAW = readFileSync(join(ROOT, ".github", "workflows", "release.yml"), "utf8");

interface Step {
  id?: string;
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
  on: Record<string, { branches?: string[]; tags?: string[]; inputs?: Record<string, unknown> }>;
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
  bucket: { environment: "release-publish", secrets: ["R2_ACCESS_KEY_ID", "R2_SECRET_ACCESS_KEY"] },
  publish: { environment: "release-publish", secrets: ["R2_ACCESS_KEY_ID", "R2_SECRET_ACCESS_KEY"] },
  "cli-publish": { environment: "release-publish", secrets: ["R2_ACCESS_KEY_ID", "R2_SECRET_ACCESS_KEY"] },
  tap: { environment: "release-tap", secrets: ["HOMEBREW_TAP_DEPLOY_KEY"] },
  crate: { environment: "release-crate", secrets: ["CARGO_REGISTRY_TOKEN"] },
  ios: { environment: "release-ios", secrets: ["IOS_DISTRIBUTION_P12_BASE64", "IOS_DISTRIBUTION_P12_PASSWORD", "IOS_PROVISIONING_PROFILE_BASE64"] },
  "ios-upload": { environment: "release-testflight", secrets: ["ASC_API_ISSUER_ID", "ASC_API_KEY_ID", "ASC_API_PRIVATE_KEY"] },
  "ios-record": { environment: "release-publish", secrets: ["R2_ACCESS_KEY_ID", "R2_SECRET_ACCESS_KEY"] },
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

describe("what starts the workflow (UPD-007, R-184, R-198)", () => {
  test("only a push to main and a run by hand start it; no tag and no other branch does", () => {
    expect(Object.keys(workflow.on).sort()).toEqual(["push", "workflow_dispatch"]);
    expect(workflow.on["push"]).toEqual({ branches: ["main"] });
  });

  test("no pull request starts it", () => {
    expect(RAW).not.toMatch(/pull_request/);
  });

  test("releases run one at a time and none in progress is cancelled; a trial has a group of its own", () => {
    expect(workflow.concurrency).toEqual({
      group: "${{ github.event_name == 'workflow_dispatch' && 'release-trial' || 'release' }}",
      "cancel-in-progress": false,
    });
  });

});

describe("which push is a release (R-184, R-198)", () => {
  test("R-184 the first job decides with release-decision, from the feeds of today and the tip of main before the push", () => {
    const text = runText(job("commit"));
    for (const feed of ["darwin-aarch64", "darwin-x86_64", "linux-x86_64"]) expect(text).toContain(`published ${feed}`);
    expect(text).toContain('sh deploy/bin/release-decision --before "$BEFORE" "$sha" desktop "$mac_arm" "$mac_intel" "$linux"');
    expect(job("commit").steps.find((step) => step.id === "read")?.env?.["BEFORE"]).toBe("${{ github.event.before }}");
    expect(text).toContain("404) echo none ;;");
    expect(text).toMatch(/\*\) echo "::error::The feed \$1 answered \$code\." >&2; return 1 ;;/);
  });

  test("when the decision is nothing, no job builds, signs or publishes", () => {
    for (const name of ["tool", "macos", "linux"]) {
      expect(job(name).if).toBe("needs.commit.outputs.release == 'true'");
    }
    expect(job("gates").if).toBe("needs.commit.outputs.release == 'true' || (needs.commit.outputs.release_ios == 'true' && vars.ANIMA_PUBLIC_TESTFLIGHT == 'on')");
    for (const name of ["sign", "publish", "trial"]) {
      expect(waitsFor(name).has("macos")).toBe(true);
      expect(job(name).if ?? "").not.toMatch(/always\(\)|cancelled\(\)|failure\(\)/);
    }
  });

  test("the switch of the operator: with the variable not on, a push builds nothing and publish cannot run", () => {
    const read = job("commit").steps.find((step) => step.id === "read");
    expect(read?.env?.["SWITCH"]).toBe("${{ vars.ANIMA_PUBLIC_RELEASES }}");
    expect(runText(job("commit"))).toMatch(/if \[ "\$release" = true \] && \[ "\$SWITCH" != on \]; then\n\s+release=false/);
    expect(job("publish").if).toContain("vars.ANIMA_PUBLIC_RELEASES == 'on'");
  });

  test("the reason of a decision is in the summary of the run", () => {
    expect(runText(job("commit"))).toContain('echo "Decision: $decided" >> "$GITHUB_STEP_SUMMARY"');
  });

});

describe("the commit that each job builds (BLD-041, R-183)", () => {
  test("BLD-041 the first job refuses a commit that is not on main, and each other job waits for it", () => {
    expect(runText(job("commit"))).toContain('git merge-base --is-ancestor "$sha" refs/remotes/origin/main');
    for (const name of Object.keys(jobs)) {
      if (name !== "commit") expect(waitsFor(name).has("commit")).toBe(true);
    }
  });

  test("R-183 the version of a release is the version of its commit, and the job has the full history for the count", () => {
    const checkout = job("commit").steps.find((step) => step.uses?.startsWith("actions/checkout@"));
    expect(checkout?.with?.["fetch-depth"]).toBe(0);
    const text = runText(job("commit"));
    expect(text).toContain('version=$(sh deploy/bin/commit-version "$sha")');
    expect(text).toContain('"release $version") release=true ;;');
    expect(text).toContain("version=$TRIAL_VERSION");
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
  test("only the jobs with a key have an environment, and each key has an environment of its own", () => {
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
    for (const name of ["sign", "bucket", "publish", "cli-publish", "ios-upload", "ios-record", "trial"]) {
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

  test("publish runs only for a push to main with the switch on, and only after every other job passed", () => {
    expect(job("publish").if).toBe("github.event_name == 'push' && github.ref == 'refs/heads/main' && vars.ANIMA_PUBLIC_RELEASES == 'on'");
    expect(job("publish").if).not.toMatch(/always\(\)|cancelled\(\)|failure\(\)/);
    expect([...waitsFor("publish")].sort()).toEqual(["bucket", "commit", "gates", "linux", "macos", "sign", "tool"]);
  });

});

describe("the bucket check and the cache of the gates", () => {
  test("a release reads the bucket with the token in its first minute, writes nothing there, and a trial never does", () => {
    const found = job("bucket");
    expect(needsOf("bucket")).toEqual(["commit"]);
    expect(found.if).toBe("needs.commit.outputs.release == 'true' && github.event_name == 'push' && github.ref == 'refs/heads/main' && vars.ANIMA_PUBLIC_RELEASES == 'on'");
    expect(found.steps.some((step) => step.uses !== undefined)).toBe(false);
    const text = runText(found);
    expect(text).toContain("aws s3api head-object");
    expect(text).not.toMatch(/put-object|delete-object|copy-object|s3 cp|s3 rm|s3 sync/);
    expect(waitsFor("trial").has("bucket")).toBe(false);
  });

  test("only the gates job has a cache: nothing that is shipped or that holds a key is built from one", () => {
    for (const [name, found] of Object.entries(jobs)) {
      const cached = found.steps.some((step) => step.uses?.startsWith("actions/cache@"));
      expect(cached).toBe(name === "gates");
      for (const step of found.steps.filter((one) => one.uses?.startsWith("oven-sh/setup-bun@"))) expect(step.with?.["no-cache"]).toBe(true);
      for (const step of found.steps.filter((one) => one.uses?.startsWith("actions/setup-go@"))) expect(step.with?.["cache"]).toBe(false);
    }
    expect(Object.keys(HOLDERS)).not.toContain("gates");
    expect(runText(job("gates"))).toMatch(/brew install [^\n]*sccache/);
  });

});

describe("the end of a run (UPD-008)", () => {
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

  test("both build jobs of the workflow ask for a release build, for a release and for a trial run alike", () => {
    for (const name of ["macos", "linux"]) {
      const build = job(name).steps.filter((step) => (step.run ?? "").includes("trial-bundle.sh"));
      expect(build).toHaveLength(1);
      expect(build[0]?.env?.["ANIMA_RELEASE_BUILD"]).toBe("1");
      expect(build[0]?.if).toBeUndefined();
      expect(job(name).if).toBe("needs.commit.outputs.release == 'true'");
    }
    expect(RAW).not.toMatch(/ANIMA_LOCAL_BUILD/);
  });
});
