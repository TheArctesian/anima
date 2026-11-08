import { createHash } from 'node:crypto';

// The CSP hashes of the inline scripts in `index.html`, read exactly as the hub reads them (WP-43 A-2,
// `crates/hub/src/web_client/scripts.rs`): the same scan, the same edge cases, so a page the hub would block is blocked here.

const OPEN = '<script';
const CLOSE = '</script';

/** Rust's `is_ascii_whitespace`: space, tab, line feed, form feed, carriage return (not vertical tab). */
function isAsciiWhitespace(char: string | undefined): boolean {
  return char === ' ' || char === '\t' || char === '\n' || char === '\f' || char === '\r';
}

/** ASCII-only lowercasing: every offset in the result is the same offset in `text` (`toLowerCase` can change lengths). */
function asciiLower(text: string): string {
  return text.replace(/[A-Z]/g, (char) => char.toLowerCase());
}

/** `'sha256-<base64>'` of `script`'s UTF-8 bytes, as a browser hashes an inline script. */
export function sourceHash(script: string): string {
  return `'sha256-${createHash('sha256').update(script, 'utf8').digest('base64')}'`;
}

/** The offset of the `>` that ends a start tag whose attributes begin `tag`; a `>` inside a quoted value does not end it. */
function tagEnd(tag: string): number | undefined {
  let quote: string | undefined;
  for (let offset = 0; offset < tag.length; offset += 1) {
    const char = tag[offset];
    if (quote !== undefined) {
      if (char === quote) quote = undefined;
    } else if (char === '"' || char === "'") {
      quote = char;
    } else if (char === '>') {
      return offset;
    }
  }
  return undefined;
}

/** The first offset at or after `start` where `skip` is false, or the end of `text`. */
function skipWhile(text: string, start: number, skip: (char: string | undefined) => boolean): number {
  let offset = start;
  while (offset < text.length && skip(text[offset])) offset += 1;
  return offset;
}

/** The text after one attribute value at the start of `value`: quoted, or up to whitespace. */
function afterValue(value: string): string {
  const quote = value[0];
  if (quote === '"' || quote === "'") {
    const close = value.indexOf(quote, 1);
    return close === -1 ? '' : value.slice(close + 1);
  }
  return value.slice(skipWhile(value, 0, (char) => !isAsciiWhitespace(char)));
}

/** The names of the attributes in a start tag's attribute text, values skipped. */
function attributeNames(attributes: string): string[] {
  const names: string[] = [];
  let rest = attributes;
  for (;;) {
    rest = rest.slice(skipWhile(rest, 0, (char) => isAsciiWhitespace(char) || char === '/'));
    if (rest === '') return names;
    const end = skipWhile(rest, 0, (char) => !(isAsciiWhitespace(char) || char === '=' || char === '/'));
    names.push(rest.slice(0, end));
    rest = rest.slice(skipWhile(rest, end, isAsciiWhitespace));
    if (rest.startsWith('=')) rest = afterValue(rest.slice(skipWhile(rest, 1, isAsciiWhitespace)));
  }
}

/**
 * The body of each inline script in `html`, in page order. A script with a `src` attribute is not inline and is left out;
 * an unclosed script ends the search.
 */
export function inlineScripts(html: string): string[] {
  const lower = asciiLower(html);
  const bodies: string[] = [];
  let from = 0;
  for (;;) {
    const found = lower.indexOf(OPEN, from);
    if (found === -1) return bodies;
    const tag = found + OPEN.length;
    const next = lower[tag];
    if (next === undefined) return bodies;
    if (!(isAsciiWhitespace(next) || next === '>' || next === '/')) {
      from = tag; // `<scripts`, `<script-x`: another element.
      continue;
    }
    const tagLength = tagEnd(lower.slice(tag));
    if (tagLength === undefined) return bodies;
    const body = tag + tagLength + 1;
    const close = lower.indexOf(CLOSE, body);
    if (close === -1) return bodies;
    if (!attributeNames(lower.slice(tag, tag + tagLength)).includes('src')) bodies.push(html.slice(body, close));
    from = close + CLOSE.length;
  }
}

/** The CSP source `'sha256-<base64>'` of each inline script in `html`, in page order: what the hub puts in `script-src`. */
export function inlineScriptHashes(html: string): string[] {
  return inlineScripts(html).map(sourceHash);
}
