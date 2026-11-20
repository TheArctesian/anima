// deploy/release/applications.txt against the workspace: the crates that the map lets affect
// `anima` alone are the crates that the program is built from, as `cargo metadata` gives them
// (task B8h). A crate that is added to the program, or a new crate that the map does not
// know, fails here until the map says where it goes.
import { describe, expect, test } from "bun:test";
import { readdirSync, readFileSync } from "node:fs";
import { join, relative } from "node:path";

const ROOT = join(import.meta.dir, "..", "..");
const MAP = readFileSync(join(ROOT, "deploy", "release", "applications.txt"), "utf8");

interface Package {
  name: string;
  manifest_path: string;
  dependencies: { name: string; kind: string | null; path?: string; target: string | null }[];
}

/**
 * The folders, from the root, of the workspace crates that `program` is built from; no crate of
 * its tests. It reads only the manifests of the workspace (`--no-deps`), so it needs no crate of
 * the registry on the computer and no network.
 */
function cratesOf(program: string, includes: (dependency: Package["dependencies"][number]) => boolean = () => true): string[] {
  const ran = Bun.spawnSync(["cargo", "metadata", "--format-version", "1", "--no-deps", "--offline"], { cwd: ROOT, stdout: "pipe", stderr: "pipe" });
  if (ran.exitCode !== 0) throw new Error(`cargo metadata failed: ${ran.stderr.toString()}`);
  const packages = (JSON.parse(ran.stdout.toString()) as { packages: Package[] }).packages;
  const folderOf = (one: Package): string => join(one.manifest_path, "..");
  const byFolder = new Map(packages.map((one) => [folderOf(one), one]));
  const start = packages.find((one) => one.name === program);
  if (start === undefined) throw new Error(`the workspace has no package ${program}`);
  const used = new Set<string>();
  const waiting = [folderOf(start)];
  for (let folder = waiting.pop(); folder !== undefined; folder = waiting.pop()) {
    if (used.has(folder)) continue;
    used.add(folder);
    for (const dependency of byFolder.get(folder)?.dependencies ?? []) {
      if (dependency.kind !== "dev" && dependency.path !== undefined && byFolder.has(dependency.path) && includes(dependency)) {
        waiting.push(dependency.path);
      }
    }
  }
  return [...used].map((folder) => `${relative(ROOT, folder).replaceAll("\\", "/")}/`).sort();
}

/** The folders below `crates/` that no `out` line of `application` names. */
function cratesIn(application: string): string[] {
  const out = MAP.split("\n")
    .map((line) => line.split(" "))
    .filter(([name, kind]) => name === application && kind === "out")
    .map(([, , path]) => path ?? "");
  return readdirSync(join(ROOT, "crates"), { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => `crates/${entry.name}/`)
    .filter((folder) => !out.includes(folder))
    .sort();
}

describe("the map of `anima` alone (task B8h)", () => {
  test.skipIf(Bun.which("cargo") === null)("the crates that affect it are the crates that the program is built from", () => {
    expect(cratesIn("cli")).toEqual(cratesOf("anima-cli"));
  });
});

describe("the map of the iOS application (task B8i)", () => {
  test.skipIf(Bun.which("cargo") === null)("the crates that affect it are the crates below `crates/` that the shell is built from", () => {
    // The release map includes the desktop shell's shared crates, while Android-only code cannot affect iOS.
    const crates = cratesOf("anima-app", (dependency) => dependency.target !== 'cfg(target_os = "android")');
    expect(cratesIn("ios")).toEqual(crates.filter((folder) => folder.startsWith("crates/")));
  });
});
