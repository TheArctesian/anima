// Resolves a colour role of the generated token CSS to hex, for one appearance, and computes WCAG contrast (UI-002, SITE-009).

import { readFileSync } from "node:fs";
import { join } from "node:path";

type Scope = ReadonlyMap<string, string>;
export type Appearance = "dark" | "light";

const TOKENS = readFileSync(join(import.meta.dir, "../../client/src/styles/tokens.css"), "utf8");

function declarations(block: string): Map<string, string> {
  const found = new Map<string, string>();
  for (const [, name, value] of block.matchAll(/(--[\w-]+):\s*([^;]+);/g)) {
    if (name !== undefined && value !== undefined) found.set(name, value.trim());
  }
  return found;
}

/** The body of the first rule whose selector is `selector`, inside `text`. */
function ruleBody(text: string, selector: string): string {
  const start = text.indexOf(`${selector} {`);
  if (start < 0) throw new Error(`No rule ${selector} in tokens.css.`);
  return text.slice(start, text.indexOf("\n}", start));
}

/** The Graphite palette (the default, UI-023) for an appearance: primitives, dark roles, then the light roles of `system`. */
export function scopeOf(appearance: Appearance): Scope {
  const scope = new Map(declarations(ruleBody(TOKENS, ":root")));
  for (const [name, value] of declarations(ruleBody(TOKENS, ':root, :root[data-palette="graphite"]'))) scope.set(name, value);
  if (appearance === "light") {
    const media = TOKENS.slice(TOKENS.indexOf("@media (prefers-color-scheme: light)"));
    for (const [name, value] of declarations(ruleBody(media, ':root[data-theme="system"]'))) scope.set(name, value);
  }
  return scope;
}

function channels(hex: string): [number, number, number] {
  const value = Number.parseInt(hex.slice(1), 16);
  return [(value >> 16) & 255, (value >> 8) & 255, value & 255];
}

/** The colour of a value, as channels: a hex, `var(--x)` or `color-mix(in srgb, A p%, B)`. */
function resolve(value: string, scope: Scope): [number, number, number] {
  const text = value.trim();
  const reference = /^var\((--[\w-]+)\)$/.exec(text);
  if (reference?.[1] !== undefined) {
    const next = scope.get(reference[1]);
    if (next === undefined) throw new Error(`${reference[1]} is not defined.`);
    return resolve(next, scope);
  }
  const mix = /^color-mix\(in srgb, (.+) (\d+(?:\.\d+)?)%, (.+)\)$/.exec(text);
  if (mix?.[1] !== undefined && mix[2] !== undefined && mix[3] !== undefined) {
    const share = Number(mix[2]) / 100;
    const from = resolve(mix[1], scope);
    const to = resolve(mix[3], scope);
    return [0, 1, 2].map((index) => (from[index] ?? 0) * share + (to[index] ?? 0) * (1 - share)) as [number, number, number];
  }
  if (/^#[0-9a-f]{6}$/i.test(text)) return channels(text);
  throw new Error(`Cannot resolve ${text}.`);
}

function luminance([red, green, blue]: [number, number, number]): number {
  const linear = [red, green, blue].map((channel) => {
    const unit = channel / 255;
    return unit <= 0.04045 ? unit / 12.92 : ((unit + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * (linear[0] ?? 0) + 0.7152 * (linear[1] ?? 0) + 0.0722 * (linear[2] ?? 0);
}

/** The WCAG contrast ratio of two roles (for example `text` and `surface-base`) in an appearance. */
export function contrastOf(foreground: string, background: string, appearance: Appearance): number {
  const scope = scopeOf(appearance);
  const first = luminance(resolve(`var(--${foreground})`, scope));
  const second = luminance(resolve(`var(--${background})`, scope));
  return (Math.max(first, second) + 0.05) / (Math.min(first, second) + 0.05);
}
