import { extensionOf, type SourceFile } from "./limits.ts";
import type { Violation } from "./report.ts";

const BANNED_WORDS = new Set([
  "manager", "service", "helper", "handler", "processor", "factory",
  "util", "data", "info", "item", "misc", "common",
]);
const BANNED_FOLDERS = new Set(["utils", "helpers", "common", "components", "services", "models"]);
const SKIPPED_PREFIXES = ["docs/", "node_modules/"];
const TYPE_EXTENSIONS = [".rs", ".ts", ".js", ".svelte"];

const RUST_TYPE = /^\s*(?:pub(?:\([^)]*\))?\s+)?(?:struct|enum|trait|type|union)\s+([A-Za-z_]\w*)/gm;
const TS_TYPE =
  /^\s*(?:export\s+)?(?:default\s+)?(?:declare\s+)?(?:abstract\s+)?(?:const\s+)?(?:class|interface|type|enum)\s+([A-Za-z_$][\w$]*)/gm;

/** Split a name into lowercase word segments: camelCase, PascalCase, snake_case, kebab-case, dots. */
export function wordSegments(name: string): string[] {
  return name
    .replace(/([a-z0-9])([A-Z])/g, "$1 $2")
    .replace(/([A-Z]+)([A-Z][a-z])/g, "$1 $2")
    .toLowerCase()
    .split(/[^a-z0-9]+/)
    .filter((segment) => segment !== "");
}

/** A segment is banned when it equals a banned word or that word plus `s`. */
export function bannedWordIn(name: string): string | undefined {
  return wordSegments(name).find(
    (segment) => BANNED_WORDS.has(segment) || (segment.endsWith("s") && BANNED_WORDS.has(segment.slice(0, -1))),
  );
}

function pathViolations(path: string): Violation[] {
  const parts = path.split("/");
  const violations: Violation[] = [];
  parts.forEach((part, index) => {
    const isFile = index === parts.length - 1;
    if (!isFile && BANNED_FOLDERS.has(part.toLowerCase())) {
      violations.push({ path, signal: "banned-folder", detail: `"${part}"` });
      return;
    }
    const word = bannedWordIn(part);
    if (word !== undefined) {
      violations.push({ path, signal: isFile ? "banned-file-name" : "banned-folder-name", detail: `"${word}" in "${part}"` });
    }
  });
  return violations;
}

function declaredTypes(file: SourceFile): string[] {
  const extension = extensionOf(file.path);
  if (!TYPE_EXTENSIONS.includes(extension)) return [];
  const pattern = extension === ".rs" ? RUST_TYPE : TS_TYPE;
  return [...file.text.matchAll(pattern)].map((match) => match[1] ?? "");
}

function typeViolations(file: SourceFile): Violation[] {
  return declaredTypes(file).flatMap((name) => {
    const word = bannedWordIn(name);
    return word === undefined ? [] : [{ path: file.path, signal: "banned-type-name", detail: `"${word}" in "${name}"` }];
  });
}

/** CQ-030: banned words in file names, folder names and declared type names. */
export function checkNames(files: SourceFile[]): Violation[] {
  return files
    .filter((file) => !SKIPPED_PREFIXES.some((prefix) => file.path.startsWith(prefix)))
    .flatMap((file) => [...pathViolations(file.path), ...typeViolations(file)]);
}
