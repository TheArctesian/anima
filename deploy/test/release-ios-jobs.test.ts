// Rules for the jobs of the iOS application in .github/workflows/release.yml (R-203; task
// B8i): its switches, and which job holds which key. Nothing here runs the workflow, builds
// the application or reads a secret; no run has proved these jobs.
import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

interface Step {
  id?: string;
  uses?: string;
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
const UPLOAD = "github.event_name == 'push' && github.ref == 'refs/heads/main' && vars.ANIMA_IOS_BUILD == 'on' && vars.ANIMA_PUBLIC_TESTFLIGHT == 'on'";

describe("the switches of the iOS application", () => {
  test("nothing of it runs while the build switch is off, in a release and in a trial", () => {
    const decide = text("commit");
    expect(decide.match(/release_ios=false\n\s*if \[ "\$IOS_BUILD" = on \]; then/g)).toHaveLength(2);
    expect(job("ios").if).toBe("needs.commit.outputs.release_ios == 'true'");
  });

  test("with the TestFlight switch off, a push builds and signs and no job that uploads can run", () => {
    expect(text("commit")).toMatch(/if \[ "\$decided_ios" = "release \$version" \]; then\n\s*release_ios=true/);
    expect(job("ios").if).not.toContain("ANIMA_PUBLIC_TESTFLIGHT");
    for (const name of ["ios-upload", "ios-record"]) expect(job(name).if).toContain("vars.ANIMA_PUBLIC_TESTFLIGHT == 'on'");
  });

  test("a build goes to TestFlight only from a push to main with the TestFlight switch on, and after the gates", () => {
    expect(job("ios-upload").if).toBe(UPLOAD);
    expect(needs("ios-upload").sort()).toEqual(["commit", "gates", "ios"]);
    expect(job("ios-record").if).toBe(UPLOAD);
    expect(needs("ios-record")).toContain("ios-upload");
  });

  test("it does not wait for the desktop application, and the desktop application does not wait for it", () => {
    expect(job("ios-upload").if).not.toContain("ANIMA_PUBLIC_RELEASES");
    for (const name of ["sign", "publish", "trial", "cli", "cli-publish", "tap", "crate"]) {
      for (const other of ["ios", "ios-upload", "ios-record"]) expect(needs(name)).not.toContain(other);
    }
    for (const name of ["ios", "ios-upload", "ios-record"]) {
      for (const other of ["macos", "linux", "publish", "sign"]) expect(needs(name)).not.toContain(other);
    }
  });
});

describe("the keys of the iOS application", () => {
  test("the job that builds says in plain words when its secrets are not in its own environment", () => {
    expect(text("ios")).toContain("The three iOS signing secrets are not set in the environment release-ios. They are not read from the environment release");
    expect(text("ios")).toContain("apps/desktop/src-tauri/ios-archive.sh is not in this commit");
  });

  test("the job that builds uploads nothing and has no key of App Store Connect", () => {
    expect(JSON.stringify(job("ios"))).not.toMatch(/ASC_API|altool|notarytool|iTMSTransporter/);
  });

  test("the job with the key of App Store Connect reads no repository and sends only the file that the build job reported", () => {
    expect(job("ios-upload").steps.some((step) => step.uses?.startsWith("actions/checkout@"))).toBe(false);
    const send = text("ios-upload");
    expect(send.indexOf('echo "$IPA_SHA256  ios/Anima.ipa" | shasum -a 256 --check --strict')).toBeLessThan(send.indexOf("xcrun altool --upload-app"));
    expect(send).toContain("trap 'rm -f \"$keys/AuthKey_$ASC_API_KEY_ID.p8\"' EXIT");
  });

  test("the job takes the file only when the script says that it is signed and of the version of the release", () => {
    const build = text("ios");
    expect(build).toContain('sh apps/desktop/src-tauri/ios-archive.sh "$VERSION" | tee "$RUNNER_TEMP/archive.log"');
    expect(build).toContain('if [ "$(said version)" != "$VERSION" ] || [ "$(said signed)" != yes ]; then');
    expect(build.indexOf('"$(said signed)" != yes')).toBeLessThan(build.indexOf('cp "$ipa" out/Anima.ipa'));
  });
});
