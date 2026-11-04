/**
 * Reads HuJSON, the JSON-with-comments format of Headscale's policy file: `//` and
 * `/* *\/` comments and trailing commas are removed outside strings, then the text is
 * read as plain JSON. Pure; malformed input throws `HujsonError`.
 */

export class HujsonError extends Error {
  constructor(message: string, options?: ErrorOptions) {
    super(message, options);
    this.name = 'HujsonError';
  }
}

const BYTE_ORDER_MARK = '﻿';

export function parseHujson(text: string): unknown {
  const json = standardize(text.startsWith(BYTE_ORDER_MARK) ? text.slice(1) : text);
  try {
    return JSON.parse(json) as unknown;
  } catch (error) {
    throw new HujsonError(`not valid HuJSON: ${(error as Error).message}`, { cause: error });
  }
}

function standardize(text: string): string {
  const out: string[] = [];
  // Index in `out` of a comma that follows a value and is not yet followed by another;
  // dropped if `}` or `]` comes next. A comma after `{`, `[`, `,` or `:` is left for
  // JSON.parse to refuse.
  let comma = -1;
  let previous = '';
  let at = 0;
  while (at < text.length) {
    const char = text.charAt(at);
    const next = text.charAt(at + 1);
    if (char === '"') {
      const end = stringEnd(text, at);
      out.push(text.slice(at, end));
      comma = -1;
      previous = char;
      at = end;
    } else if (char === '/' && (next === '/' || next === '*')) {
      out.push(' ');
      at = next === '/' ? lineEnd(text, at) : blockEnd(text, at);
    } else {
      if ((char === '}' || char === ']') && comma !== -1) out[comma] = ' ';
      if (!/\s/.test(char)) {
        comma = char === ',' && !'{[,:'.includes(previous) ? out.length : -1;
        previous = char;
      }
      out.push(char);
      at += 1;
    }
  }
  return out.join('');
}

function stringEnd(text: string, start: number): number {
  for (let at = start + 1; at < text.length; at += 1) {
    const char = text.charAt(at);
    if (char === '\\') at += 1;
    else if (char === '"') return at + 1;
  }
  throw new HujsonError(`unterminated string at offset ${String(start)}`);
}

function lineEnd(text: string, start: number): number {
  const end = text.indexOf('\n', start);
  return end === -1 ? text.length : end;
}

function blockEnd(text: string, start: number): number {
  const end = text.indexOf('*/', start + 2);
  if (end === -1) throw new HujsonError(`unterminated comment at offset ${String(start)}`);
  return end + 2;
}
