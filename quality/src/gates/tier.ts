import type { Gate } from "./gates.ts";

/** One Rust package or one bun package: its folder from the repository root, and its name. */
export interface Member {
  folder: string;
  name: string;
}

/** The packages of the repository, as `cargo metadata` and the workspaces of `package.json` give them. */
export interface Layout {
  crates: Member[];
  packages: Member[];
}

/** The members of one language that a change touches: each of them, or these by name. */
export type Touched = "all" | string[];

/** What the fast tier runs for a set of changed paths, and why. */
export type Choice =
  | { tier: "nothing"; reasons: string[] }
  | { tier: "documents"; reasons: string[] }
  | { tier: "code"; rust: Touched; go: boolean; bun: Touched; reasons: string[] }
  | { tier: "full"; reasons: string[] };

/** Files that change how each Rust package builds. */
const RUST_ROOT = ["Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "clippy.toml", "rustfmt.toml", ".config/", ".cargo/"];
/** Files that change how each bun package builds, lints or tests. `design/` holds the tokens that the client reads. */
const BUN_ROOT = ["package.json", "bun.lock", "bun.lockb", "bunfig.toml", "tsconfig.json", "tsconfig.base.json", "eslint.config.js", "eslint.config.ts", "design/"];
/** `contracts/` holds the GraphQL schema that the command line and the client compile against. */
const CONTRACTS = "contracts/";
const GO_MODULE = "link/";
/** The tests of the deploy package read the workflow files. */
const WORKFLOWS = ".github/";
const WORKFLOW_TESTS = "@anima/deploy";

function isDocument(path: string): boolean {
  return path.startsWith("docs/") || (!path.includes("/") && path.endsWith(".md"));
}

function under(path: string, prefix: string): boolean {
  return prefix.endsWith("/") ? path.startsWith(prefix) : path === prefix;
}

/** The member whose folder holds `path`; the longest folder wins, so `apps/desktop/src-tauri` is the crate. */
function ownerOf(path: string, members: { member: Member; rust: boolean }[]) {
  return members
    .filter(({ member }) => path.startsWith(`${member.folder}/`))
    .sort((one, other) => other.member.folder.length - one.member.folder.length)[0];
}

function add(touched: Touched, name: string): Touched {
  if (touched === "all" || touched.includes(name)) return touched;
  return [...touched, name];
}

/**
 * Chooses the fast tier from the changed paths. Pure. A path that no rule knows gives the full
 * list: the fast tier never guesses that a file is safe.
 */
export function choose(paths: string[], layout: Layout): Choice {
  if (paths.length === 0) return { tier: "nothing", reasons: ["no file is different from the base"] };
  if (paths.every(isDocument)) return { tier: "documents", reasons: [`${paths.length} changed file(s), each a document`] };
  const members = [
    ...layout.crates.map((member) => ({ member, rust: true })),
    ...layout.packages.map((member) => ({ member, rust: false })),
  ];
  let rust: Touched = [];
  let bun: Touched = [];
  let go = false;
  const reasons: string[] = [];
  for (const path of paths.filter((each) => !isDocument(each))) {
    const owner = ownerOf(path, members);
    if (path.startsWith(GO_MODULE)) {
      go = true;
    } else if (path.startsWith(CONTRACTS)) {
      [rust, bun] = ["all", "all"];
      reasons.push(`${path}: a contract, so each Rust and bun package`);
    } else if (RUST_ROOT.some((root) => under(path, root))) {
      rust = "all";
      reasons.push(`${path}: each Rust package`);
    } else if (BUN_ROOT.some((root) => under(path, root))) {
      bun = "all";
      reasons.push(`${path}: each bun package`);
    } else if (path.startsWith(WORKFLOWS)) {
      bun = add(bun, WORKFLOW_TESTS);
    } else if (owner?.rust) {
      rust = add(rust, owner.member.name);
    } else if (owner !== undefined) {
      bun = add(bun, owner.member.name);
    } else {
      return { tier: "full", reasons: [`${path}: no rule of the fast tier knows this path`] };
    }
  }
  return { tier: "code", rust, go, bun, reasons };
}

function packageFlags(names: string[]): string[] {
  return names.flatMap((name) => ["-p", name]);
}

function rustGates(full: Gate[], touched: Touched): Gate[] {
  if (touched === "all") return full.filter((gate) => gate.group === "rust");
  if (touched.length === 0) return [];
  const named = (name: string) => full.filter((gate) => gate.name === name);
  const flags = packageFlags(touched);
  return [
    ...named("fmt"),
    { name: "clippy", group: "rust", argv: ["cargo", "clippy", ...flags, "--all-targets", "--", "-D", "warnings"] },
    {
      name: "rust-test",
      group: "rust",
      // A package with no test must not fail the tier: the full list still runs each test.
      argv: ["bun", "quality/bin/with-postgres.ts", "cargo", "nextest", "run", ...flags, "--no-tests=pass"],
      requires: named("rust-test")[0]?.requires,
    },
    ...(touched.includes("anima-app") ? named("pinning") : []),
  ];
}

function bunGates(full: Gate[], touched: Touched): Gate[] {
  if (touched === "all") return full.filter((gate) => gate.group === "bun");
  if (touched.length === 0) return [];
  const filters = touched.flatMap((name) => ["--filter", name]);
  const script = (name: string): Gate => ({ name, group: "bun", argv: ["bun", "run", ...filters, name] });
  return [
    ...full.filter((gate) => gate.name === "install"),
    script("lint"),
    script("check"),
    script("test"),
    ...(touched.includes("@anima/client") ? full.filter((gate) => gate.name === "build") : []),
  ];
}

/**
 * The gates of the fast tier for `choice`, taken from the full list `full` so that a gate has one
 * definition. The quality gates run for each code change: they take seconds. The doctests run only
 * with each Rust package, and the dependents of a touched package are not tested: the full list does that.
 */
export function fastGates(choice: Choice, full: Gate[]): Gate[] {
  switch (choice.tier) {
    case "nothing":
      return [];
    case "documents":
      return full.filter((gate) => gate.name === "documents");
    case "full":
      return full;
    case "code":
      return [
        ...rustGates(full, choice.rust),
        ...(choice.go ? full.filter((gate) => gate.group === "go") : []),
        ...bunGates(full, choice.bun),
        ...full.filter((gate) => gate.group === "quality"),
      ];
  }
}

/** One line for the start of a fast run: the tier and its reasons. */
export function describeChoice(choice: Choice): string {
  const part = (label: string, touched: Touched) =>
    touched === "all" ? `${label}: each package` : touched.length > 0 ? `${label}: ${touched.join(", ")}` : undefined;
  const parts =
    choice.tier === "code"
      ? [part("Rust", choice.rust), choice.go ? "Go: the link" : undefined, part("bun", choice.bun)].filter((each) => each !== undefined)
      : [];
  return [`fast tier: ${choice.tier}`, ...parts, ...choice.reasons].join("; ");
}
