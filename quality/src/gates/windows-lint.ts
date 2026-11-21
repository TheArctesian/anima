/**
 * Clippy of the workspace for Windows, on Linux (`bun quality/bin/gates.ts --windows-lint`, the job
 * `windows-lint` of the workflow `gates`). Code behind `cfg(windows)` is compiled only for Windows,
 * so the Linux jobs never see it; on 2026-11-21 three faults of that code reached `dev` unseen (a
 * command not registered, a pedantic lint, a missing field). The GNU target needs only mingw-w64,
 * and Clippy links nothing for the target, so it type-checks and lints the Windows code on Linux.
 * No crate of the workspace asks for `target_env = "msvc"`, so the code is the code of the
 * Windows build. The Windows job (windows-trial.yml) still builds with MSVC and runs the tests.
 */

export const WINDOWS_TARGET = "x86_64-pc-windows-gnu";

/** A crate that the GNU target cannot build, with the reason. The job then does not lint its Windows code. */
export interface LeftOut {
  crate: string;
  reason: string;
}

/** The crates left out of the lint. Empty: each crate of the workspace builds for the GNU target. */
export const LEFT_OUT: readonly LeftOut[] = [];

/**
 * The Clippy command of the lint, with warnings denied as in the gate `clippy`. `--keep-going`: a
 * fault in one crate does not stop the check of each crate that does not depend on it. Pure.
 */
export function windowsLintArgv(leftOut: readonly LeftOut[] = LEFT_OUT): string[] {
  return [
    "cargo",
    "clippy",
    "--workspace",
    ...leftOut.flatMap(({ crate }) => ["--exclude", crate]),
    "--all-targets",
    "--target",
    WINDOWS_TARGET,
    "--locked",
    "--keep-going",
    "--",
    "-D",
    "warnings",
  ];
}
