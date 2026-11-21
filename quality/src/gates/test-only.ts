import type { Gate } from "./gates.ts";
import type { Member } from "./tier.ts";

/**
 * Changes that only tests of one crate read (`bun quality/bin/gates.ts --jobs`). Such a change
 * cannot change what any other crate builds, so the Rust jobs of a pushed branch lint and test only
 * the crates that own it. The train still runs each job before `dev` moves. A path is a test of a
 * crate when it is in the crate's `tests/` folder, or when it is in a module `tests` or `<x>_tests`
 * (its `.rs` file or a file of its folder) whose parent file declares it as `mod <name>;` directly
 * after `#[cfg(test)]` or a `#[cfg(all(...))]` with `test` among its arguments. The parent must not
 * change in the same branch: its line is read as it is now.
 */

/** The text of a file of the repository as it is now, or undefined when there is none. */
export type ReadFile = (path: string) => string | undefined;

/** The crate whose folder holds `path`: the member with the longest folder, or undefined. */
function crateOf(path: string, crates: readonly Member[]): Member | undefined {
  const owners = crates.filter((crate) => crate.folder !== "" && path.startsWith(`${crate.folder}/`));
  return owners.sort((a, b) => b.folder.length - a.folder.length)[0];
}

/** The files that can declare the module `tests` whose path in the crate starts with `parent`. */
function parentFiles(folder: string, parent: string[]): string[] {
  if (parent.length === 0) return [`${folder}/src/lib.rs`, `${folder}/src/main.rs`];
  const module = `${folder}/src/${parent.join("/")}`;
  return [`${module}.rs`, `${module}/mod.rs`];
}

/** Whether `line` compiles the next item only in a test build: `cfg(test)` or `cfg(all(test, ...))`. */
function onlyInTests(line: string | undefined): boolean {
  if (line === "#[cfg(test)]") return true;
  const all = /^#\[cfg\(all\((.*)\)\)\]$/.exec(line ?? "")?.[1];
  if (all === undefined) return false;
  let depth = 0;
  const top = [...all].map((char) => {
    depth += char === "(" ? 1 : char === ")" ? -1 : 0;
    return char === "," && depth === 0 ? "\0" : char;
  });
  return top.join("").split("\0").some((part) => part.trim() === "test");
}

/** Each declaration `mod <module>;` of `text` (with any visibility), with whether the line before it limits it to tests. */
function declarations(text: string, module: string): boolean[] {
  const declared = new RegExp(`^(pub(\\([^)]*\\))? )?mod ${module};$`);
  const lines = text.split("\n").map((line) => line.trim());
  return lines.flatMap((line, at) => (declared.test(line) ? [onlyInTests(lines[at - 1])] : []));
}

const TEST_MODULE = /^(\w+_)?tests$/;

/**
 * The crate whose tests alone read `path`, or undefined when the path is anything else. `changed`
 * are the other changed paths: a parent file that changed is not trusted. Pure.
 */
export function testOwner(path: string, crates: readonly Member[], read: ReadFile, changed: ReadonlySet<string>): string | undefined {
  const crate = crateOf(path, crates);
  if (crate === undefined || !path.endsWith(".rs")) return undefined;
  const parts = path.slice(crate.folder.length + 1).split("/");
  if (parts[0] === "tests" && parts.length > 1) return crate.name;
  if (parts[0] !== "src") return undefined;
  const module = (part: string, index: number) => (index === parts.length - 1 ? part.replace(/\.rs$/, "") : part);
  const at = parts.findIndex((part, index) => index > 0 && TEST_MODULE.test(module(part, index)));
  const name = parts[at];
  if (at === -1 || name === undefined) return undefined;
  const found = parentFiles(crate.folder, parts.slice(1, at)).flatMap((file) => {
    if (changed.has(file)) return [false];
    const text = read(file);
    return text === undefined ? [] : declarations(text, module(name, at));
  });
  return found.length > 0 && found.every(Boolean) ? crate.name : undefined;
}

/**
 * The Rust gates of a run that tests only `crates`: Clippy and nextest of those crates; no doctest
 * (a doctest reads no `cfg(test)` code and no file of `tests/`); `pinning` only for the shell, whose
 * test it is. Each other gate is as it was. Pure.
 */
export function crateGates(gates: readonly Gate[], crates: readonly string[]): Gate[] {
  const flags = crates.flatMap((crate) => ["-p", crate]);
  return gates.flatMap((gate): Gate[] => {
    if (gate.name === "clippy") return [{ ...gate, argv: ["cargo", "clippy", ...flags, "--all-targets", "--", "-D", "warnings"] }];
    if (gate.name === "rust-test") return [{ ...gate, argv: ["bun", "quality/bin/with-postgres.ts", "cargo", "nextest", "run", ...flags, "--no-tests=pass"] }];
    if (gate.name === "doctest") return [];
    if (gate.name === "pinning" && !crates.includes("anima-app")) return [];
    return [gate];
  });
}
