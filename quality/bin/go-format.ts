import { goFiles, unformatted } from "../src/gates/go-format.ts";
import { listMeasurableFiles, REPO_ROOT } from "../src/repository.ts";
import { report } from "../src/report.ts";

const files = goFiles(listMeasurableFiles(REPO_ROOT));
if (files.length === 0) process.exit(report("go-fmt", []));
const run = Bun.spawnSync(["gofmt", "-l", ...files], { cwd: REPO_ROOT, stderr: "inherit" });
if (run.exitCode !== 0) process.exit(run.exitCode ?? 1);
process.exit(report("go-fmt", unformatted(run.stdout.toString())));
