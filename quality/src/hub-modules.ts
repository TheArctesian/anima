import { isRecord } from "./record.ts";

/** The hub's top-level modules and which domains each domain may import (the hub-domains contract, REPO-004). */
export interface HubModules {
  /** May be imported by everyone; import no domain. */
  leaves: string[];
  /** May import anything (`main`, `lib`, `schema`). */
  wiring: string[];
  /** Domain -> domains it may import. */
  domains: Record<string, string[]>;
  /**
   * Domain -> extra domains its `graphql` module only may import, to resolve K1 fields that join
   * domains (R-011). These edges are left out of the cycle checks.
   */
  graphql: Record<string, string[]>;
}

export class HubModulesShapeError extends Error {
  constructor(message: string) {
    super(`hub-modules.json: ${message}`);
    this.name = "HubModulesShapeError";
  }
}

function stringList(value: unknown, what: string): string[] {
  if (!Array.isArray(value) || !value.every((item): item is string => typeof item === "string")) {
    throw new HubModulesShapeError(`${what} must be a list of strings`);
  }
  return value;
}

function edgeMap(value: unknown, what: string): Record<string, string[]> {
  if (!isRecord(value)) {
    throw new HubModulesShapeError(`${what} must map names to lists`);
  }
  const map: Record<string, string[]> = {};
  for (const [name, allowed] of Object.entries(value)) map[name] = stringList(allowed, `${what}.${name}`);
  return map;
}

export function parseHubModules(text: string): HubModules {
  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch {
    throw new HubModulesShapeError("not valid JSON");
  }
  if (!isRecord(parsed) || !isRecord(parsed["domains"])) {
    throw new HubModulesShapeError("needs leaves, wiring and domains");
  }
  const domains = edgeMap(parsed["domains"], "domains");
  const graphql = parsed["graphql"] === undefined ? {} : edgeMap(parsed["graphql"], "graphql");
  for (const name of Object.keys(graphql)) {
    if (!Object.hasOwn(domains, name)) throw new HubModulesShapeError(`graphql.${name} is not a domain`);
  }
  return { leaves: stringList(parsed["leaves"], "leaves"), wiring: stringList(parsed["wiring"], "wiring"), domains, graphql };
}

export function isListed(modules: HubModules, name: string): boolean {
  return modules.leaves.includes(name) || modules.wiring.includes(name) || Object.hasOwn(modules.domains, name);
}

/** The first cycle in a graph of node -> neighbours, as `a -> b -> a`, or undefined. */
export function findCycle(graph: Map<string, Set<string>>): string[] | undefined {
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
