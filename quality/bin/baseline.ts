import { BaseRefError, baseRefFrom, growth, parseBaseline } from "../src/baseline.ts";
import { readFileAtRef, readRepoFile, REPO_ROOT } from "../src/repository.ts";
import { report } from "../src/report.ts";

let ref: string;
try {
  ref = baseRefFrom(process.argv, "phase-2");
} catch (error) {
  if (!(error instanceof BaseRefError)) throw error;
  console.error(`baseline: ${error.message}`);
  process.exit(2);
}
const BASELINE = "quality/baseline.json";

const base = parseBaseline(readFileAtRef(REPO_ROOT, ref, BASELINE) ?? "{}");
const current = parseBaseline(readRepoFile(REPO_ROOT, BASELINE));
process.exit(report("baseline", growth(base, current)));
