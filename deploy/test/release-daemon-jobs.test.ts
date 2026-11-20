// Rules for the jobs of the daemon alone in .github/workflows/release.yml (task U2; ROL-002 to
// ROL-005): where its file comes from, who signs its record, and that nothing of it is public
// before the desktop application of the same run. Nothing here runs the workflow or reads a
// secret; no run has proved these jobs.
import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

import { ownCondition } from "./release-waits.ts";

interface Step {
  name?: string;
  uses?: string;
  run?: string;
  env?: Record<string, string>;
  with?: Record<string, string>;
}
interface Job {
  needs?: string | string[];
  if?: string;
  environment?: string;
  outputs?: Record<string, string>;
  steps: Step[];
}

const ROOT = join(import.meta.dir, "..", "..");
const RAW = readFileSync(join(ROOT, ".github", "workflows", "release.yml"), "utf8");
const jobs = (Bun.YAML.parse(RAW) as { jobs: Record<string, Job> }).jobs;

function job(name: string): Job {
  const found = jobs[name];
  if (found === undefined) throw new Error(`the workflow has no job ${name}`);
  return found;
}
const needs = (name: string): string[] => [job(name).needs ?? []].flat();
const text = (name: string): string => job(name).steps.map((step) => step.run ?? "").join("\n");
const PUBLIC = "github.event_name == 'push' && github.ref == 'refs/heads/main' && vars.ANIMA_PUBLIC_RELEASES == 'on'";

describe("the daemon alone in a release", () => {
  test("ROL-002 its file is the program that the build job put in each application; the job builds nothing and has no secret", () => {
    expect(needs("daemon").sort()).toEqual(["commit", "linux", "macos"]);
    expect(job("daemon").if).toBe("needs.commit.outputs.release == 'true'");
    expect(job("daemon").environment).toBeUndefined();
    const run = text("daemon");
    expect(run).toContain("tar -xzf macos/Anima.app.tar.gz -C work/mac Anima.ing.app/Contents/MacOS/anima-daemon");
    expect(run).toContain("--appimage-extract usr/bin/anima-daemon");
    expect(run).toContain("Mach-O universal binary with 2 architectures");
    expect(run).toMatch(/ldd work\/linux\/squashfs-root\/usr\/bin\/anima-daemon \| grep 'not found'/);
    expect(run).not.toMatch(/(^|[\s;&|(])(cargo|bun|go|git) /);
    expect(Object.keys(job("daemon").outputs ?? {}).sort()).toEqual(["linux_sha256", "macos_sha256"]);
  });

  test("ROL-004 the job that holds the updater key signs each record, after it compared the two files with their digests", () => {
    const sign = text("sign");
    expect(sign).toContain('echo "$DAEMON_MACOS_SHA256  daemon/darwin-universal/anima-daemon"');
    expect(sign).toContain('echo "$DAEMON_LINUX_SHA256  daemon/linux-x86_64/anima-daemon"');
    expect(sign.indexOf("sha256sum --check --strict")).toBeLessThan(sign.indexOf("release record linux-x86_64"));
    expect(sign).toContain('tool/anima-hub release record linux-x86_64 "$VERSION" daemon/linux-x86_64');
    expect(sign).toContain('tool/anima-hub release record darwin-universal "$VERSION" daemon/darwin-universal');
    const step = job("sign").steps.find((one) => (one.run ?? "").includes("release record"));
    expect(step?.env).toMatchObject({
      ANIMA_RELEASE_COMMIT: "${{ needs.commit.outputs.sha }}",
      ANIMA_RELEASE_REPLACES_FROM: "${{ needs.commit.outputs.replaces_from }}",
      SOURCE_DATE_EPOCH: "${{ needs.commit.outputs.time }}",
    });
  });

  test("ROL-041 the lowest version that a release can replace is one line of the repository, read by the first job", () => {
    const value = readFileSync(join(ROOT, "deploy", "release", "replaces-from"), "utf8")
      .split("\n")
      .filter((line) => line !== "" && !line.startsWith("#"));
    expect(value).toEqual(["0.0.0"]);
    expect(text("commit")).toContain("grep -v '^#' deploy/release/replaces-from");
    expect(text("commit")).toContain('echo "replaces_from=$replaces_from"');
  });

  test("ROL-005 nothing of it is public before the desktop application of the run, and only with the switch on", () => {
    expect(needs("daemon-publish").sort()).toEqual(["commit", "daemon", "publish", "sign"]);
    expect(ownCondition(job("daemon-publish").if, needs("daemon-publish"))).toBe(PUBLIC);
    expect(job("daemon-publish").environment).toBe("release-publish");
    for (const name of ["sign", "publish", "trial"]) expect(needs(name)).not.toContain("daemon-publish");
  });

  test("ROL-005 for each target the program comes first, then the record, then the signature of the record", () => {
    const run = text("daemon-publish");
    const order = ["anima-daemon\" \"$target/$VERSION/anima-daemon\"", "release-record.json\" \"$target/$VERSION/release-record.json\"", "release-record.json.sig\" \"$target/$VERSION/release-record.json.sig\""].map((key) => run.indexOf(key));
    expect(order.every((at) => at > 0)).toBe(true);
    expect([...order].sort((a, b) => a - b)).toEqual(order);
    expect(run).toContain("for target in linux-x86_64 darwin-universal; do");
    expect(run).toContain('echo "::error::The bucket holds $2 with other bytes."');
    expect(run).not.toMatch(/delete-object|copy-object|latest\.json/);
  });
});
