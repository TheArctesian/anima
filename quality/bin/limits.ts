import { parseBaseline } from "../src/baseline.ts";
import { isMeasured, checkLimits } from "../src/limits.ts";
import { listMeasurableFiles, readRepoFile, REPO_ROOT } from "../src/repository.ts";
import { report } from "../src/report.ts";

const files = listMeasurableFiles(REPO_ROOT)
  .filter(isMeasured)
  .map((path) => ({ path, text: readRepoFile(REPO_ROOT, path) }));
const baseline = parseBaseline(readRepoFile(REPO_ROOT, "quality/baseline.json"));
process.exit(report("limits", checkLimits(files, baseline)));
