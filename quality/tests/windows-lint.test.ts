import { describe, expect, test } from "bun:test";
import { windowsLintArgv } from "../src/gates/windows-lint.ts";

describe("windowsLintArgv", () => {
  test("lints each target of the workspace for Windows GNU with warnings denied, as the gate clippy does", () => {
    expect(windowsLintArgv([]).join(" ")).toBe("cargo clippy --workspace --all-targets --target x86_64-pc-windows-gnu --locked --keep-going -- -D warnings");
  });

  test("leaves out only the crates that are named", () => {
    expect(windowsLintArgv([{ crate: "anima-app", reason: "x" }]).join(" ")).toContain("--workspace --exclude anima-app --all-targets");
  });
});
