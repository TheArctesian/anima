import { GENERATED_NOTICE, aliasName, aliasOrValue, bezierOf, fieldsOf, numberOf, springOf, trimmedDecimal, tokensOfType, type Token } from './model.ts';
import { sampleCurve, settleMs } from './spring.ts';

type Declaration = readonly [property: string, value: string];

const SOLID_BLUR = '0px';
const SOLID_SATURATE = '100%';

function fontFamily(names: unknown): string {
  return (names as readonly string[]).map((name) => (/\s/.test(name) ? `"${name}"` : name)).join(', ');
}

function colourMix(colour: string, percent: number, over: string): string {
  return `color-mix(in srgb, ${colour} ${trimmedDecimal(percent)}%, ${over})`;
}

function tint(token: Token): string {
  const parts = fieldsOf(token.value);
  const source = fieldsOf(token.source);
  return colourMix(aliasOrValue(source['color'], parts['color']), numberOf(parts['percent']), aliasOrValue(source['over'], parts['over']));
}

function springFragment(token: Token): string {
  const spring = springOf(token);
  return `${settleMs(spring)}ms linear(${sampleCurve(spring).map(String).join(', ')})`;
}

function material(token: Token): Declaration[] {
  const parts = fieldsOf(token.value);
  const base = aliasOrValue(fieldsOf(token.source)['base'], parts['base']);
  const prefix = `--${token.name}`;
  return [
    [`${prefix}-bg`, colourMix(base, numberOf(parts['opacityPercent']), 'transparent')],
    [`${prefix}-blur`, String(parts['blur'])],
    [`${prefix}-saturate`, `${trimmedDecimal(numberOf(parts['saturatePercent']))}%`],
  ];
}

function elevation(token: Token): string {
  if (typeof token.value === 'string') return token.value;
  const parts = fieldsOf(token.value);
  const colour = aliasOrValue(fieldsOf(token.source)['color'], parts['color']);
  return `0 ${String(parts['offsetY'])} ${String(parts['blur'])} ${colourMix(colour, numberOf(parts['alphaPercent']), 'transparent')}`;
}

function valueOf(token: Token): string {
  switch (token.type) {
    case 'color':
      return aliasOrValue(token.source, token.value);
    case 'tint':
      return tint(token);
    case 'fontFamily':
      return fontFamily(token.value);
    case 'cubicBezier':
      return `cubic-bezier(${bezierOf(token).map((point) => trimmedDecimal(point)).join(', ')})`;
    case 'spring':
      return springFragment(token);
    case 'stagger':
      return `${trimmedDecimal(numberOf(fieldsOf(token.value)['intervalMs']))}ms`;
    case 'elevation':
      return elevation(token);
    default:
      return String(token.value);
  }
}

/** The custom properties a token declares. Breakpoints are TypeScript only (K5). */
export function declarations(token: Token): Declaration[] {
  if (token.type === 'breakpoint') return [];
  if (token.type === 'material') return material(token);
  return [[`--${token.name}`, valueOf(token)]];
}

function block(selector: string, lines: readonly Declaration[], indent = ''): string {
  const body = lines.map(([property, value]) => `${indent}  ${property}: ${value};`);
  return [`${indent}${selector} {`, ...body, `${indent}}`].join('\n');
}

function reducedMotion(tokens: readonly Token[]): Declaration[] {
  const changed = tokens
    .filter((token) => token.reduced !== undefined)
    .flatMap((token) => declarations({ ...token, value: token.reduced, source: token.reduced }));
  const springs = tokensOfType(tokens, 'spring').map((token): Declaration => [`--${token.name}`, '0ms linear']);
  return [...changed, ...springs];
}

function solidMaterials(tokens: readonly Token[]): Declaration[] {
  return tokensOfType(tokens, 'material').flatMap((token): Declaration[] => {
    const base = aliasName(fieldsOf(token.source)['base']);
    return [
      [`--${token.name}-bg`, `var(--${base ?? 'surface'})`],
      [`--${token.name}-blur`, SOLID_BLUR],
      [`--${token.name}-saturate`, SOLID_SATURATE],
    ];
  });
}

function media(query: string, lines: readonly Declaration[]): string {
  return [`@media (${query}) {`, block(':root', lines, '  '), '}'].join('\n');
}

/** tokens.css: custom properties on :root and the user-preference overrides (TOK-001, MAT-006, MOT-040). */
export function renderCss(tokens: readonly Token[]): string {
  const root = tokens.flatMap(declarations);
  const solid = solidMaterials(tokens);
  return [
    `/* ${GENERATED_NOTICE} */`,
    block(':root', root),
    '/* Motion changes (D-5): press and stagger are instant, springs jump; thresholds never change (MOT-021). */',
    media('prefers-reduced-motion: reduce', reducedMotion(tokens)),
    '/* Reduced transparency (5.6): solid materials, no blur (D-6). */',
    media('prefers-reduced-transparency: reduce', solid),
    '/* Increased contrast (MAT-006): solid materials and text-muted borders. */',
    media('prefers-contrast: more', [...solid, ['--border', 'var(--text-muted)']]),
    '',
  ].join('\n');
}
