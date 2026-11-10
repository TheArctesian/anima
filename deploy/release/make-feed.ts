// Makes the files of one release from the downloaded build artifacts (UPD-009, UPD-010).
//
// Usage: bun deploy/release/make-feed.ts --version 0.1.7 --tag v0.1.7 --artifacts DIR --out DIR
//
// DIR/macos and DIR/linux hold the artifact of each build that passed in this run. A folder
// that is missing means that platform did not build, and the feed leaves it out.
import { createHash } from "node:crypto";
import { copyFileSync, existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { basename, join } from "node:path";
import { parseArgs } from "node:util";
import { assertSignedVersion } from "./signed-version.ts";
// The feed is the builder's too (BLD-063), so it lives with the builder loop in the client.
import {
  buildFeed,
  checksums,
  stableName,
  type BuiltPlatforms,
  type NamedDigest,
  type UpdateFile,
} from "../../apps/client/src/lib/builds/feed.ts";

const RELEASES = "https://github.com/TheArctesian/anima/releases/download";

function filesUnder(folder: string): string[] {
  if (!existsSync(folder)) return [];
  return readdirSync(folder, { recursive: true, withFileTypes: true })
    .filter((entry) => entry.isFile())
    .map((entry) => join(entry.parentPath, entry.name));
}

/** The updater bundle with the given suffix and the text of its `.sig` file, if both exist. */
function updateFile(paths: string[], suffix: string, version: string): UpdateFile | undefined {
  const bundle = paths.find((path) => path.endsWith(suffix));
  if (bundle === undefined) return undefined;
  const sig = `${bundle}.sig`;
  if (!paths.includes(sig)) {
    console.error(`warning: ${basename(bundle)} has no .sig file, so its platform is left out of the feed`);
    return undefined;
  }
  const signature = readFileSync(sig, "utf8");
  assertSignedVersion(basename(bundle), signature, version);
  return { file: basename(bundle), signature };
}

const { values } = parseArgs({
  options: {
    version: { type: "string" },
    tag: { type: "string" },
    artifacts: { type: "string" },
    out: { type: "string" },
  },
});
const { version, tag, artifacts, out } = values;
if (!version || !tag || !artifacts || !out) {
  console.error("usage: make-feed.ts --version V --tag T --artifacts DIR --out DIR");
  process.exit(2);
}
if (!/^\d+\.\d+\.\d+$/.test(version)) {
  console.error(`version must be MAJOR.MINOR.PATCH with no "v": ${version}`);
  process.exit(2);
}

const macos = filesUnder(join(artifacts, "macos"));
const linux = filesUnder(join(artifacts, "linux"));
const built: BuiltPlatforms = {
  macos: updateFile(macos, ".app.tar.gz", version),
  linux: updateFile(linux, ".AppImage", version),
};

mkdirSync(out, { recursive: true });
const released: NamedDigest[] = [];
const publish = (name: string, source: string): void => {
  copyFileSync(source, join(out, name));
  released.push({ name, sha256: createHash("sha256").update(readFileSync(source)).digest("hex") });
};
for (const path of [...macos, ...linux]) {
  publish(basename(path), path);
  const stable = stableName(basename(path));
  if (stable !== undefined) publish(stable, path);
}

const feed = buildFeed({ version, baseUrl: `${RELEASES}/${tag}`, date: new Date(), built });
writeFileSync(join(out, "latest.json"), `${JSON.stringify(feed, null, 2)}\n`);
writeFileSync(join(out, "SHA256SUMS"), checksums(released));
console.log(`feed for ${version}: ${Object.keys(feed.platforms).join(", ")}`);
