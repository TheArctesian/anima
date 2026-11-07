import { readFileSync } from 'node:fs';
import { compile } from 'svelte/compiler';
import { expect, it } from 'vitest';

const FILE = 'LiveSessions.svelte';

/** The component's stylesheet as Svelte compiles it: declarations by selector, scoping classes removed. */
function rules(): Map<string, Map<string, string>> {
  const source = readFileSync(new URL(FILE, import.meta.url), 'utf8');
  const code = (compile(source, { filename: FILE, css: 'external' }).css?.code ?? '').replaceAll(/\/\*[\s\S]*?\*\//g, '');
  const found = new Map<string, Map<string, string>>();
  for (const [, selector = '', body = ''] of code.matchAll(/([^{}]+)\{([^{}]*)\}/g)) {
    const declarations = body
      .split(';')
      .map((declaration) => declaration.split(':').map((part) => part.trim()))
      .filter(([property]) => property !== '');
    found.set(selector.replaceAll(/\.svelte-[\w-]+/g, '').trim(), new Map(declarations.map(([property = '', ...value]) => [property, value.join(':')])));
  }
  return found;
}

it('MAT-008, UI-002: the panel over a live display is the opaque raised surface with no blur', () => {
  const panel = rules().get('.panel');
  expect(panel?.get('background')).toBe('var(--surface-raised)');
  expect(panel?.get('color')).toBe('var(--text)');
  expect(panel?.get('box-shadow')).toBe('var(--elevation-2)');
  expect(panel?.get('border')).toContain('var(--border)');
  expect([...(panel?.keys() ?? [])].filter((property) => property.includes('backdrop'))).toEqual([]);
  expect(readFileSync(new URL(FILE, import.meta.url), 'utf8')).not.toContain('material-overlay');
});

it('MAT-008: failure text is --text on --status-down-tint (7.9:1), not --status-down (2.1:1)', () => {
  const failure = rules().get('.failure');
  expect(failure?.get('color')).toBe('var(--text)');
  expect(failure?.get('background')).toBe('var(--status-down-tint)');
});
