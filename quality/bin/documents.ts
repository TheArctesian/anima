import { existsSync } from "node:fs";
import { join } from "node:path";
import { checkDocuments, parseKnown } from "../src/documents/documents.ts";
import { listMeasurableFiles, readRepoFile, REPO_ROOT } from "../src/repository.ts";
import { report } from "../src/report.ts";

const documents = listMeasurableFiles(REPO_ROOT)
  .filter((path) => path.endsWith(".md"))
  .map((path) => ({ path, text: readRepoFile(REPO_ROOT, path) }));
const known = parseKnown(readRepoFile(REPO_ROOT, "quality/duplicate-identifiers.txt"));
process.exit(report("documents", checkDocuments(documents, (path) => existsSync(join(REPO_ROOT, path)), known)));
