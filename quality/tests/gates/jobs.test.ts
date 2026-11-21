import { describe, expect, test } from "bun:test";
import { EACH_JOB, jobLines, jobsFor } from "../../src/gates/jobs.ts";

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
    expect(jobLines({ rust: false, go: true, documentsOnly: false })).toEqual(["rust=false", "go=true", "documents_only=false"]);
  });
});
