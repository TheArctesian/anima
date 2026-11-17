import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { join } from "node:path";
import { checkDocuments, MOCKUP, parseKnown } from "../src/documents/documents.ts";
import { hasList, REGISTER } from "../src/documents/register.ts";
import { listMeasurableFiles, readRepoFile, REPO_ROOT } from "../src/repository.ts";
import { report } from "../src/report.ts";

// A path can be in the list more than once while a merge has conflicts.
const documents = [...new Set(listMeasurableFiles(REPO_ROOT))]
  .filter((path) => path.endsWith(".md") || MOCKUP.test(path))
  .map((path) => ({ path, text: path.endsWith(".md") ? readRepoFile(REPO_ROOT, path) : "" }));
const known = parseKnown(readRepoFile(REPO_ROOT, "quality/duplicate-identifiers.txt"));

/** The number of commits after `commit` that changed a layout file (Svelte or CSS) of a screen folder. */
function layoutChangedSince(screen: string, commit: string): number | undefined {
  const pathspecs = ["svelte", "css"].map((kind) => `:(glob)${screen}/**/*.${kind}`);
  const counted = spawnSync("git", ["rev-list", "--count", `${commit}..HEAD`, "--", ...pathspecs], { cwd: REPO_ROOT, encoding: "utf8" });
  return counted.status === 0 ? Number(counted.stdout.trim()) : undefined;
}

const exists = (path: string): boolean => existsSync(join(REPO_ROOT, path));
const violations = checkDocuments(documents, exists, known, layoutChangedSince);
// A register with no markers has no generated list, so nothing would compare it with the headers.
if (!hasList(readRepoFile(REPO_ROOT, REGISTER))) violations.push({ path: REGISTER, signal: "register-without-list", detail: "the two marker comments of the generated list are missing" });
process.exit(report("documents", violations));
