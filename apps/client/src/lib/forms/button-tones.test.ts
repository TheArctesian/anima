import { readFileSync } from 'node:fs';
import { compile } from 'svelte/compiler';
import { expect, it } from 'vitest';

const FILE = 'FormButton.svelte';

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

function declared(selector: string, property: string): string | undefined {
  return rules().get(selector)?.get(property);
}

it('UI-029, UI-004: primary is the accent fill with an on-accent label, hover accent-hover, pressed accent-pressed', () => {
  expect(declared('.primary', 'background')).toBe('var(--accent)');
  expect(declared('.primary', 'color')).toBe('var(--on-accent)');
  expect(declared('.primary:hover:not(:disabled, [aria-disabled=\'true\'])', 'background')).toBe('var(--accent-hover)');
  expect(declared('.primary:active:not(:disabled, [aria-disabled=\'true\'])', 'background')).toBe('var(--accent-pressed)');
});

it('UI-029: secondary is the neutral surface-overlay fill with text colour', () => {
  expect(declared('.secondary', 'background')).toBe('var(--surface-overlay)');
  expect(declared('button', 'color')).toBe('var(--text)');
});

it('UI-029: ghost has no fill and text-secondary; hover shows surface-hover and text', () => {
  expect(declared('.ghost', 'color')).toBe('var(--text-secondary)');
  expect(declared('.ghost', 'background')).toBeUndefined();
  const hover = [...rules()].find(([selector]) => selector.startsWith('.ghost:hover'))?.[1];
  expect(hover?.get('background')).toBe('var(--surface-hover)');
  expect(hover?.get('color')).toBe('var(--text)');
});

it('UI-029, UI-004: danger is an outline by default and a solid danger fill with an on-danger label when filled', () => {
  expect(declared('.danger', 'border-color')).toBe('var(--danger)');
  expect(declared('.danger', 'background')).toBeUndefined();
  expect(declared('.danger.fill', 'background')).toBe('var(--danger)');
  expect(declared('.danger.fill', 'color')).toBe('var(--on-danger)');
});

it('UI-013: the sizes take their height from the control tokens', () => {
  expect(declared('button', 'block-size')).toBe('var(--control-md)');
  expect(declared('.sm', 'block-size')).toBe('var(--control-sm)');
  expect(declared('.lg', 'block-size')).toBe('var(--control-lg)');
});
