import { describe, expect, test } from "bun:test";
import { gateList } from "../../src/gates/gates.ts";
import { crateGates, testOwner } from "../../src/gates/test-only.ts";

const CRATES = [
  { folder: "crates/hub", name: "anima-hub" },
  { folder: "apps/desktop/src-tauri", name: "anima-app" },
  { folder: "apps/desktop", name: "outer" },
];
const DECLARED = "pub mod model;\n\n#[cfg(test)]\nmod tests;\n";

function owner(path: string, files: Record<string, string>, changed: string[] = []): string | undefined {
  return testOwner(path, CRATES, (file) => files[file], new Set(changed));
}

describe("a path that only the tests of one crate read", () => {
  test("a file of the crate's tests/ folder belongs to that crate, also in a crate inside a crate's folder", () => {
    expect(owner("crates/hub/tests/it/devices.rs", {})).toBe("anima-hub");
    expect(owner("apps/desktop/src-tauri/tests/pinning/main.rs", {})).toBe("anima-app");
  });

  test("a tests.rs or a file in a folder tests/ belongs to the crate when its parent declares it #[cfg(test)]", () => {
    expect(owner("crates/hub/src/devices/tests.rs", { "crates/hub/src/devices.rs": DECLARED })).toBe("anima-hub");
    expect(owner("crates/hub/src/devices/tests/enrol.rs", { "crates/hub/src/devices/mod.rs": DECLARED })).toBe("anima-hub");
    expect(owner("crates/hub/src/tests.rs", { "crates/hub/src/lib.rs": DECLARED })).toBe("anima-hub");
  });

  test("a test cfg with more conditions, a visibility and a module named <x>_tests are test-only too", () => {
    expect(owner("crates/hub/src/a/tests.rs", { "crates/hub/src/a.rs": "#[cfg(all(test, unix))]\npub(crate) mod tests;\n" })).toBe("anima-hub");
    expect(owner("crates/hub/src/a/windows_tests.rs", { "crates/hub/src/a.rs": "#[cfg(all(windows, test))]\nmod windows_tests;\n" })).toBe("anima-hub");
    expect(owner("crates/hub/src/a/tests/b.rs", { "crates/hub/src/a.rs": "#[cfg(test)]\npub(in crate::a) mod tests;\n" })).toBe("anima-hub");
  });

  test("a cfg that a build outside the tests can meet is not test-only", () => {
    for (const cfg of ["#[cfg(any(test, feature = \"x\"))]", "#[cfg(not(test))]", "#[cfg(all(unix, any(test, debug_assertions)))]", "#[cfg(unix)]"]) {
      expect(owner("crates/hub/src/a/tests.rs", { "crates/hub/src/a.rs": `${cfg}\nmod tests;\n` }), cfg).toBeUndefined();
    }
  });

  test("a module tests that the parent compiles outside a test build is not test-only", () => {
    expect(owner("crates/hub/src/devices/tests.rs", { "crates/hub/src/devices.rs": "pub mod tests;\n" })).toBeUndefined();
    expect(owner("crates/hub/src/devices/tests.rs", { "crates/hub/src/devices.rs": "mod tests;\n" })).toBeUndefined();
    expect(owner("crates/hub/src/devices/tests.rs", { "crates/hub/src/devices.rs": "#[cfg(test)]\n#[path = \"x.rs\"]\nmod tests;\n" })).toBeUndefined();
    // A library and a program can both declare it; each declaration must be a test one.
    expect(owner("crates/hub/src/tests.rs", { "crates/hub/src/lib.rs": DECLARED, "crates/hub/src/main.rs": "mod tests;\n" })).toBeUndefined();
  });

  test("a parent that the branch also changed, or that is not there, is not trusted", () => {
    expect(owner("crates/hub/src/devices/tests.rs", { "crates/hub/src/devices.rs": DECLARED }, ["crates/hub/src/devices.rs"])).toBeUndefined();
    expect(owner("crates/hub/src/devices/tests.rs", {})).toBeUndefined();
  });

  test("other code of a crate, a file that is not Rust and a path outside each crate are not test-only", () => {
    expect(owner("crates/hub/src/devices.rs", {})).toBeUndefined();
    expect(owner("crates/hub/src/tests_support.rs", {})).toBeUndefined();
    expect(owner("crates/hub/tests/fixture.json", {})).toBeUndefined();
    expect(owner("crates/hub/build.rs", {})).toBeUndefined();
    expect(owner("contracts/tests/x.rs", {})).toBeUndefined();
  });
});

describe("the gates of a run that tests only some crates", () => {
  const names = (crates: string[]) => crateGates(gateList("x").filter((gate) => gate.group === "rust"), crates);

  test("Clippy and nextest take only those crates, the doctests do not run, and fmt is as it was", () => {
    const gates = names(["anima-hub", "anima-cli"]);
    expect(gates.map((gate) => gate.name)).toEqual(["fmt", "clippy", "rust-test"]);
    expect(gates.find((gate) => gate.name === "clippy")?.argv.join(" ")).toBe("cargo clippy -p anima-hub -p anima-cli --all-targets -- -D warnings");
    expect(gates.find((gate) => gate.name === "rust-test")?.argv.join(" ")).toContain("cargo nextest run -p anima-hub -p anima-cli --no-tests=pass");
    expect(gates.find((gate) => gate.name === "fmt")).toEqual(gateList("x").find((gate) => gate.name === "fmt"));
  });

  test("pinning, the test of the desktop shell, runs when the shell is one of the crates", () => {
    expect(names(["anima-app"]).map((gate) => gate.name)).toContain("pinning");
  });
});
