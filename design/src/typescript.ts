import { GENERATED_NOTICE, bezierOf, fieldsOf, leadingNumber, springOf, trimmedDecimal, type Token } from './model.ts';
import { toLibraryForm } from './spring.ts';

interface LiteralObject {
  readonly [key: string]: Literal;
}
type Literal = number | string | null | readonly Literal[] | LiteralObject;

/** Export name for each source group, and the K5 prefix that its keys drop. */
export const EXPORTS = [
  { group: 'motion.spring', export: 'spring', prefix: 'spring-' },
  { group: 'motion.duration', export: 'duration', prefix: 'duration-' },
  { group: 'motion.ease', export: 'ease', prefix: 'ease-' },
  { group: 'motion.stagger', export: 'stagger', prefix: 'stagger-' },
  { group: 'motion.threshold', export: 'threshold', prefix: 'threshold-' },
  { group: 'layout.bp', export: 'breakpoint', prefix: 'bp-' },
] as const;

type ExportSpec = (typeof EXPORTS)[number];

/** `threshold-loading-min` becomes `loadingMin`. */
export function keyOf(name: string, prefix: string): string {
  return name
    .slice(prefix.length)
    .replace(/-([a-z0-9])/g, (_, letter: string) => letter.toUpperCase());
}

function render(value: Literal, indent: string): string {
  if (value === null) return 'null';
  if (typeof value === 'number') return String(value);
  if (typeof value === 'string') return `'${value}'`;
  if (Array.isArray(value)) return `[${value.map((item) => render(item, indent)).join(', ')}]`;
  const entries = Object.entries(value as Record<string, Literal>);
  const inner = `${indent}  `;
  const lines = entries.map(([key, item]) => `${inner}${key}: ${render(item, inner)},`);
  return `{\n${lines.join('\n')}\n${indent}}`;
}

function springValue(token: Token): Literal {
  const { dampingRatio, responseS, stiffness, damping } = toLibraryForm(springOf(token));
  return { dampingRatio, responseS, stiffness: Number(trimmedDecimal(stiffness)), damping: Number(trimmedDecimal(damping)) };
}

function exportedValue(token: Token): Literal {
  switch (token.type) {
    case 'spring':
      return springValue(token);
    case 'duration':
    case 'dimension':
      return leadingNumber(token.value);
    case 'cubicBezier':
      return [...bezierOf(token)];
    default:
      return fieldsOf(token.value) as LiteralObject;
  }
}

function section(spec: ExportSpec, tokens: readonly Token[]): string {
  const members = tokens.filter((token) => token.group === spec.group);
  const entries = Object.fromEntries(members.map((token) => [keyOf(token.name, spec.prefix), exportedValue(token)]));
  return `export const ${spec.export} = ${render(entries, '')} as const;`;
}

function reducedSection(tokens: readonly Token[]): string {
  const reduced = (spec: ExportSpec): Literal => {
    const members = tokens.filter((token) => token.group === spec.group);
    return Object.fromEntries(
      members.map((token) => {
        const value = token.reduced === undefined ? token : { ...token, value: token.reduced };
        return [keyOf(token.name, spec.prefix), exportedValue(value)];
      }),
    );
  };
  const [duration, stagger] = [EXPORTS[1], EXPORTS[3]].map(reduced);
  return [
    '/** Values under prefers-reduced-motion (D-5, MOT-041). Springs become instant moves; thresholds never change (MOT-021). */',
    `export const reducedMotion = ${render({ duration: duration ?? null, stagger: stagger ?? null, springs: 'instant' }, '')} as const;`,
  ].join('\n');
}

/** tokens.ts: the typed constants for springs, durations, easing, stagger, thresholds and breakpoints (K5). */
export function renderTypescript(tokens: readonly Token[]): string {
  const springNote = '// Spring: stiffness = (2π / responseS)², damping = 4π × dampingRatio / responseS, mass 1 (5.7.2).';
  return [
    `// ${GENERATED_NOTICE}\n${springNote}`,
    ...EXPORTS.map((spec) => section(spec, tokens)),
    reducedSection(tokens),
  ].join('\n\n').concat('\n');
}
