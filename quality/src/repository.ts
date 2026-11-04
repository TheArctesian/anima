import { existsSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { parseGeneratedList, withoutGenerated } from "./glob.ts";

/** The repository root: the parent of `quality/`. */
export const REPO_ROOT = resolve(import.meta.dir, "..", "..");

function git(root: string, args: string[]): { ok: boolean; stdout: string } {
  const run = Bun.spawnSync(["git", ...args], { cwd: root });
  return { ok: run.exitCode === 0, stdout: run.stdout.toString() };
}

/** Tracked and untracked-but-not-ignored files that exist, minus generated globs. */
export function listMeasurableFiles(root: string): string[] {
  const listed = git(root, ["ls-files", "-z", "--cached", "--others", "--exclude-standard"]);
  if (!listed.ok) throw new Error(`git ls-files failed in ${root}`);
  const paths = listed.stdout.split("\0").filter((path) => path !== "" && existsSync(join(root, path)));
  const generatedPath = join(root, "quality", "generated.txt");
  const generated = existsSync(generatedPath) ? parseGeneratedList(readFileSync(generatedPath, "utf8")) : [];
  return withoutGenerated(paths, generated);
}

export function readRepoFile(root: string, path: string): string {
  return readFileSync(join(root, path), "utf8");
}

/** The file's content at a git ref, or undefined when the file is absent there. */
export function readFileAtRef(root: string, ref: string, path: string): string | undefined {
  if (!git(root, ["rev-parse", "--verify", `${ref}^{commit}`]).ok) {
    throw new Error(`unknown git ref: ${ref}`);
  }
  const shown = git(root, ["show", `${ref}:${path}`]);
  return shown.ok ? shown.stdout : undefined;
}
