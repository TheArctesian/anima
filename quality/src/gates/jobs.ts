import { isDocument } from "./tier.ts";

/**
 * The jobs of the workflow `gates` that a pushed branch needs, from the paths that it changed
 * against `dev` (`bun quality/bin/gates.ts --jobs`), and of a push to `dev` (`--dev-push`). A run
 * for a train or for a call runs each job. The train tests each branch again with the full list
 * before `dev` moves, so a job that this leaves out is still run before a merge.
 */
export interface Jobs {
  /** The jobs rust-lint and rust-test. */
  rust: boolean;
  /** Only these crates are linted and tested: each Rust path is a test of one of them (test-only.ts). */
  rustCrates?: string[];
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

/** The crate whose tests alone read a path, or undefined (test-only.ts `testOwner`). */
export type TestOwner = (path: string) => string | undefined;

/**
 * The jobs for the changed `paths` of a pushed branch. When each path that Rust reads is a test of
 * a crate (`owner`), the Rust jobs take only those crates. Pure.
 */
export function jobsFor(paths: string[], owner: TestOwner = () => undefined): Jobs {
  if (paths.some((path) => RUNNERS.some((runner) => path.startsWith(runner)))) return EACH_JOB;
  const code = paths.filter((path) => !isDocument(path));
  if (code.length === 0) return { rust: false, go: false, documentsOnly: true };
  const rust = code.filter(readByRust);
  const owners = rust.map(owner);
  const jobs = { rust: rust.length > 0, go: code.some((path) => path.startsWith("link/")), documentsOnly: false };
  if (rust.length === 0 || owners.some((crate) => crate === undefined)) return jobs;
  return { ...jobs, rustCrates: [...new Set(owners as string[])].sort() };
}

/**
 * A push to `dev`: the tree that `dev` now holds, the tree before the push with the paths that
 * changed since, and each tree whose gates passed as it is (a run on `dev` or on a train), with
 * the number of that run.
 */
export interface DevPush {
  tree: string;
  before?: { tree: string; changed: string[] };
  proved: ReadonlyMap<string, number>;
}

/** The jobs of a push to `dev`; `provedBy` is the run that already passed for the same tree. */
export interface DevJobs {
  jobs: Jobs;
  provedBy?: number;
  reason: string;
}

/**
 * The jobs of a push to `dev`, so that `dev` never holds a tree that no run proved. A tree that
 * passed as it is needs no job. A push of documents only onto a proved tree needs the quality
 * gates, the only gates that read a document. Each other tree runs each job. Pure.
 */
export function devPushJobs({ tree, before, proved }: DevPush): DevJobs {
  const same = proved.get(tree);
  if (same !== undefined) return { jobs: { rust: false, go: false, documentsOnly: true }, provedBy: same, reason: `run ${String(same)} passed for this tree` };
  const parent = before === undefined ? undefined : proved.get(before.tree);
  const documents = before !== undefined && before.changed.length > 0 && before.changed.every(isDocument);
  if (parent !== undefined && documents) {
    return { jobs: { rust: false, go: false, documentsOnly: true }, reason: `only documents changed on the tree that run ${String(parent)} proved` };
  }
  return { jobs: EACH_JOB, reason: parent === undefined ? "no run proved the tree before this push" : "the push changed more than documents" };
}

/** The lines for `$GITHUB_OUTPUT`; `proved` names the run that makes each job unnecessary. */
export function jobLines(jobs: Jobs, proved?: number): string[] {
  const lines = [`rust=${String(jobs.rust)}`, `go=${String(jobs.go)}`, `documents_only=${String(jobs.documentsOnly)}`, `rust_crates=${jobs.rustCrates?.join(",") ?? ""}`];
  return proved === undefined ? lines : [...lines, `proved=${String(proved)}`];
}
