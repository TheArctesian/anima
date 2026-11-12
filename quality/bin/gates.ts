import { existsSync } from "node:fs";
import { join } from "node:path";
import { compileCache } from "../src/gates/compile-cache.ts";
import { formatGateEvent, gateList, missingRequirement, runGates } from "../src/gates/gates.ts";
import { REPO_ROOT } from "../src/repository.ts";

const baseRef = process.env["BASE_REF"] || "phase-2";
const cache = compileCache(process.env, existsSync, join(REPO_ROOT, "quality", "bin", "compile-cache.ts"));
console.log(cache.line);

const outcome = runGates(
  gateList(baseRef),
  (gate) => {
    const missing = missingRequirement(gate, cache.env["PATH"] ?? "", existsSync);
    if (missing !== undefined) throw new Error(missing);
    const result = Bun.spawnSync(gate.argv, {
      cwd: REPO_ROOT,
      env: cache.env,
      stdin: "inherit",
      stdout: "inherit",
      stderr: "inherit",
    });
    return result.exitCode ?? 1;
  },
  (event) => {
    console.log(formatGateEvent(event));
  },
);
process.exit(outcome.exitCode);
