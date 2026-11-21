import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

interface Step { name?: string; uses?: string; run?: string; if?: string; with?: Record<string, unknown> }
interface Workflow {
  on: Record<string, unknown>;
  permissions: Record<string, string>;
  jobs: Record<string, { permissions: Record<string, string>; environment?: string; needs?: string[]; steps: Step[] }>;
}
const root = join(import.meta.dir, "..", "..");
const raw = readFileSync(join(root, ".github/workflows/windows-trial.yml"), "utf8");
const workflow = Bun.YAML.parse(raw) as Workflow;

test("Windows dev trials are secret-free, read-only and never publish", () => {
  expect(Object.keys(workflow.on).sort()).toEqual(["workflow_call", "workflow_dispatch"]);
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
  const bundle = workflow.jobs["windows"]?.steps ?? [];
  const build = bundle.findIndex((step) => step.run?.includes("windows-bundle.ps1"));
  const install = bundle.findIndex((step) => step.run?.includes("deploy/windows/install.ps1"));
  const installed = bundle.findIndex((step) => step.with?.["name"] === "windows-installed");
  expect(build).toBeGreaterThanOrEqual(0);
  expect(install).toBeGreaterThan(build);
  expect(installed).toBeGreaterThan(install);
  expect(bundle[build]?.run).not.toContain("-Release");
  expect(workflow.jobs["windows-gates"]?.steps.some((step) => step.run?.includes("bun run gates"))).toBe(true);
  // The trial appears only when the installer was installed and checked and the gates passed.
  const trial = workflow.jobs["windows-trial"];
  expect(trial?.needs?.sort()).toEqual(["windows", "windows-gates"]);
  for (const name of ["windows-trial", "windows-trial-cli"]) {
    const artifact = trial?.steps.find((step) => step.with?.["name"] === name);
    expect(artifact?.if).toBeUndefined();
    expect(artifact?.with?.["if-no-files-found"]).toBe("error");
  }
  for (const job of Object.values(workflow.jobs)) {
    expect(job.steps.some((step) => step.with?.["name"] === "windows-trial" && step.uses?.startsWith("actions/upload-artifact@"))).toBe(job === trial);
  }
});

test("the Windows gates use the pinned cargo-nextest of the Linux gates and check its digest", () => {
  const install = workflow.jobs["windows-gates"]?.steps.find((step) => step.run?.includes("cargo-nextest"))?.run ?? "";
  const linux = /cargo-nextest@([\d.]+)/.exec(readFileSync(join(root, ".github/workflows/gates.yml"), "utf8"))?.[1];
  expect(install).toContain(`/cargo-nextest-${linux}/cargo-nextest-${linux}-x86_64-pc-windows-msvc.zip`);
  expect(install).toMatch(/Get-FileHash .* -ne '[0-9a-f]{64}'/s);
  expect(install).not.toContain("cargo install");
});

test("only a run started by hand goes on after a failed gate, and it still fails at the end", () => {
  const step = workflow.jobs["windows-gates"]?.steps.find((each) => each.run?.includes("bun run gates"));
  expect((step as { env?: Record<string, string> } | undefined)?.env?.["EVENT"]).toBe("${{ github.event_name }}");
  expect(step?.run).toContain("$keepGoing = @(); if ($proof) { $keepGoing = @('--keep-going') }");
  expect(step?.run).toContain("if ($gates -ne 0 -and -not $proof) { throw 'Windows gates failed' }");
  expect(step?.run?.trimEnd().endsWith("if ($gates -ne 0) { throw 'Windows gates failed' }")).toBe(true);
});

test("the Windows installer carries the seven programs, and a Windows cargo build needs none of them", () => {
  const conf = (name: string) => JSON.parse(readFileSync(join(root, "apps/desktop/src-tauri", name), "utf8")) as { bundle: Record<string, unknown> };
  // tauri-build reads tauri.windows.conf.json on each Windows build, the gates too: it must name no program.
  expect(conf("tauri.windows.conf.json").bundle["externalBin"]).toBeUndefined();
  expect(conf("tauri.windows.conf.json").bundle["resources"]).toBeUndefined();
  const carried = conf("tauri.windows-carried.json").bundle;
  const programs = ["anima", "anima-daemon", "anima-hub", "anima-local-hub", "anima-database", "anima-gateway", "anima-link"];
  expect((carried["externalBin"] as string[]).toSorted()).toEqual(programs.map((program) => `binaries/${program}`).toSorted());
  expect(carried["resources"]).toEqual({ "database/": "database/" });
  // The bundle script builds each program it names under that name, and passes the file to tauri build.
  const script = readFileSync(join(root, "deploy/bin/windows-bundle.ps1"), "utf8");
  expect(script).toContain("'--config', 'src-tauri/tauri.windows-carried.json'");
  expect(script).toContain("$binaries/anima-link-x86_64-pc-windows-msvc.exe");
  const copied = /foreach \(\$program in @\(([^)]*)\)\) \{\s*Copy-Item/.exec(script)?.[1]?.match(/'([\w-]+)'/g)?.map((name) => name.slice(1, -1));
  expect([...(copied ?? []), "anima-link"].toSorted()).toEqual(programs.toSorted());
  // The gates job of the trial makes no stand-in programs.
  expect(workflow.jobs["windows-gates"]?.steps.some((step) => step.run?.includes("binaries"))).toBe(false);
});

interface Gates { jobs: Record<string, { env?: Record<string, string>; if?: string; with?: Record<string, string>; steps?: Step[] }> }
const gates = Bun.YAML.parse(readFileSync(join(root, ".github/workflows/gates.yml"), "utf8")) as Gates;

test("merge trains build Windows from the same dev base and provide its trial artifacts", () => {
  expect(gates.jobs["windows"]?.if).toBe("(github.ref == 'refs/heads/dev' || startsWith(github.ref, 'refs/heads/train/') || github.event_name == 'workflow_dispatch') && needs.base.outputs.documents_only != 'true'");
  expect(gates.jobs["windows"]?.with?.["dev_sha"]).toBe("${{ needs.base.outputs.dev_sha }}");
  const steps = workflow.jobs["windows"]?.steps ?? [];
  expect(steps.find((step) => step.uses?.startsWith("actions/checkout@"))?.with?.["ref"]).toBe("${{ inputs.ref }}");
});

test.skipIf(process.platform === "win32")("the gates reject failed Windows builds and never excuse a skipped Linux gate", () => {
  const script = gates.jobs["passed"]?.steps?.[0]?.run;
  expect(script).toBeDefined();
  for (const [windows, linux, success] of [["success", "success", true], ["skipped", "success", true], ["failure", "success", false], ["cancelled", "success", false], ["success", "skipped", false]] as const) {
    const results = { windows: { result: windows }, "rust-test": { result: linux } };
    const ran = Bun.spawnSync(["sh", "-c", script ?? "exit 9"], { env: { ...process.env, RESULTS: JSON.stringify(results) }, stdout: "pipe", stderr: "pipe" });
    expect(ran.exitCode === 0).toBe(success);
  }
  // A Rust job skipped because the job base said that the changes do not need it passes; any other skip fails.
  const run = (rust: string, results: object) =>
    Bun.spawnSync(["sh", "-c", script ?? "exit 9"], { env: { ...process.env, RUST_NEEDED: rust, RESULTS: JSON.stringify(results) }, stdout: "pipe", stderr: "pipe" }).exitCode;
  expect(run("false", { "rust-test": { result: "skipped" }, "rust-lint": { result: "skipped" } })).toBe(0);
  expect(run("true", { "rust-test": { result: "skipped" } })).not.toBe(0);
  expect(run("false", { bun: { result: "skipped" } })).not.toBe(0);
});

test("the Windows release uses the remote baseline present in a fresh checkout", () => {
  const release = Bun.YAML.parse(readFileSync(join(root, ".github/workflows/release.yml"), "utf8")) as Gates;
  expect(release.jobs["windows"]?.env?.["BASE_REF"]).toBe("origin/phase-2");
});
