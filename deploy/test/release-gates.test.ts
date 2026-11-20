// The gates of a release (BLD-042, R-190): the release calls the workflow `gates`, does not
// call it for a content that passed it on `dev` (deploy/bin/gated-run), and runs the tests
// that only a Mac runs (deploy/bin/macos-tests). Nothing here runs a workflow.
import { describe, expect, test } from "bun:test";
import { mkdtempSync, readdirSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { ownCondition } from "./release-waits.ts";

const ROOT = join(import.meta.dir, "..", "..");
const GATED_RUN = join(ROOT, "deploy", "bin", "gated-run");
const REPOSITORY = "owner/anima";

interface Job {
  needs?: string | string[];
  if?: string;
  uses?: string;
  with?: Record<string, string>;
  permissions?: Record<string, string>;
  steps?: { id?: string; if?: string; run?: string; uses?: string; env?: Record<string, string>; with?: Record<string, unknown> }[];
}
const read = (...path: string[]): string => readFileSync(join(ROOT, ...path), "utf8");
const jobs = (Bun.YAML.parse(read(".github", "workflows", "release.yml")) as { jobs: Record<string, Job> }).jobs;
const job = (name: string): Job => {
  const found = jobs[name];
  if (found === undefined) throw new Error(`the workflow has no job ${name}`);
  return found;
};
const needs = (name: string): string[] => [job(name).needs ?? []].flat();
const text = (name: string): string => (job(name).steps ?? []).map((step) => step.run ?? "").join("\n");
const WHEN = "needs.commit.outputs.release == 'true' || (needs.commit.outputs.release_ios == 'true' && vars.ANIMA_PUBLIC_TESTFLIGHT == 'on')";

describe("the gates jobs of the release", () => {
  test("the 21 gates are the workflow `gates` of this commit of the repository, called for the release commit", () => {
    const linux = job("gates-linux");
    expect(linux.uses).toBe("./.github/workflows/gates.yml");
    expect(linux.with).toEqual({ ref: "${{ needs.commit.outputs.sha }}" });
    expect(linux.permissions).toEqual({ contents: "read" });
    expect(linux.steps).toBeUndefined();
    const called = Bun.YAML.parse(read(".github", "workflows", "gates.yml")) as { on: { workflow_call: { inputs: Record<string, unknown> } }; jobs: Record<string, unknown> };
    expect(Object.keys(called.on.workflow_call.inputs)).toContain("ref");
    expect(Object.keys(called.jobs)).toContain("passed");
    expect(read(".github", "workflows", "gates.yml")).not.toMatch(/secrets\.|:\s*write\b/);
  });

  test("they are skipped only when the first job found a run for the same content", () => {
    expect(job("gates-linux").if).toBe(`(${WHEN}) && needs.commit.outputs.gated != 'true'`);
    expect(job("gates-macos").if).toBe(WHEN);
    expect(text("gates-macos")).toContain("sh deploy/bin/macos-tests");
  });

  test("the job `gates` passes only with the Mac tests, and with the Linux gates passed or found on dev", () => {
    const gates = job("gates");
    expect(needs("gates").sort()).toEqual(["commit", "gates-linux", "gates-macos"]);
    expect(gates.if).toBe(`always() && (${WHEN})`);
    const run = text("gates");
    expect(run).toContain('[ "$MACOS" = success ] ||');
    expect(run).toMatch(/case "\$LINUX:\$GATED" in\n\s*success:\*\) ;;\n\s*skipped:true\) ;;\n\s*\*\) echo "::error::The gates did not pass\."; exit 1 ;;/);
    expect(gates.steps?.[0]?.env).toEqual({
      GATED: "${{ needs.commit.outputs.gated }}",
      LINUX: "${{ needs.gates-linux.result }}",
      MACOS: "${{ needs.gates-macos.result }}",
    });
  });

  test("only `gates` runs whatever came before it; each job after it names the result of each job that it waits for", () => {
    const after = (name: string): boolean => needs(name).some((one) => one === "gates" || after(one));
    const waiting = Object.keys(jobs).filter(after);
    expect(waiting.sort()).toEqual(["cli-publish", "crate", "ios-record", "ios-upload", "publish", "sign", "tap", "trial"]);
    for (const name of waiting) expect(() => ownCondition(job(name).if, needs(name))).not.toThrow();
    for (const [name, found] of Object.entries(jobs)) {
      if (name !== "gates") expect(found.if ?? "").not.toContain("always()");
    }
    expect(ownCondition(job("sign").if, needs("sign"))).toBe("");
    expect(needs("sign")).toContain("gates");
  });

  test("the first job asks GitHub, and any fault means that the gates run", () => {
    const step = job("commit").steps?.find((one) => one.id === "gated");
    expect(step?.env?.["GH_TOKEN"]).toBe("${{ github.token }}");
    expect(job("commit").permissions).toEqual({ contents: "read", actions: "read" });
    const run = step?.run ?? "";
    expect(run).toContain("actions/workflows/gates.yml/runs?branch=dev&status=success");
    expect(run).toContain('sh deploy/bin/gated-run "$REPOSITORY" "$SHA" < runs.json');
    expect(run).toContain("sh deploy/bin/gated-run --passed < jobs.json");
    expect(run.match(/gated=true/g)).toHaveLength(1);
    expect(run.indexOf("gated=false")).toBeLessThan(run.indexOf("read_api \"actions"));
    expect(run).toContain('if [ "$TRIAL_GATES" = run ]; then');
  });
});

/** A repository with two commits of one tree (as `dev` and its merge into `main`) and one of another tree. */
function repository(): { folder: string; dev: string; main: string; other: string } {
  const folder = mkdtempSync(join(tmpdir(), "gated-run-"));
  const git = (...args: string[]): string => {
    const done = Bun.spawnSync(["git", "-c", "user.name=t", "-c", "user.email=t@t.invalid", ...args], { cwd: folder });
    if (done.exitCode !== 0) throw new Error(done.stderr.toString());
    return done.stdout.toString().trim();
  };
  git("init", "-q", "-b", "main");
  git("commit", "-q", "--allow-empty", "-m", "first");
  git("switch", "-q", "-c", "dev");
  Bun.write(join(folder, "file"), "one\n");
  git("add", "file");
  git("commit", "-q", "-m", "work");
  const dev = git("rev-parse", "HEAD");
  git("switch", "-q", "main");
  git("merge", "-q", "--no-ff", "-m", "merge dev", "dev");
  const main = git("rev-parse", "HEAD");
  git("commit", "-q", "--allow-empty", "-m", "same tree again");
  git("switch", "-q", "dev");
  git("commit", "-q", "--allow-empty", "-m", "x");
  Bun.write(join(folder, "file"), "two\n");
  git("commit", "-q", "-am", "more work");
  return { folder, dev, main, other: git("rev-parse", "HEAD") };
}

function run(at: { head_sha: string } & Record<string, unknown>): Record<string, unknown> {
  return {
    id: 7,
    path: ".github/workflows/gates.yml",
    head_branch: "dev",
    event: "push",
    status: "completed",
    conclusion: "success",
    repository: { full_name: REPOSITORY },
    head_repository: { full_name: REPOSITORY },
    ...at,
  };
}

function gated(folder: string, commit: string, runs: unknown): { exitCode: number; printed: string } {
  const done = Bun.spawnSync(["sh", GATED_RUN, REPOSITORY, commit], { cwd: folder, stdin: Buffer.from(typeof runs === "string" ? runs : JSON.stringify({ workflow_runs: runs })) });
  return { exitCode: done.exitCode, printed: done.stdout.toString().trim() };
}

describe("gated-run: which run of the gates counts for a commit", () => {
  const made = repository();
  const { folder, dev, main, other } = made;

  test("a run that passed on dev for a commit with the same tree counts, also when the commit is not the same", () => {
    expect(gated(folder, main, [run({ head_sha: dev })])).toEqual({ exitCode: 0, printed: `7 ${dev}` });
    expect(gated(folder, main, [run({ head_sha: dev, event: "workflow_dispatch" })]).printed).toBe(`7 ${dev}`);
    expect(gated(folder, main, [run({ head_sha: other, id: 8 }), run({ head_sha: dev })]).printed).toBe(`7 ${dev}`);
  });

  test("a run for a different content does not count", () => {
    expect(gated(folder, main, [run({ head_sha: other })])).toEqual({ exitCode: 0, printed: "" });
    expect(gated(folder, other, [run({ head_sha: dev })]).printed).toBe("");
    expect(gated(folder, main, []).printed).toBe("");
  });

  test.each([
    ["a different branch", { head_branch: "release/x" }],
    ["the branch main", { head_branch: "main" }],
    ["a pull request", { event: "pull_request" }],
    ["a call from a different workflow", { event: "workflow_call" }],
    ["a run that failed", { conclusion: "failure" }],
    ["a run that was cancelled", { conclusion: "cancelled" }],
    ["a run that was skipped", { conclusion: "skipped" }],
    ["a run in progress", { status: "in_progress", conclusion: null }],
    ["a different workflow file", { path: ".github/workflows/other.yml" }],
    ["a fork", { head_repository: { full_name: "other/anima" } }],
    ["a different repository", { repository: { full_name: "other/anima" } }],
    ["a commit that the repository does not hold", { head_sha: "0123456789abcdef0123456789abcdef01234567" }],
    ["a commit name that is no hash", { head_sha: "dev" }],
  ])("%s does not count", (_, change) => {
    expect(gated(folder, main, [run({ head_sha: dev, ...change })])).toEqual({ exitCode: 0, printed: "" });
  });

  test("an answer that cannot be read counts as no run, and a name that is no commit is refused", () => {
    expect(gated(folder, main, "not json")).toEqual({ exitCode: 0, printed: "" });
    expect(gated(folder, main, '{"message":"Not Found"}')).toEqual({ exitCode: 0, printed: "" });
    expect(gated(folder, "no-such-commit", [run({ head_sha: dev })]).exitCode).toBe(2);
    rmSync(folder, { recursive: true, force: true });
  });

  test("--passed is true only for one job `passed` with the result success", () => {
    const passed = (jobsOf: unknown): number => Bun.spawnSync(["sh", GATED_RUN, "--passed"], { stdin: Buffer.from(JSON.stringify(jobsOf)) }).exitCode;
    const done = { name: "passed", status: "completed", conclusion: "success" };
    expect(passed({ jobs: [{ name: "go", status: "completed", conclusion: "success" }, done] })).toBe(0);
    expect(passed({ jobs: [{ ...done, conclusion: "failure" }] })).not.toBe(0);
    expect(passed({ jobs: [{ ...done, conclusion: "skipped" }] })).not.toBe(0);
    expect(passed({ jobs: [{ name: "go", status: "completed", conclusion: "success" }] })).not.toBe(0);
    expect(passed({ message: "Not Found" })).not.toBe(0);
    expect(Bun.spawnSync(["sh", GATED_RUN, "--passed"], { stdin: Buffer.from("x") }).exitCode).not.toBe(0);
  });
});

/** The packages of the workspace that hold Rust code for macOS only, by the name in their Cargo.toml. */
function packagesWithMacCode(): string[] {
  const holds = (folder: string): boolean =>
    readdirSync(folder, { withFileTypes: true }).some((entry) => {
      if (entry.name === "target" || entry.name === "node_modules" || entry.name === "gen") return false;
      const path = join(folder, entry.name);
      if (entry.isDirectory()) return holds(path);
      return entry.name.endsWith(".rs") && /target_os = "macos"|target_vendor = "apple"/.test(readFileSync(path, "utf8"));
    });
  const crates = readdirSync(join(ROOT, "crates"), { withFileTypes: true }).filter((entry) => entry.isDirectory()).map((entry) => join(ROOT, "crates", entry.name));
  return [...crates, join(ROOT, "apps", "desktop", "src-tauri")]
    .filter((folder) => holds(folder))
    .map((folder) => /^name = "([^"]+)"/m.exec(readFileSync(join(folder, "Cargo.toml"), "utf8"))?.[1] ?? folder)
    .sort();
}

describe("macos-tests: what only a Mac runs", () => {
  const script = read("deploy", "bin", "macos-tests");

  test("it tests each package that holds Rust code for macOS only", () => {
    const listed = /^PACKAGES="([^"]+)"$/m.exec(script)?.[1]?.split(" ") ?? [];
    expect(listed).toEqual(packagesWithMacCode());
  });

  test("it runs the bun tests that skip off macOS, the access lists of the link, and the pinning test", () => {
    const skipping = readdirSync(join(ROOT, "deploy", "test")).filter((name) => name.endsWith(".test.ts") && /skipIf\(process\.platform !== "darwin"\)/.test(read("deploy", "test", name)));
    expect(skipping.length).toBeGreaterThan(0);
    for (const name of skipping) expect(script).toContain(`deploy/test/${name}`);
    expect(script).toContain("go -C link test -race ./statedir/...");
    expect(script).toContain("cargo test -p anima-app --test pinning");
    expect(script).toContain("unset DATABASE_URL");
  });
});
