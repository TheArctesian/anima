import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { join } from "node:path";
import { checkDocuments, MOCKUP, parseKnown } from "../src/documents/documents.ts";
import { listMeasurableFiles, readRepoFile, REPO_ROOT } from "../src/repository.ts";
import { report } from "../src/report.ts";

// A path can be in the list more than once while a merge has conflicts.
const documents = [...new Set(listMeasurableFiles(REPO_ROOT))]
  .filter((path) => path.endsWith(".md") || MOCKUP.test(path))
  .map((path) => ({ path, text: path.endsWith(".md") ? readRepoFile(REPO_ROOT, path) : "" }));
const known = parseKnown(readRepoFile(REPO_ROOT, "quality/duplicate-identifiers.txt"));

/** The day of the last commit that changed a layout file (Svelte or CSS) of a screen folder. */
function layoutChanged(screen: string): string | undefined {
  const pathspecs = ["svelte", "css"].map((kind) => `:(glob)${screen}/**/*.${kind}`);
  const shown = spawnSync("git", ["log", "-1", "--format=%cs", "--", ...pathspecs], { cwd: REPO_ROOT, encoding: "utf8" });
  const day = shown.status === 0 ? shown.stdout.trim() : "";
  return day === "" ? undefined : day;
}

const exists = (path: string): boolean => existsSync(join(REPO_ROOT, path));
process.exit(report("documents", checkDocuments(documents, exists, known, layoutChanged)));
