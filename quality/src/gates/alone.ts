/**
 * Checks each crate of the workspace alone: one `cargo check -p <crate>` for each. A build of the
 * workspace turns on the union of the features that its crates ask for, so a crate can compile
 * there and fail when it is built by itself, as the release builds `anima` and the application do
 * (anima-cli on dev at 6364bb038: a feature of rustix that only a different crate asked for).
 * The job `rust-lint` of the workflow `gates` runs it (`bun quality/bin/gates.ts --alone`).
 */

/** Runs `argv` in the repository and returns its exit code. */
export type Run = (argv: string[]) => number;

/** Checks each of `crates` alone, reports each, and returns the exit code: 1 when one does not build alone. */
export function checkAlone(crates: string[], run: Run, report: (line: string) => void): number {
  const failed: string[] = [];
  for (const [index, crate] of crates.entries()) {
    const argv = ["cargo", "check", "-p", crate, "--locked"];
    report(`[${index + 1}/${crates.length}] ${argv.join(" ")}`);
    if (run(argv) !== 0) failed.push(crate);
  }
  if (failed.length === 0) {
    report(`alone: each of ${crates.length} crates builds alone`);
    return 0;
  }
  report(`alone: ${failed.length} crate(s) do not build alone: ${failed.join(", ")}. Each crate must name in its own Cargo.toml each feature that it uses`);
  return 1;
}
