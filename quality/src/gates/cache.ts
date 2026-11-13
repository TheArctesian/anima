import { join } from "node:path";

/** The environment the gates run with, and the line that says whether Rust builds share a cache. */
export interface CacheChoice {
  env: Record<string, string | undefined>;
  note: string;
}

/**
 * Shares compiled Rust between worktrees through sccache when it is on `PATH`, so a fresh worktree
 * does not rebuild every crate from scratch. Each worktree keeps its own `target/` (R-019): only
 * compiler outputs are shared, keyed by their inputs. sccache cannot cache incremental builds, so
 * incremental compilation is off. A `RUSTC_WRAPPER` already set, even empty, is the caller's choice
 * and is kept.
 */
export function compileCache(env: Record<string, string | undefined>, exists: (path: string) => boolean): CacheChoice {
  if (env["RUSTC_WRAPPER"] !== undefined) {
    return { env, note: `rust cache: RUSTC_WRAPPER is set (${env["RUSTC_WRAPPER"] || "empty"}), kept as it is` };
  }
  const dirs = (env["PATH"] ?? "").split(":").filter((dir) => dir !== "");
  const folder = dirs.find((dir) => exists(join(dir, "sccache")));
  if (folder === undefined) {
    return { env, note: "rust cache: off (install sccache to share Rust builds between worktrees: brew install sccache)" };
  }
  const wrapper = join(folder, "sccache");
  return { env: { ...env, RUSTC_WRAPPER: wrapper, CARGO_INCREMENTAL: "0" }, note: `rust cache: sccache (${wrapper})` };
}
