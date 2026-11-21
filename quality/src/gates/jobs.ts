import { isDocument } from "./tier.ts";

/**
 * The jobs of the workflow `gates` that a pushed branch needs, from the paths that it changed
 * against `dev` (`bun quality/bin/gates.ts --jobs`). A run for `dev`, for a train or for a call
 * runs each job: only the push of a branch asks. The train tests each branch again with the full
 * list before `dev` moves, so a job that this leaves out is still run before a merge.
 */
export interface Jobs {
  /** The jobs rust-lint and rust-test. */
  rust: boolean;
  /** The job go (the six Go gates of `link/`). */
  go: boolean;
  /** Each changed path is a document: the job bun runs only the quality gates. */
  documentsOnly: boolean;
}

export const EACH_JOB: Jobs = { rust: true, go: true, documentsOnly: false };

/** Paths that change how each job runs: the workflows and the programs that run the gates. */
const RUNNERS = [
  ".github/",
  "quality/bin/gates.ts",
  "quality/src/gates/",
  "quality/bin/with-postgres.ts",
  "quality/bin/go-format.ts",
  "quality/bin/rustc-cache.sh",
];

/**
 * Paths that no Rust build or test reads, with the exceptions that one does read. Found on
 * 2026-11-21 from each `include_str!`, `include_bytes!`, path from `CARGO_MANIFEST_DIR` and build
 * script of the workspace: the Rust tests read `link/hublink/`, `deploy/postgres/roles.sql` and
 * `contracts/`; the theme crate is generated from `design/tokens`, so `design/` stays Rust. The
 * desktop shell embeds `apps/client/build`, but no Rust job builds the client.
 */
const NOT_RUST = ["apps/client/", "apps/desktop/", "link/", "deploy/", "quality/"];
const RUST_INSIDE = ["apps/desktop/src-tauri/", "link/hublink/", "deploy/postgres/"];

function readByRust(path: string): boolean {
  if (RUST_INSIDE.some((prefix) => path.startsWith(prefix))) return true;
  return !NOT_RUST.some((prefix) => path.startsWith(prefix));
}

/** The jobs for the changed `paths` of a pushed branch. Pure. */
export function jobsFor(paths: string[]): Jobs {
  if (paths.some((path) => RUNNERS.some((runner) => path.startsWith(runner)))) return EACH_JOB;
  const code = paths.filter((path) => !isDocument(path));
  if (code.length === 0) return { rust: false, go: false, documentsOnly: true };
  return { rust: code.some(readByRust), go: code.some((path) => path.startsWith("link/")), documentsOnly: false };
}

/** The lines for `$GITHUB_OUTPUT`. */
export function jobLines(jobs: Jobs): string[] {
  return [`rust=${String(jobs.rust)}`, `go=${String(jobs.go)}`, `documents_only=${String(jobs.documentsOnly)}`];
}
