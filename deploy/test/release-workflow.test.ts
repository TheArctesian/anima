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
  with?: Record<string, string>;
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
    expect(needsOf(publish).sort()).toEqual(["build-macos", "sign-linux", "version"]);
  });

  test("UPD-009 publish runs after a failed build only when the version job and one build passed", () => {
    const condition = publish.if ?? "";
    expect(condition).toContain("!cancelled()");
    expect(condition).not.toContain("always()");
    expect(condition).toContain("needs.version.result == 'success'");
    expect(condition).toContain("needs.build-macos.result == 'success'");
    expect(condition).toContain("needs.sign-linux.result == 'success'");
    expect(condition).not.toContain("needs.build-linux");
  });

  test("UPD-012 the Linux key job waits for the Linux build and the version", () => {
    expect(needsOf(jobs["sign-linux"] as Job).sort()).toEqual(["build-linux", "version"]);
  });

  test("UPD-009 each artifact is downloaded only when its build succeeded", () => {
    const downloads = publish.steps.filter((step) => step.uses?.startsWith("actions/download-artifact@"));
    expect(downloads.map((step) => step.if).sort()).toEqual([
      "needs.build-macos.result == 'success'",
      "needs.sign-linux.result == 'success'",
    ]);
  });

  test("UPD-008 the release is a draft, latest.json is uploaded after the other files, and publishing is last", () => {
    const at = (pattern: RegExp) => publish.steps.findIndex((step) => pattern.test(step.run ?? ""));
    const loop = publish.steps.find((step) => /gh release upload .*\$file/.test(step.run ?? ""));
    expect(loop?.run).toMatch(/= latest\.json \] && continue/);
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
    expect(text).toContain('if [ -z "$pubkey" ] || [ "$pubkey" = "REPLACE_WITH_OPERATOR_PUBLIC_KEY" ]; then');
    expect(text).toContain('if [ "$KEY_IS_SET" != "true" ]; then');
    expect(text.match(/\bexit 1\b/g)).toHaveLength(2);
    expect(text).toContain("plugins.updater.pubkey");
    const step = (jobs["version"] as Job).steps.find((s) => s.env?.["KEY_IS_SET"] !== undefined);
    expect(step?.env?.["KEY_IS_SET"]).toBe("${{ secrets.TAURI_SIGNING_PRIVATE_KEY != '' }}");
  });

  test("UPD-012 builds wait for the version job", () => {
    for (const name of ["build-macos", "build-linux"]) expect(needsOf(jobs[name] as Job)).toEqual(["version"]);
  });

  test("UPD-012 only the macOS bundle step and the Linux sign job carry a secret", () => {
    for (const [name, job] of Object.entries(jobs)) {
      for (const step of job.steps) {
        const secrets = Object.values(step.env ?? {}).filter((v) => v.includes("secrets."));
        const holdsKey =
          (name === "build-macos" && (step.run ?? "").includes("tauri bundle")) ||
          (name === "sign-linux" && (step.run ?? "").includes("signer sign"));
        const checksKey = name === "version" && secrets.length === 1 && !/secrets\.\w+\s*\}\}/.test(secrets[0] ?? "");
        expect(secrets.length > 0).toBe(name === "version" ? checksKey : holdsKey);
      }
    }
  });

  test("UPD-012 each build compiles with --no-bundle and bundles without a second build", () => {
    for (const name of ["build-macos", "build-linux"]) {
      const runs = (jobs[name] as Job).steps.map((step) => step.run ?? "");
      if (name === "build-linux") expect(runs.join("\n")).not.toContain("signer sign");
      const compile = runs.findIndex((r) => r.includes("tauri build --no-bundle"));
      const bundle = runs.findIndex((r) => r.includes("tauri bundle"));
      expect(compile).toBeGreaterThanOrEqual(0);
      expect(bundle).toBeGreaterThan(compile);
      expect(runs.filter((r) => /tauri build(?! --no-bundle)/.test(r))).toEqual([]);
    }
    expect(runText(jobs["build-macos"] as Job)).toContain("--target universal-apple-darwin --bundles app dmg");
    expect(runText(jobs["build-linux"] as Job)).toContain("tauri bundle --bundles appimage --no-sign");
  });

  test("UPD-012 the signing key is set on the bundle step with its password", () => {
    for (const [name, command] of [["build-macos", "tauri bundle"], ["sign-linux", "signer sign"]] as const) {
      const step = (jobs[name] as Job).steps.find((s) => (s.run ?? "").includes(command));
      expect(step?.env?.["TAURI_SIGNING_PRIVATE_KEY"]).toBe("${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}");
      expect(step?.env?.["TAURI_SIGNING_PRIVATE_KEY_PASSWORD"]).toBe(
        "${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}",
      );
    }
  });

  test("UPD-012 no step echoes a secret", () => {
    expect(RAW).not.toMatch(/echo[^\n]*(secrets\.|\$\{?(TAURI_SIGNING|APPLE_))/);
  });
});

describe("manual runs and environment (UPD-007, UPD-012)", () => {
  test("UPD-007 the version job runs only on main, and every other job needs it", () => {
    expect((jobs["version"] as Job).if).toBe("github.ref == 'refs/heads/main'");
    for (const name of ["build-macos", "build-linux", "publish"]) expect(needsOf(jobs[name] as Job)).toContain("version");
  });

  test("UPD-012 every job that reads a secret or publishes uses the release environment", () => {
    for (const job of Object.values(jobs)) expect(job.environment).toBe("release");
  });
});

describe("artifacts, tokens and version format (UPD-008, UPD-009)", () => {
  const uses = (job: Job, action: string) => job.steps.filter((s) => s.uses?.startsWith(`${action}@`));

  test("UPD-009 the macOS and Linux uploads name the bundle paths and fail when empty", () => {
    const mac = uses(jobs["build-macos"] as Job, "actions/upload-artifact")[0]?.with;
    const lin = uses(jobs["build-linux"] as Job, "actions/upload-artifact")[0]?.with;
    expect(mac?.["name"]).toBe("macos");
    expect(mac?.["path"].split("\n").filter(Boolean)).toEqual([
      "target/universal-apple-darwin/release/bundle/macos/*.app.tar.gz",
      "target/universal-apple-darwin/release/bundle/macos/*.app.tar.gz.sig",
      "target/universal-apple-darwin/release/bundle/dmg/*.dmg",
    ]);
    expect(lin?.["name"]).toBe("linux-unsigned");
    expect(lin?.["path"]).toBe("target/release/bundle/appimage/*.AppImage");
    const signed = uses(jobs["sign-linux"] as Job, "actions/upload-artifact")[0]?.with;
    expect(signed?.["name"]).toBe("linux");
    expect(signed?.["path"].split("\n").filter(Boolean)).toEqual(["unsigned/*.AppImage", "unsigned/*.AppImage.sig"]);
    expect(signed?.["if-no-files-found"]).toBe("error");
    expect(mac?.["if-no-files-found"]).toBe("error");
    expect(lin?.["if-no-files-found"]).toBe("error");
  });

  test("UPD-009 the downloads put each platform in its own folder for make-feed", () => {
    const paths = uses(publish, "actions/download-artifact").map((s) => `${s.with?.["name"]}:${s.with?.["path"]}`);
    expect(paths.sort()).toEqual(["linux:artifacts/linux", "macos:artifacts/macos"]);
  });

  test("UPD-008 every checkout drops the git credential", () => {
    const checkouts = allSteps.filter((s) => s.uses?.startsWith("actions/checkout@"));
    expect(checkouts).toHaveLength(5);
    for (const step of checkouts) expect(String(step.with?.["persist-credentials"])).toBe("false");
  });

  test("UPD-007 the version is 0.1.<run number> and the tag is v0.1.<run number>", () => {
    const step = (jobs["version"] as Job).steps.find((s) => s.run?.includes("GITHUB_OUTPUT"));
    expect(step?.env?.["RUN_NUMBER"]).toBe("${{ github.run_number }}");
    expect(step?.run).toContain('echo "version=0.1.${RUN_NUMBER}"');
    expect(step?.run).toContain('echo "tag=v0.1.${RUN_NUMBER}"');
  });

  test("UPD-012 the GitHub token is set only on steps that run gh", () => {
    expect(publish.env?.["GH_TOKEN"]).toBeUndefined();
    for (const step of allSteps) {
      expect(step.env?.["GH_TOKEN"] !== undefined).toBe(/\bgh release\b/.test(step.run ?? ""));
    }
  });
});

describe("operator guide (UPD-012, DNS-007)", () => {
  const guide = readFileSync(join(ROOT, "docs", "releases.md"), "utf8");

  test("UPD-012 the guide never tells the operator to run bunx tauri", () => {
    expect(guide).not.toMatch(/bunx tauri|npx tauri/);
    expect(guide).toContain("bun run --cwd apps/desktop tauri signer generate");
  });

  test("UPD-012 secrets are set in the release environment, never as repository secrets", () => {
    const sets = guide.match(/gh secret set[^\n]*/g) ?? [];
    expect(sets.length).toBeGreaterThan(0);
    for (const line of sets) expect(line).toContain("--env release");
  });
});

describe("Linux signing and version binding (UPD-012, UPD-009)", () => {
  const linux = (jobs["build-linux"] as Job).steps;
  const signJob = jobs["sign-linux"] as Job;
  const stepWith = (steps: Step[], text: string) => steps.find((step) => (step.run ?? "").includes(text));

  test("UPD-012 the Linux build job that downloads linuxdeploy holds no key, no secret and does not sign", () => {
    const bundle = stepWith(linux, "tauri bundle");
    expect(bundle?.run).toContain("--no-sign");
    expect(JSON.stringify(linux.map((step) => step.env ?? {}))).not.toMatch(/secrets\.|SIGNING/);
  });

  test("UPD-012 sign-linux installs the CLI from the lockfile with no scripts, before it gets the key", () => {
    const install = stepWith(signJob.steps, "bun install");
    expect(install?.run).toContain("--frozen-lockfile");
    expect(install?.run).toContain("--ignore-scripts");
    expect(install?.run).toContain("--filter @anima/desktop");
    expect(install?.env).toBeUndefined();
    expect(signJob.steps.indexOf(install as Step)).toBeLessThan(signJob.steps.indexOf(stepWith(signJob.steps, "signer sign") as Step));
    expect(runText(signJob)).not.toContain("tauri build");
  });

  test("UPD-012 the Linux sign step binds the signature to the version and holds only the signing key", () => {
    const sign = stepWith(signJob.steps, "signer sign");
    expect(sign?.run).toContain('--app-version "$VERSION"');
    expect(Object.keys(sign?.env ?? {}).sort()).toEqual([
      "TAURI_SIGNING_PRIVATE_KEY",
      "TAURI_SIGNING_PRIVATE_KEY_PASSWORD",
      "VERSION",
    ]);
    const download = signJob.steps.find((step) => step.uses?.startsWith("actions/download-artifact@"));
    expect(download?.with?.["name"]).toBe("linux-unsigned");
    expect(signJob.steps.indexOf(download as Step)).toBeLessThan(signJob.steps.indexOf(sign as Step));
  });

  test("UPD-009 every tauri build and bundle step passes the version from the version job", () => {
    for (const name of ["build-macos", "build-linux"]) {
      const steps = (jobs[name] as Job).steps.filter((s) => /tauri (build|bundle)/.test(s.run ?? ""));
      expect(steps.length).toBe(2);
      for (const step of steps) {
        expect(step.env?.["VERSION"]).toBe("${{ needs.version.outputs.version }}");
        expect(step.run).toContain('"version\\":\\"${VERSION}\\"');
      }
    }
  });

  test("UPD-012 tauri.conf.json has no beforeBundleCommand, which would run beside the key", () => {
    const conf = JSON.parse(readFileSync(join(ROOT, "apps", "desktop", "src-tauri", "tauri.conf.json"), "utf8")) as {
      build: Record<string, unknown>;
    };
    expect(conf.build["beforeBundleCommand"]).toBeUndefined();
  });
});
