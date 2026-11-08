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

it('UI-004, CR-10: a primary button is accent with a --bg label (6.24:1 dark, 6.06:1 light)', () => {
  const primary = rules().get('.primary');
  expect(primary?.get('background')).toBe('var(--accent)');
  expect(primary?.get('color')).toBe('var(--bg)');
});

it('UI-004, CR-10: the plain hover and press fills skip a primary button, so its --bg label never sits on surface-hover (1.69:1)', () => {
  const states = [...rules().keys()].filter((selector) => /^button:(hover|active)/.test(selector));
  expect(states).toHaveLength(2);
  for (const selector of states) expect(selector).toMatch(/:not\([^)]*\.primary\)/);
});

it('UI-004, CR-10: a pressed primary button is accent-deep', () => {
  const pressed = [...rules()].find(([selector]) => selector.startsWith('.primary:active'));
  expect(pressed?.[1].get('background')).toBe('var(--accent-deep)');
});
