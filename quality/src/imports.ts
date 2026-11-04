import { stripComments } from "./codelines.ts";
import type { SourceFile } from "./limits.ts";
import type { Violation } from "./report.ts";

export const HUB_SOURCE = "crates/hub/src/";
const WIRING = new Set(["main", "schema", "lib"]);

type ModulePath = string[];

/** Module path of a hub source file, for example `devices/store.rs` -> [devices, store]. */
export function modulePathOf(path: string): ModulePath {
  const segments = path.slice(HUB_SOURCE.length).replace(/\.rs$/, "").split("/");
  return segments.at(-1) === "mod" ? segments.slice(0, -1) : segments;
}

function splitTopLevel(text: string): string[] {
  const parts: string[] = [];
  let depth = 0;
  let start = 0;
  for (let i = 0; i < text.length; i++) {
    const char = text.charAt(i);
    if (char === "{") depth++;
    else if (char === "}") depth--;
    else if (char === "," && depth === 0) {
      parts.push(text.slice(start, i));
      start = i + 1;
    }
  }
  parts.push(text.slice(start));
  return parts.filter((part) => part.trim() !== "");
}

function segmentsOf(text: string): string[] {
  const segments = text
    .replace(/\s+as\s+\w+\s*$/, "")
    .split("::")
    .map((segment) => segment.trim())
    .filter((segment) => segment !== "");
  return segments.length > 1 && segments.at(-1) === "self" ? segments.slice(0, -1) : segments;
}

/** Expand a `use` tree such as `crate::{a::store, b::{x, store}}` into full paths. */
export function expandUseTree(tree: string, prefix: ModulePath = []): ModulePath[] {
  const open = tree.indexOf("{");
  if (open === -1) {
    const own = segmentsOf(tree);
    return [own.length === 1 && own[0] === "self" && prefix.length > 0 ? prefix : [...prefix, ...own]];
  }
  const head = [...prefix, ...segmentsOf(tree.slice(0, open))];
  const inner = tree.slice(open + 1, tree.lastIndexOf("}"));
  return splitTopLevel(inner).flatMap((part) => expandUseTree(part, head));
}

/** Every path a source mentions: `use` declarations first, then inline `crate::`/`super::`/`self::` paths. */
export function pathsIn(source: string): ModulePath[] {
  const paths: ModulePath[] = [];
  const rest = source.replace(/\buse\s+([^;]+);/g, (_all, tree: string) => {
    paths.push(...expandUseTree(tree));
    return " ";
  });
  for (const match of rest.matchAll(/\b(?:crate|super|self)(?:\s*::\s*\w+)+/g)) {
    paths.push(segmentsOf(match[0]));
  }
  return paths;
}

/** Make a path absolute from the crate root, using the file's own module path for `super`/`self`. */
export function resolvePath(path: ModulePath, module: ModulePath): ModulePath | undefined {
  const [first, ...rest] = path;
  if (first === "crate") return rest;
  if (first === "self") return [...module, ...rest];
  if (first !== "super") return undefined;
  let base = module.slice(0, -1);
  let remaining = rest;
  while (remaining[0] === "super") {
    base = base.slice(0, -1);
    remaining = remaining.slice(1);
  }
  return [...base, ...remaining];
}

/** Top-level domain names: each `crates/hub/src/<domain>/` folder and `<domain>.rs`, minus wiring. */
export function hubDomains(files: SourceFile[]): Set<string> {
  const domains = new Set<string>();
  for (const { path } of files) {
    if (!path.startsWith(HUB_SOURCE) || !path.endsWith(".rs")) continue;
    const [top] = modulePathOf(path);
    if (top !== undefined && !WIRING.has(top)) domains.add(top);
  }
  return domains;
}

interface Reference {
  from: string;
  to: string;
  path: string;
  isStore: boolean;
  target: string;
}

function referencesFrom(file: SourceFile, domains: Set<string>): Reference[] {
  const module = modulePathOf(file.path);
  const from = module[0];
  if (from === undefined || !domains.has(from)) return [];
  const source = stripComments(file.text, ".rs");
  return pathsIn(source).flatMap((raw) => {
    const [to, member] = resolvePath(raw, module) ?? [];
    if (to === undefined || to === from || !domains.has(to)) return [];
    return [{ from, to, path: file.path, isStore: member === "store", target: `crate::${to}::${member ?? ""}` }];
  });
}

function findCycle(graph: Map<string, Set<string>>): string[] | undefined {
  const done = new Set<string>();
  const visit = (node: string, stack: string[]): string[] | undefined => {
    const at = stack.indexOf(node);
    if (at !== -1) return [...stack.slice(at), node];
    if (done.has(node)) return undefined;
    for (const next of [...(graph.get(node) ?? [])].sort()) {
      const cycle = visit(next, [...stack, node]);
      if (cycle) return cycle;
    }
    done.add(node);
    return undefined;
  };
  for (const start of [...graph.keys()].sort()) {
    const cycle = visit(start, []);
    if (cycle) return cycle;
  }
  return undefined;
}

/** REPO-004: no domain uses another domain's `store`; the domain graph has no cycle. */
export function checkImports(files: SourceFile[]): Violation[] {
  const domains = hubDomains(files);
  const references = files.flatMap((file) => referencesFrom(file, domains));
  const violations: Violation[] = references
    .filter((reference) => reference.isStore)
    .map((reference) => ({ path: reference.path, signal: "foreign-store", detail: `${reference.target}` }));
  const graph = new Map<string, Set<string>>();
  for (const { from, to } of references) {
    graph.set(from, (graph.get(from) ?? new Set<string>()).add(to));
  }
  const cycle = findCycle(graph);
  if (cycle) violations.push({ path: `${HUB_SOURCE}${cycle[0]}`, signal: "import-cycle", detail: cycle.join(" -> ") });
  return violations;
}
