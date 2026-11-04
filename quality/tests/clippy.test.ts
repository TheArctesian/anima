import { afterAll, describe, expect, test } from "bun:test";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const repositoryRoot = resolve(import.meta.dir, "../..");
const scratch = mkdtempSync(join(tmpdir(), "anima-clippy-proof-"));
const targetDir = join(scratch, "target");
const timeoutMs = 300_000;

afterAll(() => {
  rmSync(scratch, { recursive: true, force: true });
});

type TomlValue = string | number | boolean | TomlValue[] | { [key: string]: TomlValue };

const tomlValue = (value: TomlValue): string => {
  if (Array.isArray(value)) return `[${value.map(tomlValue).join(", ")}]`;
  if (typeof value === "object") {
    return `{ ${Object.entries(value).map(([key, entry]) => `${key} = ${tomlValue(entry)}`).join(", ")} }`;
  }
  return JSON.stringify(value);
};

/** The root `[workspace.lints.*]` tables, rewritten as a crate's `[lints.*]` tables. */
const workspaceLints = (): string => {
  const root = Bun.TOML.parse(readFileSync(join(repositoryRoot, "Cargo.toml"), "utf8")) as {
    workspace?: { lints?: Record<string, Record<string, TomlValue>> };
  };
  const tables = root.workspace?.lints;
  if (tables === undefined) throw new Error("Cargo.toml has no [workspace.lints] tables");
  return Object.entries(tables)
    .map(([tool, rules]) => {
      const lines = Object.entries(rules).map(([rule, setting]) => `${rule} = ${tomlValue(setting)}`);
      return `[lints.${tool}]\n${lines.join("\n")}`;
    })
    .join("\n\n");
};

interface ClippyRun {
  passed: boolean;
  output: string;
}

const runClippy = (name: string, source: string): ClippyRun => {
  const cargo = Bun.which("cargo");
  if (cargo === null) {
    throw new Error("cargo is not on PATH: the Clippy limits proof cannot run (CQ-024: never skipped)");
  }
  const crateDir = join(scratch, name);
  mkdirSync(join(crateDir, "src"), { recursive: true });
  const manifest = `[package]\nname = "${name}"\nversion = "0.0.0"\nedition = "2024"\npublish = false\n\n${workspaceLints()}\n`;
  writeFileSync(join(crateDir, "Cargo.toml"), manifest);
  writeFileSync(join(crateDir, "src", "lib.rs"), source);
  const result = Bun.spawnSync([cargo, "clippy", "--", "-D", "warnings"], {
    cwd: crateDir,
    env: { ...process.env, CLIPPY_CONF_DIR: repositoryRoot, CARGO_TARGET_DIR: targetDir, CARGO_TERM_COLOR: "never" },
  });
  return {
    passed: result.exitCode === 0,
    output: `${result.stdout.toString()}${result.stderr.toString()}`,
  };
};

const expectRejected = (run: ClippyRun, lint: string) => {
  expect(run.passed).toBe(false);
  // Clippy prints the lint's documentation link as `...#<lint_name>`; the lint name is not always spelled the same elsewhere.
  expect(run.output).toMatch(new RegExp(`#${lint}\\s`));
};

const expectAccepted = (run: ClippyRun) => {
  if (!run.passed) throw new Error(`Clippy rejected a clean crate:\n${run.output}`);
};

const statements = (count: number) =>
  Array.from({ length: count }, () => "    std::hint::black_box(1_u32);").join("\n");

const nested = (depth: number) => {
  // A statement before each inner block keeps `collapsible_if` from firing instead.
  const open = Array.from({ length: depth }, () => "if std::hint::black_box(true) {\nstd::hint::black_box(0_u32);").join("\n");
  return `pub fn deep() {\n${open}\n${"}\n".repeat(depth)}}\n`;
};

const branches = (count: number) =>
  Array.from({ length: count }, (_, n) => `    if std::hint::black_box(${n}_u32) == 1 {\n        std::hint::black_box(${n}_u32);\n    }`).join("\n");

describe("CQ-010 Clippy rejects code over the limits in clippy.toml", () => {
  test("CQ-010 a 61-line function is rejected (too_many_lines)", () => {
    expectRejected(runClippy("long_function", `pub fn long() {\n${statements(61)}\n}\n`), "too_many_lines");
  }, timeoutMs);

  test("CQ-010 a 60-line function is accepted", () => {
    expectAccepted(runClippy("limit_function", `pub fn limit() {\n${statements(60)}\n}\n`));
  }, timeoutMs);

  test("CQ-010 a function with 5 parameters is rejected (too_many_arguments)", () => {
    const source = "#[must_use]\npub fn wide(a: u8, b: u8, c: u8, d: u8, e: u8) -> u8 {\n    a ^ b ^ c ^ d ^ e\n}\n";
    expectRejected(runClippy("wide_function", source), "too_many_arguments");
  }, timeoutMs);

  test("CQ-010 a function with 4 parameters is accepted", () => {
    const source = "#[must_use]\npub fn narrow(a: u8, b: u8, c: u8, d: u8) -> u8 {\n    a ^ b ^ c ^ d\n}\n";
    expectAccepted(runClippy("narrow_function", source));
  }, timeoutMs);

  // excessive_nesting is proved through its threshold: -D warnings keeps it on even without the explicit deny.
  test("CQ-010 nesting of 5 blocks in a body is rejected (excessive_nesting)", () => {
    expectRejected(runClippy("deep_nesting", nested(5)), "excessive_nesting");
  }, timeoutMs);

  test("CQ-010 nesting of 4 blocks in a body is accepted", () => {
    expectAccepted(runClippy("limit_nesting", nested(4)));
  }, timeoutMs);

  test("CQ-010 cognitive complexity over 15 is rejected (cognitive_complexity)", () => {
    const source = `pub fn tangled() {\n${branches(20)}\n}\n`;
    expectRejected(runClippy("tangled_function", source), "cognitive_complexity");
  }, timeoutMs);

  test("CQ-010 cognitive complexity under the limit is accepted", () => {
    expectAccepted(runClippy("simple_function", `pub fn simple() {\n${branches(10)}\n}\n`));
  }, timeoutMs);
});

describe("CQ-040 Clippy rejects lint suppression without a reason", () => {
  test("CQ-040 an #[allow(...)] attribute is rejected (allow_attributes)", () => {
    const source = "#[allow(dead_code)]\nfn unused() {}\n";
    expectRejected(runClippy("allow_attribute", source), "allow_attributes");
  }, timeoutMs);

  test("CQ-040 a clean file passes", () => {
    expectAccepted(runClippy("clean_file", "#[must_use]\npub fn add(a: u8, b: u8) -> u8 {\n    a.wrapping_add(b)\n}\n"));
  }, timeoutMs);
});
