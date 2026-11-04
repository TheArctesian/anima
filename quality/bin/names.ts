import { checkNames } from "../src/names.ts";
import { listMeasurableFiles, readRepoFile, REPO_ROOT } from "../src/repository.ts";
import { report } from "../src/report.ts";

const files = listMeasurableFiles(REPO_ROOT).map((path) => ({
  path,
  text: /\.(rs|ts|js|svelte)$/.test(path) ? readRepoFile(REPO_ROOT, path) : "",
}));
process.exit(report("names", checkNames(files)));
