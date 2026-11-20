// Rules for the jobs of `anima` alone in .github/workflows/release.yml (INS-010 to INS-012;
// R-217 to R-219; task B8h): what each may hold, and that nothing of them is public while a
// switch of the operator is off. Nothing here runs the workflow or reads a secret.
import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

import { ownCondition } from "./release-waits.ts";

interface Step {
  id?: string;
  name?: string;
  uses?: string;
  if?: string;
  run?: string;
  env?: Record<string, string>;
}
interface Job {
  needs?: string | string[];
  if?: string;
  environment?: string;
  steps: Step[];
}

const RAW = readFileSync(join(import.meta.dir, "..", "..", ".github", "workflows", "release.yml"), "utf8");
const jobs = (Bun.YAML.parse(RAW) as { jobs: Record<string, Job> }).jobs;

function job(name: string): Job {
  const found = jobs[name];
  if (found === undefined) throw new Error(`the workflow has no job ${name}`);
  return found;
}
const needs = (name: string): string[] => [job(name).needs ?? []].flat();
const text = (name: string): string => job(name).steps.map((step) => step.run ?? "").join("\n");
const BOTH_SWITCHES = "github.event_name == 'push' && github.ref == 'refs/heads/main' && vars.ANIMA_PUBLIC_RELEASES == 'on' && vars.ANIMA_PUBLIC_CLI == 'on'";

describe("the switches of `anima` alone", () => {
  test("each job that makes it public runs only for a push to main with both switches on", () => {
    for (const name of ["cli-publish", "tap", "crate"]) {
      expect(ownCondition(job(name).if, needs(name))).toBe(BOTH_SWITCHES);
      expect(needs(name)).toContain("cli");
    }
  });

  test("the decision for it needs its own switch, and a release of the desktop application in the same run", () => {
    const decide = text("commit");
    expect(decide).toMatch(/release_cli=false\n\s+if \[ "\$release" = true \]; then/);
    expect(decide).toContain('if [ "$decided_cli" = "release $version" ] && [ "$CLI_SWITCH" = on ]; then');
    expect(job("commit").steps.find((step) => step.id === "read")?.env?.["CLI_SWITCH"]).toBe("${{ vars.ANIMA_PUBLIC_CLI }}");
    expect(job("cli").if).toBe("needs.commit.outputs.release_cli == 'true'");
  });

  test("the desktop application does not wait for it: a fault of these jobs cannot stop a release", () => {
    for (const name of ["sign", "publish", "trial"]) {
      for (const other of ["cli", "cli-publish", "tap", "crate"]) expect(needs(name)).not.toContain(other);
    }
  });

  test("nothing of it is public before the desktop application of the same version", () => {
    expect(needs("cli-publish")).toContain("publish");
    expect(needs("tap")).toContain("cli-publish");
    expect(needs("crate")).toContain("cli-publish");
  });
});

describe("the files of `anima` alone (INS-010, INS-002)", () => {
  test("the job that makes them has no secret, compiles nothing, and takes each program from a checked application", () => {
    expect(job("cli").environment).toBeUndefined();
    expect(JSON.stringify(job("cli"))).not.toContain("secrets.");
    const made = text("cli");
    expect(made).not.toMatch(/(^|[\s;&|(])(cargo|rustc|go|bun) /);
    expect(made.indexOf("sha256sum --check --strict")).toBeLessThan(made.indexOf("tar -xzf macos/Anima.app.tar.gz"));
    expect(made).toContain('[ "$("$linux/anima" --version)" = "anima $VERSION" ]');
  });

  test("the bucket keeps a file with the same bytes, refuses other bytes, and gets the version file last", () => {
    const put = text("cli-publish");
    expect(put).toContain("cmp expected SHA256SUMS");
    expect(put).toContain('echo "::error::The bucket holds $2 with other bytes."');
    const last = put.lastIndexOf('put SHA256SUMS "anima/$VERSION/SHA256SUMS"');
    expect(last).toBeGreaterThan(0);
    expect(put.indexOf("--key anima/latest.json")).toBeGreaterThan(last);
    expect(put).not.toMatch(/delete-object|s3 rm|s3 sync/);
  });
});

describe("the tap and the crate (R-218, R-219)", () => {
  test("the job with the key of the tap reads no repository of code, and pushes only the formula that the job cli made", () => {
    expect(job("tap").steps.some((step) => step.uses?.startsWith("actions/checkout@"))).toBe(false);
    const push = text("tap");
    expect(push.indexOf('echo "$FORMULA_SHA256  cli/anima.rb" | sha256sum --check --strict')).toBeLessThan(push.indexOf("git clone"));
    expect(push).toContain("StrictHostKeyChecking=yes");
    expect(push).toContain("git@github.com:TheArctesian/homebrew-anima.git");
    expect(push).toContain("git push --quiet origin HEAD:refs/heads/main");
  });

  test("the token of crates.io is in one step, and that step builds nothing", () => {
    const holders = job("crate").steps.filter((step) => JSON.stringify(step).includes("secrets."));
    expect(holders).toHaveLength(1);
    expect(holders[0]?.run).toContain("cargo publish -p anima-cli --locked --allow-dirty --no-verify");
    expect(holders[0]?.run).not.toMatch(/cargo (build|package|test|run)|crate-version|apt-get/);
    expect(holders[0]?.if).toBe("steps.package.outputs.there == 'false'");
  });

  test("the crate gets the version of the release, and a version that crates.io already has is not sent again", () => {
    const pack = job("crate").steps.find((step) => step.id === "package")?.run ?? "";
    expect(pack).toContain('sh deploy/bin/crate-version "$VERSION"');
    expect(pack).toContain("cargo package -p anima-cli --locked --allow-dirty");
    expect(pack.indexOf("https://index.crates.io/an/im/anima-cli")).toBeLessThan(pack.indexOf("crate-version"));
  });
});
