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
