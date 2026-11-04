import { stripComments } from "./codelines.ts";
import { findCycle, isListed, type HubModules } from "./hub-modules.ts";
import type { Violation } from "./report.ts";
import type { SourceFile } from "./source.ts";
import { bindingsOf, mentionsIn, modulePathOf, resolveInline, type ModulePath } from "./use-paths.ts";

export const HUB_SOURCE = "crates/hub/src/";
export const HUB_MODULES_FILE = "quality/hub-modules.json";

interface Reference {
  from: string;
  to: string;
  path: string;
  isStore: boolean;
  target: string;
}

function hubModulePath(file: SourceFile): ModulePath | undefined {
  if (!file.path.startsWith(HUB_SOURCE) || !file.path.endsWith(".rs")) return undefined;
  return modulePathOf(file.path.slice(HUB_SOURCE.length));
}

/** Domains a file refers to (`from` is its own top-level module), by `use` and inline paths, through aliases. */
function referencesFrom(file: SourceFile, module: ModulePath, modules: HubModules): Reference[] {
  const from = module[0];
  if (from === undefined) return [];
  const { uses, inline } = mentionsIn(stripComments(file.text, ".rs"));
  const bindings = bindingsOf(uses, module);
  const resolved = [...uses.map((use) => use.path), ...inline].map((path) => resolveInline(path, module, bindings));
  return resolved.flatMap((absolute) => {
    const [to, member] = absolute ?? [];
    if (to === undefined || to === from || !Object.hasOwn(modules.domains, to)) return [];
    return [{ from, to, path: file.path, isStore: member === "store", target: `crate::${to}::${member ?? ""}` }];
  });
}

function unlistedModules(files: SourceFile[], modules: HubModules): Violation[] {
  const tops = new Set(files.flatMap((file) => hubModulePath(file)?.slice(0, 1) ?? []));
  return [...tops]
    .filter((top) => !isListed(modules, top))
    .sort()
    .map((top) => ({ path: `${HUB_SOURCE}${top}`, signal: "unlisted-module", detail: `"${top}" is not in hub-modules.json` }));
}

function edgeViolations(reference: Reference, modules: HubModules): Violation[] {
  const { from, to, path } = reference;
  const violations: Violation[] = [];
  if (modules.leaves.includes(from)) {
    violations.push({ path, signal: "leaf-imports-domain", detail: `${from} -> ${to}` });
  } else if (Object.hasOwn(modules.domains, from) && !modules.domains[from]?.includes(to)) {
    violations.push({ path, signal: "forbidden-edge", detail: `${from} -> ${to}` });
  }
  if (reference.isStore && Object.hasOwn(modules.domains, from)) violations.push({ path, signal: "foreign-store", detail: reference.target });
  return violations;
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
    return module && isListed(modules, module[0] ?? "") ? referencesFrom(file, module, modules) : [];
  });
  const allowed = Object.entries(modules.domains).flatMap(([from, tos]) => tos.map((to): [string, string] => [from, to]));
  const used = references.filter((reference) => Object.hasOwn(modules.domains, reference.from));
  const all = [
    ...unlistedModules(files, modules),
    ...references.flatMap((reference) => edgeViolations(reference, modules)),
    ...cycleViolation("allow-list-cycle", HUB_MODULES_FILE, allowed),
    ...cycleViolation(
      "import-cycle",
      `${HUB_SOURCE}${used[0]?.from ?? ""}`,
      used.map((reference): [string, string] => [reference.from, reference.to]),
    ),
  ];
  const seen = new Set<string>();
  return all.filter((v) => {
    const key = `${v.path}\0${v.signal}\0${v.detail}`;
    return !seen.has(key) && seen.add(key);
  });
}
