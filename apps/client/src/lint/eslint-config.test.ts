import { fileURLToPath } from 'node:url';
import { ESLint } from 'eslint';
import { describe, expect, it } from 'vitest';

const root = fileURLToPath(new URL('../../../../', import.meta.url));
const eslint = new ESLint({ cwd: root });

async function rulesFired(code: string, filePath = 'apps/client/src/lint/sample.ts') {
  const [result] = await eslint.lintText(code, { filePath: `${root}${filePath}` });
  return result?.messages.map((m) => m.ruleId) ?? [];
}

const statements = (n: number) => Array.from({ length: n }, (_, i) => `  total += ${i};`).join('\n');

describe('ESLint limits and bans', () => {
  it('CQ-010 refuses a function with 5 parameters (max-params)', async () => {
    const code = 'export const f = (a: number, b: number, c: number, d: number, e: number) => a + b + c + d + e;\n';
    expect(await rulesFired(code)).toContain('max-params');
  });

  it('CQ-010 refuses a function longer than 60 code lines (max-lines-per-function)', async () => {
    const code = `export function f(): number {\n  let total = 0;\n${statements(61)}\n  return total;\n}\n`;
    expect(await rulesFired(code)).toContain('max-lines-per-function');
  });

  it('CQ-010 allows 60 code lines when blanks and comments are excluded', async () => {
    const body = `${statements(40)}\n\n// comment\n${'\n'.repeat(30)}`;
    const code = `export function f(): number {\n  let total = 0;\n${body}\n  return total;\n}\n`;
    expect(await rulesFired(code)).not.toContain('max-lines-per-function');
  });

  it('CQ-010 refuses nesting deeper than 4 (max-depth)', async () => {
    const code = `export function f(x: boolean): number {
  if (x) { if (x) { if (x) { if (x) { if (x) { return 1; } } } } }
  return 0;
}\n`;
    expect(await rulesFired(code)).toContain('max-depth');
  });

  it('CQ-010 refuses cyclomatic complexity above 15 (complexity)', async () => {
    const branches = Array.from({ length: 15 }, (_, i) => `  if (x === ${i}) { y += 1; }`).join('\n');
    const code = `export function f(x: number): number {\n  let y = 0;\n${branches}\n  return y;\n}\n`;
    expect(await rulesFired(code)).toContain('complexity');
  });

  it('CQ-025 refuses any', async () => {
    expect(await rulesFired('export const f = (x: any): unknown => x;\n')).toContain(
      '@typescript-eslint/no-explicit-any',
    );
  });

  it('CQ-025 refuses the non-null assertion', async () => {
    const code = 'export const f = (x: string | null): number => x!.length;\n';
    expect(await rulesFired(code)).toContain('@typescript-eslint/no-non-null-assertion');
  });

  it('CQ-013 refuses an eslint-disable comment without a reason', async () => {
    const code = '// eslint-disable-next-line max-params\nexport const f = (a: number, b: number, c: number, d: number, e: number) => a + b + c + d + e;\n';
    expect(await rulesFired(code)).toContain('@eslint-community/eslint-comments/require-description');
  });

  it('CQ-013 accepts an eslint-disable comment with a reason', async () => {
    const code = '// eslint-disable-next-line max-params -- wire format has five fields\nexport const f = (a: number, b: number, c: number, d: number, e: number) => a + b + c + d + e;\n';
    expect(await rulesFired(code)).toEqual([]);
  });

  it('CQ-043 passes a clean TypeScript snippet', async () => {
    const code = 'export const add = (a: number, b: number): number => a + b;\n';
    expect(await rulesFired(code)).toEqual([]);
  });

  it('CQ-043 applies the limits inside a Svelte script block', async () => {
    const code = '<script lang="ts">\n  export const f = (a: number, b: number, c: number, d: number, e: number) => a + b + c + d + e;\n</script>\n';
    expect(await rulesFired(code, 'apps/client/src/lint/Sample.svelte')).toContain('max-params');
  });

  it('CQ-010 covers quality/**/*.ts', async () => {
    const code = 'export const f = (a: number, b: number, c: number, d: number, e: number) => a + b + c + d + e;\n';
    expect(await rulesFired(code, 'quality/src/sample.ts')).toContain('max-params');
  });

  it('CQ-043 activates eslint-plugin-svelte recommended (no-at-html-tags)', async () => {
    const code = '<script lang="ts">\n  const x = "<b>hi</b>";\n</script>\n{@html x}\n';
    expect(await rulesFired(code, 'apps/client/src/lint/Sample.svelte')).toContain('svelte/no-at-html-tags');
  });
});

describe('ESLint token literals (TOK-003)', () => {
  const svelte = (markup: string) => `<script lang="ts">\n  export let x = 1;\n</script>\n${markup}\n`;
  const restricted = 'no-restricted-syntax';
  const sample = 'apps/client/src/lib/sample.ts';
  const svelteSample = 'apps/client/src/lib/Sample.svelte';
  const refused: Record<string, string> = {
    'a 6-digit hex colour string': "export const c = '#ff0000';\n",
    'a 3-digit hex colour string': "export const c = '#f00';\n",
    'an 8-digit hex colour string': "export const c = '#ff000080';\n",
    'a hex colour in a template literal': 'export const c = `border: 1px solid #fff`;\n',
    'an rgb() string': "export const c = 'rgb(1, 2, 3)';\n",
    'an rgba() string': "export const c = 'rgba(1, 2, 3, 0.5)';\n",
    'an hsl() string': "export const c = 'hsl(10 20% 30%)';\n",
    'an oklch() string': "export const c = 'oklch(0.5 0.1 200)';\n",
    'a cubic-bezier() string': "export const e = 'cubic-bezier(0.2, 0, 0, 1)';\n",
    'a 200ms string': "export const d = '200ms';\n",
    'a 0.2s string': "export const d = '0.2s';\n",
    'a duration inside a template literal': 'export const d = `opacity 200ms ease`;\n',
    'a numeric duration property': 'export const o = { duration: 200 };\n',
    'a numeric delay property': 'export const o = { delay: 0 };\n',
    'a numeric stiffness property': 'export const o = { stiffness: 170 };\n',
    'a numeric damping property': 'export const o = { damping: 26 };\n',
    'a numeric mass property': 'export const o = { mass: 1 };\n',
    'a numeric bounce property': 'export const o = { bounce: 0.2 };\n',
    'a numeric visualDuration property': 'export const o = { visualDuration: 0.3 };\n',
    'a numeric ease array': 'export const o = { ease: [0.2, 0, 0, 1] };\n',
    'a numeric zIndex property': 'export const o = { zIndex: 10 };\n',
    'a quoted numeric duration key': "export const o = { 'duration': 200 };\n",
    'an assigned zIndex': 'export const f = (el: { zIndex: number }) => { el.zIndex = 5; };\n',
  };
  for (const [name, code] of Object.entries(refused)) {
    it(`TOK-003 refuses ${name} in a .ts file`, async () => {
      expect(await rulesFired(code, sample)).toContain(restricted);
    });
  }

  it('TOK-003 names the rule in the message', async () => {
    const [result] = await eslint.lintText("export const c = '#ff0000';\n", { filePath: `${root}${sample}` });
    expect(result?.messages[0]?.message).toContain('Use a design token (TOK-003)');
  });

  it('TOK-003 refuses a hex colour in a Svelte script block', async () => {
    const code = '<script lang="ts">\n  const c = "#ff0000";\n  export { c };\n</script>\n';
    expect(await rulesFired(code, svelteSample)).toContain(restricted);
  });

  it('TOK-003 refuses an inline style attribute with a hex colour', async () => {
    expect(await rulesFired(svelte('<p style="color: #fff">x</p>'), svelteSample)).toContain(restricted);
  });

  it('TOK-003 refuses a style: directive with a raw duration', async () => {
    expect(await rulesFired(svelte('<p style:transition-duration="200ms">x</p>'), svelteSample)).toContain(restricted);
  });

  it('TOK-003 accepts an inline style attribute that reads tokens', async () => {
    const code = svelte('<p style="color: var(--text); margin: var(--space-2)">x</p>');
    expect(await rulesFired(code, svelteSample)).toEqual([]);
  });

  it('TOK-003 accepts token references, identifiers and non-colour strings', async () => {
    const code = `import { duration, spring, layer, ease } from '$lib/motion/tokens';
export const a = { duration: duration.fast, delay: duration.base, zIndex: layer.modal };
export const b = { stiffness: spring.default.stiffness };
export const c = 'var(--text)';
export const d = '/devices#overview';
export const e = { count: 3, width: 200, ease: ease.enter };
export const f = 'items: 5 of 10';
`;
    expect(await rulesFired(code, sample)).toEqual([]);
  });

  it('TOK-003 exempts the generated lib/motion/tokens.ts', async () => {
    const code = "export const spring = { default: { stiffness: 170, damping: 26 } };\nexport const c = '#ff0000';\nexport const o = { duration: 200 };\n";
    expect(await rulesFired(code, 'apps/client/src/lib/motion/tokens.ts')).toEqual([]);
  });

  it('TOK-003 exempts test files', async () => {
    expect(await rulesFired("export const c = '#ff0000';\n", 'apps/client/src/lib/sample.test.ts')).toEqual([]);
  });

  it('TOK-003 does not apply outside apps/client', async () => {
    expect(await rulesFired("export const c = '#ff0000';\n", 'quality/src/sample.ts')).toEqual([]);
  });
});
