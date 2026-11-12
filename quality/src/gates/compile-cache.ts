// sccache for the cargo gates (WP-55, R-112, R-101): a shared, content-addressed cache of compiler
// output. Each checkout keeps its own CARGO_TARGET_DIR; the cache only hands rustc's output for the
// same inputs back to whichever checkout asks.
import { join } from "node:path";

/** The variable that tells `bin/compile-cache.ts` which sccache to run. */
export const SCCACHE_VARIABLE = "ANIMA_SCCACHE";

/** The gate run's environment and the one line that says whether the compile cache is on. */
export interface CompileCache {
  env: Record<string, string | undefined>;
  line: string;
}

/**
 * Sets `RUSTC_WRAPPER` to `wrapper` (`bin/compile-cache.ts`) and names the first `sccache` on `PATH` for it, unless
 * `RUSTC_WRAPPER` is already set (an empty value is cargo's way to say "no wrapper", so it is kept too).
 * Without sccache the gates compile with plain rustc: the cache only saves time, so it never fails a gate.
 */
export function compileCache(
  env: Record<string, string | undefined>,
  exists: (path: string) => boolean,
  wrapper: string,
): CompileCache {
  const current = env["RUSTC_WRAPPER"];
  if (current !== undefined) {
    return { env, line: `compile cache: RUSTC_WRAPPER is already set (${JSON.stringify(current)}); left as is` };
  }
  const dirs = (env["PATH"] ?? "").split(":").filter((dir) => dir !== "");
  const dir = dirs.find((candidate) => exists(join(candidate, "sccache")));
  if (dir === undefined) {
    return { env, line: "compile cache: off (sccache is not on PATH); compiling with plain rustc" };
  }
  const sccache = join(dir, "sccache");
  return {
    env: { ...env, RUSTC_WRAPPER: wrapper, [SCCACHE_VARIABLE]: sccache },
    line: `compile cache: on (${sccache})`,
  };
}

/**
 * The environment sccache gets for one rustc call: the caller's, without the target folder.
 * sccache puts every `CARGO_*` variable in its cache key, and `CARGO_TARGET_DIR` names the checkout (R-101),
 * so with it no checkout could reuse a dependency another one compiled. rustc never reads it; cargo keeps it.
 */
export function rustcEnv(env: Record<string, string | undefined>): Record<string, string | undefined> {
  const kept = Object.entries(env).filter(([name]) => name !== "CARGO_TARGET_DIR" && name !== "CARGO_BUILD_TARGET_DIR");
  return Object.fromEntries(kept);
}
