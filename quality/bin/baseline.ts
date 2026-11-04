import { growth, parseBaseline } from "../src/baseline.ts";
import { readFileAtRef, readRepoFile, REPO_ROOT } from "../src/repository.ts";
import { report } from "../src/report.ts";

const flag = process.argv.indexOf("--base");
const value = flag === -1 ? "phase-1" : process.argv[flag + 1];
if (value === undefined || value.startsWith("--")) {
  console.error("baseline: --base needs a git ref");
  process.exit(2);
}
const ref = value;
const BASELINE = "quality/baseline.json";

const base = parseBaseline(readFileAtRef(REPO_ROOT, ref, BASELINE) ?? "{}");
const current = parseBaseline(readRepoFile(REPO_ROOT, BASELINE));
process.exit(report("baseline", growth(base, current)));
