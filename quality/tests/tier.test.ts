import { describe, expect, test } from "bun:test";
import { gateList, ofGroups } from "../src/gates/gates.ts";
import { choose, describeChoice, fastGates, type Layout } from "../src/gates/tier.ts";

const LAYOUT: Layout = {
  crates: [
    { folder: "crates/hub", name: "anima-hub" },
    { folder: "crates/daemon", name: "anima-daemon" },
    { folder: "apps/desktop/src-tauri", name: "anima-app" },
  ],
  packages: [
    { folder: "apps/client", name: "@anima/client" },
    { folder: "apps/desktop", name: "@anima/desktop" },
    { folder: "quality", name: "@anima/quality" },
    { folder: "deploy", name: "@anima/deploy" },
    { folder: "deploy/dns", name: "@anima/dns" },
  ],
};
const FULL = gateList("phase-2");
const names = (paths: string[]) => fastGates(choose(paths, LAYOUT), FULL).map((gate) => gate.name);
const argvOf = (paths: string[], name: string) =>
  fastGates(choose(paths, LAYOUT), FULL).find((gate) => gate.name === name)?.argv.join(" ");
const QUALITY = ["limits", "names", "imports", "documents", "baseline"];

describe("the fast tier (gates:fast)", () => {
  test("a change of documents only runs the documents gate", () => {
    expect(names(["docs/plans/x-plan.md", "docs/rulings.md", "AGENTS.md", ".gitattributes"])).toEqual(["documents"]);
  });

  test("no change runs nothing", () => {
    expect(names([])).toEqual([]);
    expect(choose([], LAYOUT).tier).toBe("nothing");
  });

  test("a change in one crate runs format, lint and the tests of that crate, then the quality gates", () => {
    const paths = ["crates/daemon/src/hub.rs", "docs/need-human-testing.md"];
    expect(names(paths)).toEqual(["fmt", "clippy", "rust-test", ...QUALITY]);
    expect(argvOf(paths, "clippy")).toBe("cargo clippy -p anima-daemon --all-targets -- -D warnings");
    expect(argvOf(paths, "rust-test")).toBe(
      "bun quality/bin/with-postgres.ts cargo nextest run -p anima-daemon --no-tests=pass",
    );
  });

  test("the desktop shell is a crate, not the bun package around it, and brings its pinning test", () => {
    const paths = ["apps/desktop/src-tauri/src/link.rs"];
    expect(names(paths)).toEqual(["fmt", "clippy", "rust-test", "pinning", ...QUALITY]);
    expect(argvOf(paths, "clippy")).toContain("-p anima-app");
  });

  test("a change of the link runs the six Go gates and no Rust or bun gate", () => {
    expect(names(["link/devicelink/devicelink.go"])).toEqual([...ofGroups(FULL, ["go"]).map((gate) => gate.name), ...QUALITY]);
  });

  test("a change in one bun package runs install and the lint, check and tests of that package", () => {
    const paths = ["deploy/dns/src/zone.ts"];
    expect(names(paths)).toEqual(["install", "lint", "check", "test", ...QUALITY]);
    expect(argvOf(paths, "test")).toBe("bun run --filter @anima/dns test");
  });

  test("a change of the client also builds it", () => {
    expect(names(["apps/client/src/lib/x.ts"])).toEqual(["install", "lint", "check", "test", "build", ...QUALITY]);
  });

  test("a file that each Rust package depends on runs the whole Rust group, doctests included", () => {
    for (const path of ["Cargo.lock", "Cargo.toml", "rust-toolchain.toml", ".config/nextest.toml"]) {
      expect(names([path])).toEqual([...ofGroups(FULL, ["rust"]).map((gate) => gate.name), ...QUALITY]);
    }
  });

  test("a contract runs each Rust and each bun package", () => {
    expect(names(["contracts/schema.graphql"])).toEqual(ofGroups(FULL, ["rust", "bun", "quality"]).map((gate) => gate.name));
  });

  test("a workflow file runs the tests of the deploy package, which read it", () => {
    expect(argvOf([".github/workflows/gates.yml"], "test")).toBe("bun run --filter @anima/deploy test");
  });

});

describe("the fast tier: what it does not guess", () => {
  test("a path that no rule knows runs the full list", () => {
    const choice = choose(["crates/hub/src/main.rs", "scripts/unknown.sh"], LAYOUT);
    expect(choice.tier).toBe("full");
    expect(fastGates(choice, FULL)).toEqual(FULL);
    expect(describeChoice(choice)).toContain("scripts/unknown.sh");
  });

  test("changes in two languages run the gates of the two", () => {
    const paths = ["crates/hub/src/main.rs", "crates/daemon/src/lib.rs", "link/main.go", "apps/client/src/a.ts"];
    expect(names(paths)).toEqual([
      "fmt", "clippy", "rust-test",
      ...ofGroups(FULL, ["go"]).map((gate) => gate.name),
      "install", "lint", "check", "test", "build",
      ...QUALITY,
    ]);
    expect(argvOf(paths, "clippy")).toContain("-p anima-hub -p anima-daemon");
    expect(describeChoice(choose(paths, LAYOUT))).toContain("Rust: anima-hub, anima-daemon");
  });
});

describe("the groups of the gates (--group)", () => {
  test("each gate has one group, and the groups together are the full list in its order", () => {
    const grouped = ofGroups(FULL, ["rust", "go", "bun", "quality"]);
    expect(grouped).toEqual(FULL);
    expect(ofGroups(FULL, ["go"]).map((gate) => gate.name)).toEqual([
      "go-fmt", "go-vet", "go-staticcheck", "go-revive", "go-test", "go-build",
    ]);
    expect(ofGroups(FULL, ["rust"]).map((gate) => gate.name)).toEqual(["fmt", "clippy", "rust-test", "doctest", "pinning"]);
  });
});
