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
  id: string;
  name: string;
  manifest_path: string;
}
interface Node {
  id: string;
  deps: { pkg: string; dep_kinds: { kind: string | null }[] }[];
}
interface Metadata {
  packages: Package[];
  workspace_members: string[];
  resolve: { nodes: Node[] };
}

/** The folders, from the root, of the workspace crates that `program` is built from; no crate of its tests. */
function cratesOf(program: string): string[] {
  const ran = Bun.spawnSync(["cargo", "metadata", "--format-version", "1", "--locked", "--offline"], { cwd: ROOT, stdout: "pipe", stderr: "pipe" });
  if (ran.exitCode !== 0) throw new Error(`cargo metadata failed: ${ran.stderr.toString()}`);
  const metadata = JSON.parse(ran.stdout.toString()) as Metadata;
  const members = new Set(metadata.workspace_members);
  const packages = new Map(metadata.packages.map((one) => [one.id, one]));
  const nodes = new Map(metadata.resolve.nodes.map((one) => [one.id, one]));
  const start = metadata.packages.find((one) => one.name === program && members.has(one.id));
  if (start === undefined) throw new Error(`the workspace has no package ${program}`);
  const used = new Set<string>();
  const waiting = [start.id];
  for (let id = waiting.pop(); id !== undefined; id = waiting.pop()) {
    if (used.has(id)) continue;
    used.add(id);
    for (const dependency of nodes.get(id)?.deps ?? []) {
      if (dependency.dep_kinds.some((kind) => kind.kind !== "dev")) waiting.push(dependency.pkg);
    }
  }
  return [...used]
    .filter((id) => members.has(id))
    .map((id) => `${relative(ROOT, join(packages.get(id)?.manifest_path ?? "", ".."))}/`)
    .sort();
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
