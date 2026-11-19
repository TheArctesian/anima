import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { join } from "node:path";
import { checkDocuments, MOCKUP, parseKnown } from "../src/documents/documents.ts";
import { QUEUE } from "../src/documents/queue.ts";
import { REGISTER } from "../src/documents/register.ts";
import { isShared, placeholders } from "../src/merge/numbers.ts";
import { listMeasurableFiles, readRepoFile, REPO_ROOT } from "../src/repository.ts";
import { report } from "../src/report.ts";

// A path can be in the list more than once while a merge has conflicts.
const documents = [...new Set(listMeasurableFiles(REPO_ROOT))]
  .filter((path) => path.endsWith(".md") || MOCKUP.test(path))
  .map((path) => ({ path, text: path.endsWith(".md") ? readRepoFile(REPO_ROOT, path) : "" }));
const known = parseKnown(readRepoFile(REPO_ROOT, "quality/duplicate-identifiers.txt"));

/**
 * The number of layout files (Svelte or CSS) below a screen folder that changed after `commit`.
 * The folder is a literal path: `[id]` in a route is a name, not a pattern.
 */
function layoutChangedSince(screen: string, commit: string): number | undefined {
  const changed = spawnSync("git", ["diff", "--name-only", `${commit}..HEAD`, "--", `:(literal)${screen}`], { cwd: REPO_ROOT, encoding: "utf8" });
  if (changed.status !== 0) return undefined;
  return changed.stdout.split("\n").filter((path) => path.endsWith(".svelte") || path.endsWith(".css")).length;
}

const exists = (path: string): boolean => existsSync(join(REPO_ROOT, path));
const violations = checkDocuments(documents, exists, known, layoutChangedSince);
// The generated parts are checked inside `checkDocuments`; a file that does not exist is not in its list.
for (const [path, signal] of [[REGISTER, "register-without-list"], [QUEUE, "queue-without-list"]] as const) {
  if (!exists(path)) violations.push({ path, signal, detail: "the file is missing" });
}
// PLN-021: a branch can hold a placeholder of a number; `dev` and `main` cannot. The merge script
// (quality/bin/merge.ts) gives each placeholder its number.
const head = spawnSync("git", ["rev-parse", "--abbrev-ref", "HEAD"], { cwd: REPO_ROOT, encoding: "utf8" }).stdout.trim();
if (isShared(head) || isShared(process.env["GITHUB_REF_NAME"] ?? "")) {
  for (const placeholder of placeholders(documents)) {
    violations.push({ path: "docs", signal: "number-not-assigned", detail: `${placeholder}: run the merge script` });
  }
}
process.exit(report("documents", violations));
