import { formatGateEvent, gateList, runGates } from "../src/gates/gates.ts";
import { REPO_ROOT } from "../src/repository.ts";

const baseRef = process.env["BASE_REF"] || "phase-1";

const outcome = runGates(
  gateList(baseRef),
  (argv) => {
    const result = Bun.spawnSync(argv, { cwd: REPO_ROOT, stdin: "inherit", stdout: "inherit", stderr: "inherit" });
    return result.exitCode ?? 1;
  },
  (event) => {
    console.log(formatGateEvent(event));
  },
);
process.exit(outcome.exitCode);
