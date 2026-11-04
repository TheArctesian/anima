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

/** One item per line; `#[rustfmt::skip]` keeps rustfmt from repacking short items. */
function array(name: string, items: readonly string[]): string {
  const lines = items.map((item) => `    ${item},\n`).join('');
  return `#[rustfmt::skip]\npub const ${name}: [Rgb; ${items.length}] = [\n${lines}];`;
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
