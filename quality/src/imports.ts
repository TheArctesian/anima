import { stripComments } from "./lexing.ts";
import { findCycle, isListed, ungrouped, type HubModules } from "./hub-modules.ts";
import type { Violation } from "./report.ts";
import type { SourceFile } from "./source.ts";
import { bindingsOf, mentionsIn, modulePathOf, resolveInline, type ModulePath } from "./use-paths.ts";

export const HUB_SOURCE = "crates/hub/src/";
export const HUB_MODULES_FILE = "quality/hub-modules.json";

interface Reference {
  from: string;
  /** The folder of `from`, such as `crates/hub/src/fleet/devices`. */
  home: string;
  to: string;
  path: string;
  isStore: boolean;
  isGlob: boolean;
  /** The file is the domain's `graphql` module (R-011). */
  isGraphql: boolean;
  target: string;
}

function hubModulePath(file: SourceFile): ModulePath | undefined {
  if (!file.path.startsWith(HUB_SOURCE) || !file.path.endsWith(".rs")) return undefined;
  return modulePathOf(file.path.slice(HUB_SOURCE.length));
}

/** The path of the top-level module `own` names inside `module` (`own` is `module` without its group). */
function topOf(module: ModulePath, own: ModulePath): ModulePath {
  return module.slice(0, module.length - own.length + 1);
}

/** Domains a file refers to (`from` is its own top-level module), by `use` and inline paths, through aliases. */
function referencesFrom(file: SourceFile, module: ModulePath, modules: HubModules): Reference[] {
  const own = ungrouped(modules, module);
  const from = own[0];
  if (from === undefined) return [];
  const home = `${HUB_SOURCE}${topOf(module, own).join("/")}`;
  const { uses, inline } = mentionsIn(stripComments(file.text, ".rs", true));
  const bindings = bindingsOf(uses, module);
  const resolved = [...uses.map((use) => use.path), ...inline].map((path) => resolveInline(path, module, bindings));
  return resolved.flatMap((absolute) => {
    if (absolute === undefined) return [];
    const named = ungrouped(modules, absolute);
    const [to, member] = named;
    if (to === undefined || to === from || !Object.hasOwn(modules.domains, to)) return [];
    const isGlob = absolute.at(-1) === "*";
    const isGraphql = own[1] === "graphql";
    const target = `crate::${[...topOf(absolute, named), member ?? ""].join("::")}`;
    return [{ from, home, to, path: file.path, isStore: member === "store", isGlob, isGraphql, target }];
  });
}

function unlistedModules(files: SourceFile[], modules: HubModules): Violation[] {
  const tops = new Map<string, string>();
  for (const file of files) {
    const module = hubModulePath(file);
    if (module === undefined) continue;
    const own = ungrouped(modules, module);
    if (own[0] !== undefined) tops.set(own[0], `${HUB_SOURCE}${topOf(module, own).join("/")}`);
  }
  return [...tops.keys()]
    .filter((top) => !isListed(modules, top))
    .sort()
    .map((top) => ({ path: tops.get(top) ?? HUB_SOURCE, signal: "unlisted-module", detail: `"${top}" is not in hub-modules.json` }));
}

function edgeViolations(reference: Reference, modules: HubModules): Violation[] {
  const { from, to, path } = reference;
  const violations: Violation[] = [];
  if (modules.leaves.includes(from)) {
    violations.push({ path, signal: "leaf-imports-domain", detail: `${from} -> ${to}` });
  } else if (Object.hasOwn(modules.domains, from) && !isAllowed(reference, modules)) {
    violations.push({ path, signal: "forbidden-edge", detail: `${from} -> ${to}` });
  }
  if (reference.isStore) violations.push({ path, signal: "foreign-store", detail: reference.target });
  if (reference.isGlob) violations.push({ path, signal: "domain-glob-import", detail: `${from} -> ${to}::*` });
  return violations;
}

function isAllowed({ from, to, isGraphql }: Reference, modules: HubModules): boolean {
  return (modules.domains[from]?.includes(to) ?? false) || (isGraphql && (modules.graphql[from]?.includes(to) ?? false));
}

/** A reference that only the graphql allow-list permits; it does not count toward cycles (R-011). */
function isGraphqlOnly(reference: Reference, modules: HubModules): boolean {
  return reference.isGraphql && !(modules.domains[reference.from]?.includes(reference.to) ?? false);
}

function graphOf(edges: [string, string][]): Map<string, Set<string>> {
  const graph = new Map<string, Set<string>>();
  for (const [from, to] of edges) graph.set(from, (graph.get(from) ?? new Set<string>()).add(to));
  return graph;
}

function cycleViolation(signal: string, path: string, edges: [string, string][]): Violation[] {
  const cycle = findCycle(graphOf(edges));
  return cycle ? [{ path, signal, detail: cycle.join(" -> ") }] : [];
}

/** REPO-004: listed modules only, allowed edges only, no foreign `store`, no cycle (also in the allow-list). */
export function checkImports(files: SourceFile[], modules: HubModules): Violation[] {
  const references = files.flatMap((file) => {
    const module = hubModulePath(file);
    return module && isListed(modules, ungrouped(modules, module)[0] ?? "") ? referencesFrom(file, module, modules) : [];
  });
  const allowed = Object.entries(modules.domains).flatMap(([from, tos]) => tos.map((to): [string, string] => [from, to]));
  const used = references.filter(
    (reference) => Object.hasOwn(modules.domains, reference.from) && !isGraphqlOnly(reference, modules),
  );
  const all = [
    ...unlistedModules(files, modules),
    ...references.flatMap((reference) => edgeViolations(reference, modules)),
    ...cycleViolation("allow-list-cycle", HUB_MODULES_FILE, allowed),
    ...cycleViolation(
      "import-cycle",
      used[0]?.home ?? HUB_SOURCE,
      used.map((reference): [string, string] => [reference.from, reference.to]),
    ),
  ];
  const seen = new Set<string>();
  return all.filter((v) => {
    const key = `${v.path}\0${v.signal}\0${v.detail}`;
    return !seen.has(key) && seen.add(key);
  });
}
