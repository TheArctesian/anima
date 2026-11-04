import { parseHubModules } from "../src/hub-modules.ts";
import { checkImports, HUB_MODULES_FILE, HUB_SOURCE } from "../src/imports.ts";
import { listMeasurableFiles, readRepoFile, REPO_ROOT } from "../src/repository.ts";
import { report } from "../src/report.ts";

const files = listMeasurableFiles(REPO_ROOT)
  .filter((path) => path.startsWith(HUB_SOURCE) && path.endsWith(".rs"))
  .map((path) => ({ path, text: readRepoFile(REPO_ROOT, path) }));
process.exit(report("imports", checkImports(files, parseHubModules(readRepoFile(REPO_ROOT, HUB_MODULES_FILE)))));
