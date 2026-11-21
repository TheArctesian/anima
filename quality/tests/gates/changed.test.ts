import { describe, expect, test } from "bun:test";
import { changedGates, dependents } from "../../src/gates/changed.ts";
import { gateList } from "../../src/gates/gates.ts";
import { choose, type Layout } from "../../src/gates/tier.ts";

const LAYOUT: Layout = {
  crates: [
    { folder: "crates/protocol", name: "anima-protocol" },
    { folder: "crates/hub", name: "anima-hub", uses: ["anima-protocol"] },
    { folder: "crates/anima-cli", name: "anima-cli", uses: ["anima-protocol"] },
    { folder: "apps/desktop/src-tauri", name: "anima-app", uses: ["anima-cli"] },
  ],
  packages: [
    { folder: "design", name: "@anima/design" },
    { folder: "apps/client", name: "@anima/client", uses: ["@anima/design"] },
    { folder: "quality", name: "@anima/quality" },
    { folder: "deploy", name: "@anima/deploy" },
  ],
};
const FULL = gateList("phase-2");
const run = (paths: string[]) => changedGates(choose(paths, LAYOUT), FULL, LAYOUT, paths);
const names = (paths: string[]) => run(paths).gates.map((gate) => gate.name);
const argvOf = (paths: string[], name: string) => run(paths).gates.find((gate) => gate.name === name)?.argv.join(" ");
const REST = ["limits", "names", "imports", "baseline"];

describe("the checks of the changed paths (gates:changed)", () => {
  test("the documents gate runs first, also when no file changed", () => {
    expect(names([])).toEqual(["documents"]);
    expect(names(["docs/rulings.md"])).toEqual(["documents"]);
    expect(names(["crates/hub/src/main.rs"])[0]).toBe("documents");
  });

  test("the tests run for the touched crate; Clippy also checks the crates that use it, through others too", () => {
    const paths = ["crates/protocol/src/lib.rs"];
    expect(names(paths)).toEqual(["documents", "fmt", "clippy", "rust-test", ...REST]);
    expect(argvOf(paths, "clippy")).toBe("cargo clippy -p anima-protocol -p anima-hub -p anima-cli -p anima-app --all-targets -- -D warnings");
    expect(argvOf(paths, "rust-test")).toBe("bun quality/bin/with-postgres.ts cargo nextest run -p anima-protocol --no-tests=pass");
    expect(run(paths).notes.join("\n")).toContain("anima-hub, anima-cli, anima-app");
  });

  test("a touched bun package runs its tests; the packages that use it get lint and the check of types", () => {
    const paths = ["design/tokens/colour.ts"];
    expect(argvOf(["apps/client/src/app.ts"], "test")).toBe("bun run --filter @anima/client test");
    expect(names(["apps/client/src/app.ts"])).toContain("build");
    expect(argvOf(["quality/src/x.ts"], "check")).toBe("bun run --filter @anima/quality check");
    expect(dependents(["@anima/design"], LAYOUT.packages)).toEqual(["@anima/client"]);
    // `design/` is a root of each bun package: lint and types for each, and the tests of the design package only.
    expect(names(paths)).toEqual(["documents", "install", "lint", "check", "test", ...REST]);
    expect(argvOf(paths, "test")).toBe("bun run --filter @anima/design test");
  });

  test("when a root file touches each package, the packages whose own files changed still run their tests", () => {
    const paths = ["package.json", "quality/src/gates/changed.ts"];
    expect(names(paths)).toEqual(["documents", "install", "lint", "check", "test", ...REST]);
    expect(argvOf(paths, "test")).toBe("bun run --filter @anima/quality test");
    expect(argvOf(["Cargo.lock", "crates/hub/src/main.rs"], "rust-test")).toBe("bun quality/bin/with-postgres.ts cargo nextest run -p anima-hub --no-tests=pass");
  });

  test("a change for each package or an unknown path never runs the full list here", () => {
    expect(names(["Cargo.lock"])).toEqual(["documents", "fmt", "clippy", ...REST]);
    expect(names(["somewhere-new/file.txt"])).toEqual(["documents", ...REST]);
    expect(run(["somewhere-new/file.txt"]).notes[0]).toContain("runs on GitHub");
  });
});
