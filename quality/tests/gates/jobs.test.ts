import { describe, expect, test } from "bun:test";
import { devPushJobs, EACH_JOB, jobLines, jobsFor } from "../../src/gates/jobs.ts";

describe("the jobs that a pushed branch needs (gates.ts --jobs)", () => {
  test("a branch of documents only runs the quality gates and no Rust or Go job", () => {
    expect(jobsFor(["docs/plans/x-plan.md", "AGENTS.md"])).toEqual({ rust: false, go: false, documentsOnly: true });
  });

  test("a branch of the client, a bun package or deploy runs no Rust and no Go job", () => {
    expect(jobsFor(["apps/client/src/lib/x.svelte", "quality/src/documents/x.ts", "deploy/release/applications.txt"])).toEqual({ rust: false, go: false, documentsOnly: false });
  });

  test("the files outside the crates that a Rust test reads still run the Rust jobs", () => {
    for (const path of ["link/hublink/fixture.json", "deploy/postgres/roles.sql", "contracts/schema.graphql", "design/tokens/colour.json", "apps/desktop/src-tauri/src/main.rs"]) {
      expect(jobsFor([path]).rust, path).toBe(true);
    }
  });

  test("the link runs the Go job, and a path that no rule knows runs the Rust jobs", () => {
    expect(jobsFor(["link/tunnel/tunnel.go"])).toEqual({ rust: false, go: true, documentsOnly: false });
    expect(jobsFor(["scripts/new.sh"]).rust).toBe(true);
  });

  test("a change of a workflow or of the programs that run the gates runs each job", () => {
    expect(jobsFor([".github/workflows/gates.yml"])).toEqual(EACH_JOB);
    expect(jobsFor(["quality/src/gates/tier.ts"])).toEqual(EACH_JOB);
    expect(jobsFor(["quality/bin/with-postgres.ts", "docs/x.md"])).toEqual(EACH_JOB);
  });

  test("the lines for the output of a step", () => {
    expect(jobLines({ rust: false, go: true, documentsOnly: false })).toEqual(["rust=false", "go=true", "documents_only=false", "rust_crates="]);
    expect(jobLines({ rust: true, go: false, documentsOnly: false, rustCrates: ["anima-app", "hub"] })).toContain("rust_crates=anima-app,hub");
  });

  test("when each Rust change is a test of a crate, the Rust jobs take only those crates", () => {
    const owner = (path: string) => ({ "crates/hub/src/a/tests.rs": "hub", "crates/hub/tests/it/x.rs": "hub", "crates/cli/src/tests/b.rs": "cli" })[path];
    expect(jobsFor(["crates/hub/src/a/tests.rs", "crates/cli/src/tests/b.rs", "crates/hub/tests/it/x.rs", "docs/x.md"], owner)).toEqual({ rust: true, go: false, documentsOnly: false, rustCrates: ["cli", "hub"] });
    expect(jobsFor(["crates/hub/src/a/tests.rs", "apps/client/src/x.ts", "link/tunnel/t.go"], owner)).toEqual({ rust: true, go: true, documentsOnly: false, rustCrates: ["hub"] });
  });

  test("one Rust change that is not a test, or a change of the runners, gives the Rust jobs each crate", () => {
    const owner = (path: string) => (path.endsWith("tests.rs") ? "hub" : undefined);
    expect(jobsFor(["crates/hub/src/a/tests.rs", "crates/hub/src/a.rs"], owner).rustCrates).toBeUndefined();
    expect(jobsFor(["crates/hub/src/a/tests.rs", "Cargo.lock"], owner).rustCrates).toBeUndefined();
    expect(jobsFor(["crates/hub/src/a/tests.rs", ".github/workflows/gates.yml"], owner)).toEqual(EACH_JOB);
    expect(jobsFor(["apps/client/src/x.ts"], owner).rustCrates).toBeUndefined();
  });
});

describe("the jobs of a push to dev (gates.ts --dev-push)", () => {
  const QUALITY_ONLY = { rust: false, go: false, documentsOnly: true };
  const proved = new Map([["tree-a", 101]]);

  test("a tree that a run on dev or a train passed needs only the quality gates, and names that run", () => {
    const chosen = devPushJobs({ tree: "tree-a", before: { tree: "tree-z", changed: ["crates/hub/src/lib.rs"] }, proved });
    expect(chosen.jobs).toEqual(QUALITY_ONLY);
    expect(chosen.provedBy).toBe(101);
    expect(jobLines(chosen.jobs, chosen.provedBy)).toContain("proved=101");
  });

  test("documents only, pushed onto a proved tree, need only the quality gates", () => {
    const chosen = devPushJobs({ tree: "tree-b", before: { tree: "tree-a", changed: ["docs/claims/x.md", "AGENTS.md"] }, proved });
    expect(chosen.jobs).toEqual(QUALITY_ONLY);
    expect(chosen.provedBy).toBeUndefined();
  });

  test("code pushed onto a proved tree runs each job", () => {
    expect(devPushJobs({ tree: "tree-b", before: { tree: "tree-a", changed: ["docs/x.md", "apps/client/src/x.ts"] }, proved }).jobs).toEqual(EACH_JOB);
  });

  test("a tree that no run proved always runs each job, also for documents only", () => {
    const unproved = [
      devPushJobs({ tree: "tree-c", before: { tree: "tree-b", changed: ["docs/claims/x.md"] }, proved }),
      devPushJobs({ tree: "tree-c", proved }),
      devPushJobs({ tree: "tree-c", before: { tree: "tree-a", changed: [] }, proved }),
      devPushJobs({ tree: "tree-a", before: { tree: "tree-a", changed: [] }, proved: new Map() }),
    ];
    for (const chosen of unproved) {
      expect(chosen.jobs).toEqual(EACH_JOB);
      expect(chosen.provedBy).toBeUndefined();
    }
  });
});
