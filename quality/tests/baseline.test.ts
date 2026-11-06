import { describe, expect, test } from "bun:test";
import { mkdtempSync, mkdirSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { BaseRefError, BaselineShapeError, baseRefFrom, growth, parseBaseline } from "../src/baseline.ts";
import { parseGeneratedList, withoutGenerated } from "../src/glob.ts";
import { listMeasurableFiles, readFileAtRef } from "../src/repository.ts";

const base = { "a.rs": { "file-lines": 400 } };

describe("CQ-011 baseline is shrink-only", () => {
  test("a larger value fails", () => {
    expect(growth(base, { "a.rs": { "file-lines": 401 } })).toEqual([
      { path: "a.rs", signal: "file-lines", detail: "401 > 400" },
    ]);
  });

  test("a new path and a new signal fail", () => {
    expect(growth(base, { ...base, "b.rs": { "file-lines": 1 } })).toHaveLength(1);
    expect(growth(base, { "a.rs": { "file-lines": 400, "folder-files": 13 } })).toHaveLength(1);
  });

  test("shrinking, removing and staying equal pass", () => {
    expect(growth(base, { "a.rs": { "file-lines": 350 } })).toEqual([]);
    expect(growth(base, {})).toEqual([]);
    expect(growth(base, base)).toEqual([]);
  });

  test("invalid JSON and wrong shape are rejected", () => {
    expect(() => parseBaseline("{")).toThrow(BaselineShapeError);
    expect(() => parseBaseline("[]")).toThrow(BaselineShapeError);
    expect(() => parseBaseline('{"a.rs": 5}')).toThrow(BaselineShapeError);
    expect(() => parseBaseline('{"a.rs": {"lines": 5}}')).toThrow(BaselineShapeError);
    expect(() => parseBaseline('{"a.rs": {"file-lines": -1}}')).toThrow(BaselineShapeError);
    expect(() => parseBaseline('{"a.rs": {"file-lines": 1.5}}')).toThrow(BaselineShapeError);
    expect(parseBaseline("{}")).toEqual({});
  });
});

describe("CQ-011 --base argument", () => {
  test("the value after --base is used, the fallback otherwise", () => {
    expect(baseRefFrom(["bun", "x", "--base", "main"], "phase-1")).toBe("main");
    expect(baseRefFrom(["bun", "x"], "phase-1")).toBe("phase-1");
  });

  test("--base with no value, or followed by a flag, is an error", () => {
    expect(() => baseRefFrom(["bun", "x", "--base"], "phase-1")).toThrow(BaseRefError);
    expect(() => baseRefFrom(["bun", "x", "--base", "--other"], "phase-1")).toThrow(BaseRefError);
  });
});

describe("CQ-011 reading the base ref", () => {
  const git = (cwd: string, ...args: string[]) =>
    Bun.spawnSync(["git", "-c", "user.name=t", "-c", "user.email=t@t", ...args], { cwd });

  test("a file missing at the ref reads as absent; an unknown ref throws", () => {
    const root = mkdtempSync(join(tmpdir(), "anima-quality-"));
    git(root, "init", "-q");
    writeFileSync(join(root, "x.txt"), "x");
    git(root, "add", ".");
    git(root, "commit", "-q", "-m", "init");
    expect(readFileAtRef(root, "HEAD", "quality/baseline.json")).toBeUndefined();
    expect(readFileAtRef(root, "HEAD", "x.txt")).toBe("x");
    expect(() => readFileAtRef(root, "no-such-ref", "x.txt")).toThrow();
  });
});

describe("CQ-012 generated files are not measured", () => {
  test("globs remove generated paths and comments are ignored", () => {
    const globs = parseGeneratedList("# c\n\ncrates/hub/migrations/**\nfoo/*.css\n");
    const paths = ["crates/hub/migrations/a/b.sql", "foo/x.css", "foo/y/x.css", "crates/hub/src/main.rs"];
    expect(withoutGenerated(paths, globs)).toEqual(["foo/y/x.css", "crates/hub/src/main.rs"]);
  });

  test("file discovery skips git-ignored files and generated globs", () => {
    const root = mkdtempSync(join(tmpdir(), "anima-quality-"));
    Bun.spawnSync(["git", "init", "-q"], { cwd: root });
    mkdirSync(join(root, "quality"));
    mkdirSync(join(root, "gen"));
    mkdirSync(join(root, "node_modules"));
    writeFileSync(join(root, ".gitignore"), "node_modules\n");
    writeFileSync(join(root, "quality", "generated.txt"), "gen/**\n");
    for (const path of ["gen/a.ts", "node_modules/b.ts", "c.ts"]) writeFileSync(join(root, path), "");
    expect(listMeasurableFiles(root)).toEqual([".gitignore", "c.ts", "quality/generated.txt"]);
  });
});
