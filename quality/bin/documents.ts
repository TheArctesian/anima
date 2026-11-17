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
process.exit(report("documents", checkDocuments(documents, (path) => existsSync(join(REPO_ROOT, path)), known)));
