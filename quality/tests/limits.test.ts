import { describe, expect, test } from "bun:test";
import { countCodeLines } from "../src/lexing.ts";
import { checkLimits } from "../src/limits.ts";
import type { SourceFile } from "../src/source.ts";

const lines = (count: number, line = "let x = 1;") => Array.from({ length: count }, () => line).join("\n");
const file = (path: string, text = ""): SourceFile => ({ path, text });

describe("CQ-010 code lines per file", () => {
  test("a 301-code-line file fails", () => {
    const violations = checkLimits([file("crates/hub/src/big.rs", lines(301))], {});
    expect(violations).toEqual([{ path: "crates/hub/src/big.rs", signal: "file-lines", detail: "301 > 300" }]);
  });

  test("300 code lines plus blanks and comments pass", () => {
    const text = `${lines(300)}\n\n\n// note\n/// doc\n//! inner\n/* a\n b\n c */\n`;
    expect(checkLimits([file("crates/hub/src/ok.rs", text)], {})).toEqual([]);
  });

  test("comment syntax per language is excluded", () => {
    expect(countCodeLines("<!-- a\nb -->\n<p>x</p>\n<script>\n// c\nlet a = 1;\n</script>", ".svelte")).toBe(4);
    expect(countCodeLines("a { color: red; } /* c */\n/* x */\n", ".css")).toBe(1);
    expect(countCodeLines('let u = "http://x"; // c\n', ".ts")).toBe(1);
    expect(countCodeLines("let c = '\"'; // c\nlet l: &'a str;\n", ".rs")).toBe(2);
  });

  test("CQ-010 nested Rust block comments and raw strings are lexed", () => {
    expect(countCodeLines("/* a /* b */ still comment */\nlet x = 1;\n", ".rs")).toBe(1);
    expect(countCodeLines('let s = r#"say "hi" // not a comment\n/* nor this"#;\nlet y = 2;\n', ".rs")).toBe(3);
    expect(countCodeLines("let x = br##\"a\"# b\"##; // c\n", ".rs")).toBe(1);
    expect(countCodeLines('let xr = 1; xr#"a\n/* \n"#;\nlet z = 1;\n*/', ".rs")).toBe(1);
    expect(countCodeLines("/* /* x */ y */ // z\n", ".ts")).toBe(1);
  });

  test("files with other extensions are not measured", () => {
    expect(checkLimits([file("docs/big.md", lines(500))], {})).toEqual([]);
  });
});

describe("CQ-010 baseline lets a file over the limit pass", () => {
  const big = file("crates/hub/src/big.rs", lines(320));

  test("a baselined over-limit file passes when the value is at least the measure", () => {
    expect(checkLimits([big], { "crates/hub/src/big.rs": { "file-lines": 320 } })).toEqual([]);
  });

  test("a file over its baseline fails", () => {
    const violations = checkLimits([big], { "crates/hub/src/big.rs": { "file-lines": 310 } });
    expect(violations).toEqual([{ path: big.path, signal: "file-lines", detail: "320 > 310" }]);
  });
});

describe("REPO-005 files per folder", () => {
  const folder = (count: number, dir = "crates/hub/src/devices") =>
    Array.from({ length: count }, (_, index) => file(`${dir}/f${index}.rs`, "fn a() {}"));

  test("13 files in a folder fail", () => {
    expect(checkLimits(folder(13), {})).toEqual([
      { path: "crates/hub/src/devices", signal: "folder-files", detail: "13 > 12" },
    ]);
  });

  test("12 files pass, and non-source files do not count", () => {
    const files = [...folder(12), file("crates/hub/src/devices/notes.md")];
    expect(checkLimits(files, {})).toEqual([]);
  });

  test("a baselined folder passes; folders outside the roots are ignored", () => {
    expect(checkLimits(folder(14), { "crates/hub/src/devices": { "folder-files": 14 } })).toEqual([]);
    expect(checkLimits(folder(20, "scripts"), {})).toEqual([]);
  });
});
