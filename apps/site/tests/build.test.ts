import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { cp, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { buildSite } from "../src/build.ts";
import { ReleaseFeedError } from "../src/release.ts";
import { readSettings } from "../src/settings.ts";
import { ReleaseSourceError } from "../src/source.ts";

const root = join(import.meta.dir, "../../..");
let work = "";
let source = "";
let output = "";

beforeEach(async () => {
  work = await mkdtemp(join(tmpdir(), "anima-site-build-"));
  source = join(work, "releases");
  output = join(work, "out");
  await cp(join(root, "apps/site/fixtures/releases"), source, { recursive: true });
});

afterEach(async () => {
  await rm(work, { recursive: true, force: true });
});

function build() {
  return buildSite(readSettings({ ANIMA_SITE_RELEASE_SOURCE: source }), root, output);
}

describe("SITE-003 and OD-19: the build step fails safe", () => {
  test("a good source builds, and the output holds the page", async () => {
    await build();
    expect(await readFile(join(output, "index.html"), "utf8")).toContain("0.1.12");
  });

  test("a missing feed file stops the build with a message that names the file, and writes nothing", async () => {
    await rm(join(source, "linux-x86_64/latest.json"));
    await expect(build()).rejects.toThrow(ReleaseSourceError);
    await expect(build()).rejects.toThrow(/linux-x86_64\/latest\.json/);
    expect(await Bun.file(join(output, "index.html")).exists()).toBe(false);
  });

  test("a malformed feed stops the build and leaves the last good output in place", async () => {
    await build();
    await writeFile(join(source, "darwin-aarch64/latest.json"), '{"version":"1.0"}');
    await expect(build()).rejects.toThrow(ReleaseFeedError);
    expect(await readFile(join(output, "index.html"), "utf8")).toContain("0.1.12");
  });

  test("a feed that names a new version without its SHA256SUMS stops the build", async () => {
    const feed = (await readFile(join(source, "linux-x86_64/latest.json"), "utf8")).replaceAll("0.1.12", "0.1.13");
    await writeFile(join(source, "linux-x86_64/latest.json"), feed);
    await expect(build()).rejects.toThrow(/0\.1\.13\/SHA256SUMS/);
  });

  test("a source URL that cannot be reached stops the build", async () => {
    const settings = readSettings({ ANIMA_SITE_RELEASE_SOURCE: "http://127.0.0.1:9" });
    await expect(buildSite(settings, root, output)).rejects.toThrow(ReleaseSourceError);
  });
});
