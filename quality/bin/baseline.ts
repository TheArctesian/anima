import { growth, parseBaseline } from "../src/baseline.ts";
import { readFileAtRef, readRepoFile, REPO_ROOT } from "../src/repository.ts";
import { report } from "../src/report.ts";

const flag = process.argv.indexOf("--base");
const ref = (flag === -1 ? undefined : process.argv[flag + 1]) ?? "phase-1";
const BASELINE = "quality/baseline.json";

const base = parseBaseline(readFileAtRef(REPO_ROOT, ref, BASELINE) ?? "{}");
const current = parseBaseline(readRepoFile(REPO_ROOT, BASELINE));
process.exit(report("baseline", growth(base, current)));
