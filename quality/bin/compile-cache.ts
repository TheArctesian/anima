#!/usr/bin/env bun
// RUSTC_WRAPPER of the gates when sccache is installed (WP-55, R-112, R-101): runs `sccache rustc ...` without
// CARGO_TARGET_DIR in its environment (see `rustcEnv`). TypeScript, not sh: dash drops variables whose names
// hold a hyphen, such as CARGO_BIN_EXE_anima-hub, which the hub's integration tests read at compile time.
import { SCCACHE_VARIABLE, rustcEnv } from "../src/gates/compile-cache.ts";

const sccache = process.env[SCCACHE_VARIABLE];
if (sccache === undefined || sccache === "") {
  console.error(`compile-cache: ${SCCACHE_VARIABLE} is not set; bun run gates sets it with RUSTC_WRAPPER`);
  process.exit(1);
}
const result = Bun.spawnSync([sccache, ...process.argv.slice(2)], {
  env: rustcEnv(process.env),
  stdin: "inherit",
  stdout: "inherit",
  stderr: "inherit",
});
process.exit(result.exitCode ?? 1);
