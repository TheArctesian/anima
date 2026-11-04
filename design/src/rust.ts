import { GENERATED_NOTICE, aliasName, isTerminal, type Token } from './model.ts';

const NORD_COUNT = 16;

function rgb(hex: unknown): string {
  const match = /^#([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i.exec(String(hex));
  if (!match) throw new TypeError(`expected a #RRGGBB colour, got ${JSON.stringify(hex)}`);
  return `Rgb::new(${match.slice(1).map((pair) => `0x${pair.toUpperCase()}`).join(', ')})`;
}

/** `NORD[n]` when the token aliases a Nord primitive, else the colour itself. */
function terminalColour(token: Token): string {
  const index = /^nord-(\d+)$/.exec(aliasName(token.source) ?? '')?.[1];
  return index === undefined ? rgb(token.value) : `NORD[${index}]`;
}

function find(tokens: readonly Token[], name: string): Token {
  const token = tokens.find((candidate) => candidate.name === name);
  if (!token) throw new Error(`missing token ${name}`);
  return token;
}

const LINE_WIDTH = 100;
const SHORT_ITEM_WIDTH = 10;

/** Items of at most 10 characters share lines; longer items take one line each (rustfmt's array layout). */
function arrayLines(items: readonly string[]): string[] {
  if (items.some((item) => item.length > SHORT_ITEM_WIDTH)) return items.map((item) => `    ${item},`);
  const lines: string[] = [];
  for (const item of items) {
    const last = lines.at(-1);
    if (last !== undefined && last.length + 1 + item.length + 1 <= LINE_WIDTH) lines[lines.length - 1] = `${last} ${item},`;
    else lines.push(`    ${item},`);
  }
  return lines;
}

function array(name: string, items: readonly string[]): string {
  return `pub const ${name}: [Rgb; ${items.length}] = [\n${arrayLines(items).join('\n')}\n];`;
}

/** generated.rs: the Nord palette and the terminal palette for crates/theme (RA-010, TOK-001). */
export function renderRust(tokens: readonly Token[]): string {
  const nord = Array.from({ length: NORD_COUNT }, (_, index) => rgb(find(tokens, `nord-${index}`).value));
  const terminal = tokens.filter(isTerminal);
  const ansi = Array.from({ length: NORD_COUNT }, (_, index) => terminalColour(find(terminal, `ansi-${index}`)));
  return [
    `// ${GENERATED_NOTICE}`,
    'use crate::Rgb;',
    array('NORD', nord),
    `pub const TERMINAL_BACKGROUND: Rgb = ${terminalColour(find(terminal, 'background'))};`,
    `pub const TERMINAL_FOREGROUND: Rgb = ${terminalColour(find(terminal, 'foreground'))};`,
    array('TERMINAL_ANSI', ansi),
  ].join('\n\n') + '\n';
}
