import { existsSync, readdirSync, readFileSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import type { Layout, Member } from "./tier.ts";

/** The Rust packages of the workspace, from `cargo metadata`. Throws when cargo cannot say. */
function crates(root: string): Member[] {
  const run = Bun.spawnSync(["cargo", "metadata", "--no-deps", "--format-version", "1", "--locked"], { cwd: root });
  if (run.exitCode !== 0) throw new Error("cargo metadata failed: the fast tier cannot find the Rust packages");
  const metadata = JSON.parse(run.stdout.toString()) as {
    packages: { name: string; manifest_path: string; dependencies: { name: string; path?: string }[] }[];
  };
  return metadata.packages.map((each) => ({
    name: each.name,
    folder: relative(root, dirname(each.manifest_path)),
    uses: each.dependencies.filter((dependency) => dependency.path !== undefined).map((dependency) => dependency.name),
  }));
}

/** The folders that one workspace entry of `package.json` names: a folder, or `folder/*`. */
function workspaceFolders(root: string, entry: string): string[] {
  if (!entry.endsWith("/*")) return [entry];
  const above = entry.slice(0, -2);
  return readdirSync(join(root, above), { withFileTypes: true })
    .filter((each) => each.isDirectory())
    .map((each) => `${above}/${each.name}`);
}

/** The bun packages of the workspace, from the `workspaces` of the root `package.json`. */
function packages(root: string): Member[] {
  const manifest = JSON.parse(readFileSync(join(root, "package.json"), "utf8")) as { workspaces?: string[] };
  return (manifest.workspaces ?? [])
    .flatMap((entry) => workspaceFolders(root, entry))
    .filter((folder) => existsSync(join(root, folder, "package.json")))
    .map((folder) => {
      const each = JSON.parse(readFileSync(join(root, folder, "package.json"), "utf8")) as {
        name?: string;
        dependencies?: Record<string, string>;
        devDependencies?: Record<string, string>;
      };
      const versions = { ...each.dependencies, ...each.devDependencies };
      const uses = Object.keys(versions).filter((name) => versions[name]?.startsWith("workspace:"));
      return { folder, name: each.name ?? folder, uses };
    });
}

/** The packages of the repository at `root`. */
export function readLayout(root: string): Layout {
  return { crates: crates(root), packages: packages(root) };
}

/**
 * The paths that are different from the merge base with `base`: committed, staged, not staged and
 * new files that git does not ignore. `undefined` when `base` is not a commit here.
 */
export function changedPaths(root: string, base: string): string[] | undefined {
  const git = (...args: string[]) => {
    const run = Bun.spawnSync(["git", ...args], { cwd: root });
    return run.exitCode === 0 ? run.stdout.toString() : undefined;
  };
  const mergeBase = git("merge-base", base, "HEAD")?.trim();
  if (mergeBase === undefined || mergeBase === "") return undefined;
  const tracked = git("diff", "--name-only", "-z", mergeBase);
  const added = git("ls-files", "-z", "--others", "--exclude-standard");
  if (tracked === undefined || added === undefined) return undefined;
  return [...new Set(`${tracked}${added}`.split("\0").filter((path) => path !== ""))].sort();
}

/**
 * The Rust packages whose folders hold `paths`: for each path, the nearest folder above it with a
 * `Cargo.toml` that has a `[package]` and its name. Reads only files, so it needs no Rust toolchain
 * (the job `base` of the workflow `gates` has none).
 */
export function cratesHolding(root: string, paths: readonly string[]): Member[] {
  const found = new Map<string, Member>();
  for (const path of paths) {
    for (let folder = dirname(path); folder !== "." && folder !== "/" && !found.has(folder); folder = dirname(folder)) {
      const manifest = join(root, folder, "Cargo.toml");
      if (!existsSync(manifest)) continue;
      const name = /^\[package\][^[]*?^name\s*=\s*"([^"]+)"/ms.exec(readFileSync(manifest, "utf8"))?.[1];
      if (name !== undefined) found.set(folder, { folder, name });
      break;
    }
  }
  return [...found.values()];
}
