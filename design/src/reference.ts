import { declarations } from './css.ts';
import { GENERATED_NOTICE, aliasName, type Token } from './model.ts';
import { EXPORTS, keyOf } from './typescript.ts';

const SPRING_PREVIEW = 80;

function cell(text: string): string {
  return text.replaceAll('|', '\\|');
}

function shown(value: string): string {
  return value.length > SPRING_PREVIEW ? `${value.slice(0, SPRING_PREVIEW)}…` : value;
}

function valueCell(token: Token): string {
  const lines = declarations(token).map(([property, value]) => `\`${property}: ${shown(value)}\``);
  if (token.type === 'breakpoint') return `\`${JSON.stringify(token.value)}\` (TypeScript only)`;
  if (token.type === 'gesture') return `\`${String(token.value)}\` (TypeScript only)`;
  const resolved = token.type === 'color' && aliasName(token.source) ? ` = \`${String(token.value)}\`` : '';
  return `${lines.join('<br>')}${resolved}${lightLine(token)}`;
}

/** The light value on a second line, so the page shows both themes (UI-005); a mix shows as its `color-mix`, as a tint does. */
function lightLine(token: Token): string {
  if (token.light === undefined) return '';
  const [declaration] = declarations({ ...token, ...token.light });
  const resolved = typeof token.light.value === 'string' ? ` = \`${token.light.value}\`` : '';
  return declaration ? `<br>light: \`${declaration[0]}: ${shown(declaration[1])}\`${resolved}` : '';
}

function table(tokens: readonly Token[]): string {
  const rows = tokens.map((token) => `| \`${token.name}\` | ${cell(valueCell(token))} | ${cell(token.description)} |`);
  return ['| Token | CSS value | Use |', '| --- | --- | --- |', ...rows].join('\n');
}

function groupsOf(tokens: readonly Token[]): Map<string, Token[]> {
  const groups = new Map<string, Token[]>();
  for (const token of tokens) groups.set(token.group, [...(groups.get(token.group) ?? []), token]);
  return groups;
}

function typescriptMap(tokens: readonly Token[]): string {
  const rows = EXPORTS.flatMap((spec) =>
    tokens
      .filter((token) => token.group === spec.group)
      .map((token) => `| \`${token.name}\` | \`${spec.export}.${keyOf(token.name, spec.prefix)}\` |`),
  );
  return ['| K5 name | TypeScript (`lib/motion/tokens.ts`) |', '| --- | --- |', ...rows].join('\n');
}

const NOTES = [
  '- Component code uses semantic tokens only; `nord-*` are primitives for the source and for themes (TOK-010).',
  '- `elevation-1` is a 1 px border and no shadow (5.6), so its value is `none`; the component draws the border.',
  '- `target-min-touch` is 44 pt on iOS and 44 CSS px on the web.',
  '- Spring tokens are `<settle time> linear(<curve>)` fragments for CSS transitions; scripts read stiffness and damping from `tokens.ts` through `$lib/motion/` (MOT-001).',
  '- `threshold-*` never change under reduced motion (MOT-021).',
];

/** reference.md: one page that lists every token with its value and use (5.5.1 documentation output). */
export function renderReference(tokens: readonly Token[]): string {
  const sections = [...groupsOf(tokens)].map(([group, members]) => `## ${group}\n\n${table(members)}`);
  return [
    `<!-- ${GENERATED_NOTICE} -->`,
    '# Design tokens',
    'Generated from `design/tokens/*.tokens.json` (TOK-001). Names follow contract K5.',
    NOTES.join('\n'),
    ...sections,
    `## TypeScript names\n\n${typescriptMap(tokens)}`,
    '',
  ].join('\n\n');
}
