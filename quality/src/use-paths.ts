export type ModulePath = string[];

/** A path named by a `use`, with the name it binds when that is not its last segment. */
export interface UsePath {
  path: ModulePath;
  alias?: string;
}

/** Module path of a hub source file relative to `crates/hub/src/`, for example `devices/store.rs` -> [devices, store]. */
export function modulePathOf(relative: string): ModulePath {
  const segments = relative.replace(/\.rs$/, "").split("/");
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
    .split("::")
    .map((segment) => segment.trim())
    .filter((segment) => segment !== "");
  return segments.length > 1 && segments.at(-1) === "self" ? segments.slice(0, -1) : segments;
}

/** Expand a `use` tree such as `crate::{a::store, b::{x, store}}` into full paths. */
export function expandUseTree(tree: string, prefix: ModulePath = []): UsePath[] {
  const open = tree.indexOf("{");
  if (open === -1) {
    const alias = /\s+as\s+(\w+)\s*$/.exec(tree)?.[1];
    const own = segmentsOf(tree.replace(/\s+as\s+\w+\s*$/, ""));
    const path = own.length === 1 && own[0] === "self" && prefix.length > 0 ? prefix : [...prefix, ...own];
    return [alias === undefined ? { path } : { path, alias }];
  }
  const head = [...prefix, ...segmentsOf(tree.slice(0, open))];
  const inner = tree.slice(open + 1, tree.lastIndexOf("}"));
  return splitTopLevel(inner).flatMap((part) => expandUseTree(part, head));
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

export interface Mentions {
  /** Every `use` declaration. */
  uses: UsePath[];
  /** Every `a::b` path outside `use` declarations. */
  inline: ModulePath[];
}

/** Pull `use` declarations and inline `x::y` paths out of comment-free Rust source. */
export function mentionsIn(source: string): Mentions {
  const uses: UsePath[] = [];
  const rest = source.replace(/\buse\s+([^;]+);/g, (_all, tree: string) => {
    uses.push(...expandUseTree(tree));
    return " ";
  });
  const inline = [...rest.matchAll(/(?<!::\s*)\b[A-Za-z_]\w*(?:\s*::\s*\w+)+/g)].map((match) => segmentsOf(match[0]));
  return { uses, inline };
}

/** Names this file binds with `use`, each mapped to the absolute path it stands for. */
export function bindingsOf(uses: UsePath[], module: ModulePath): Map<string, ModulePath> {
  const bindings = new Map<string, ModulePath>();
  for (const { path, alias } of uses) {
    const absolute = resolvePath(path, module);
    const name = alias ?? path.at(-1);
    if (absolute !== undefined && name !== undefined && name !== "*" && name !== "_") bindings.set(name, absolute);
  }
  return bindings;
}

/** Absolute form of an inline path: through a `use` binding, or `crate`/`super`/`self`. */
export function resolveInline(path: ModulePath, module: ModulePath, bindings: Map<string, ModulePath>): ModulePath | undefined {
  const bound = bindings.get(path[0] ?? "");
  return bound === undefined ? resolvePath(path, module) : [...bound, ...path.slice(1)];
}
