import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { join } from "node:path";
import { checkDocuments, MOCKUP, parseKnown } from "../src/documents/documents.ts";
import { hasQueue, QUEUE } from "../src/documents/queue.ts";
import { hasList, REGISTER } from "../src/documents/register.ts";
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
// A register with no markers has no generated list, so nothing would compare it with the headers.
if (!hasList(readRepoFile(REPO_ROOT, REGISTER))) violations.push({ path: REGISTER, signal: "register-without-list", detail: "the two marker comments of the generated list are missing" });
if (!existsSync(join(REPO_ROOT, QUEUE)) || !hasQueue(readRepoFile(REPO_ROOT, QUEUE))) violations.push({ path: QUEUE, signal: "queue-without-list", detail: "the file or its two marker comments are missing" });
process.exit(report("documents", violations));
