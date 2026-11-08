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
      // A light colour may be a mix (CR-10); the loader has checked its shape.
      return typeof token.value === 'string' ? aliasOrValue(token.source, token.value) : tint(token);
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

/** The custom properties a token declares. Breakpoints and gesture constants are TypeScript only (K5). */
export function declarations(token: Token): Declaration[] {
  if (token.type === 'breakpoint' || token.type === 'gesture') return [];
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

function media(query: string, lines: readonly Declaration[], selector = ':root'): string {
  return [`@media (${query}) {`, block(selector, lines, '  '), '}'].join('\n');
}

function lightTheme(tokens: readonly Token[]): Declaration[] {
  return tokens.flatMap((token) => (token.light === undefined ? [] : declarations({ ...token, ...token.light })));
}

/**
 * The light theme (UI-005, K5): one set of declarations for the operator's Light and for System on a light OS.
 * Empty when no token has a light value, so the dark-only output stays as it was.
 */
function lightBlocks(lines: readonly Declaration[]): string[] {
  if (lines.length === 0) return [];
  return [
    '/* Light theme (UI-005): semantic colours only (TOK-011); System follows the OS without a script. */',
    block(':root[data-theme="light"]', lines),
    media('prefers-color-scheme: light', lines, ':root[data-theme="system"]'),
  ];
}

/**
 * The operator's "Reduce motion" choice (MOT-043, D-28): `data-motion="reduce"` takes the reduced values whatever the OS
 * says, and `data-motion="full"` puts back the full values on an OS that asks for reduced motion. `:root` with one
 * attribute outranks the plain `:root` of the default and media blocks wherever it sits, and these blocks hold motion
 * tokens only, so they never meet a theme or preference block. Without the attribute (System) the media query alone
 * decides, as before. Empty when no token changes under reduced motion.
 */
function motionChoiceBlocks(tokens: readonly Token[], reduced: readonly Declaration[]): string[] {
  if (reduced.length === 0) return [];
  const values = new Map(tokens.flatMap(declarations));
  const full = reduced.flatMap(([property]): Declaration[] => {
    const value = values.get(property);
    return value === undefined ? [] : [[property, value]];
  });
  return [
    '/* Reduce motion in Settings (MOT-043): On reduces whatever the OS says; Off keeps full motion on a reducing OS. */',
    block(':root[data-motion="reduce"]', reduced),
    media('prefers-reduced-motion: reduce', full, ':root[data-motion="full"]'),
  ];
}

/** tokens.css: custom properties on :root, the light theme and the user-preference overrides (TOK-001, MAT-006, MOT-040). */
export function renderCss(tokens: readonly Token[]): string {
  const root = tokens.flatMap(declarations);
  const solid = solidMaterials(tokens);
  const light = lightTheme(tokens);
  // `:root[data-theme]` matches the light selectors' specificity, so these later blocks win over either theme (MAT-006).
  const preference = light.length === 0 ? ':root' : ':root, :root[data-theme]';
  const reduced = reducedMotion(tokens);
  return [
    `/* ${GENERATED_NOTICE} */`,
    block(':root', root),
    ...lightBlocks(light),
    '/* Motion changes (D-5): press and stagger are instant, springs jump; thresholds never change (MOT-021). */',
    media('prefers-reduced-motion: reduce', reduced),
    ...motionChoiceBlocks(tokens, reduced),
    '/* Reduced transparency (5.6): solid materials, no blur (D-6). */',
    media('prefers-reduced-transparency: reduce', solid, preference),
    '/* Increased contrast (MAT-006): solid materials and text-muted borders. */',
    media('prefers-contrast: more', [...solid, ['--border', 'var(--text-muted)']], preference),
    '',
  ].join('\n');
}
