import { fileURLToPath } from 'node:url';
import stylelint from 'stylelint';
import { describe, expect, it } from 'vitest';

const root = fileURLToPath(new URL('../../../../', import.meta.url));

async function warnings(code: string, file = 'apps/client/src/lint/sample.css') {
  const { results } = await stylelint.lint({
    code,
    codeFilename: `${root}${file}`,
    cwd: root,
    configFile: `${root}.stylelintrc.json`,
  });
  return results.flatMap((r) => r.warnings.map((w) => w.rule));
}

const inSvelte = (css: string) => `<h1>x</h1>\n<style>\n${css}\n</style>\n`;
const svelteFile = 'apps/client/src/lint/Sample.svelte';

const bad: Record<string, string> = {
  'a raw hex colour': 'a { color: #ff0000; }',
  'a raw named colour': 'a { color: red; }',
  'a raw rgb() colour': 'a { background-color: rgb(1, 2, 3); }',
  'a raw 200ms duration': 'a { transition-duration: 200ms; }',
  'a raw duration in the transition shorthand': 'a { transition: opacity 200ms var(--ease); }',
  'a raw cubic-bezier() curve': 'a { transition-timing-function: cubic-bezier(0.2, 0, 0, 1); }',
  'a raw ease keyword in the animation shorthand': 'a { animation: spin var(--d) ease-in; }',
  'a raw z-index': 'a { z-index: 10; }',
  'a raw font-family': 'a { font-family: Arial; }',
  'a raw font family in the font shorthand': 'a { font: 12px Arial; }',
  'a raw border colour': 'a { border: 1px solid #fff; }',
};

describe('Stylelint token gate', () => {
  for (const [name, css] of Object.entries(bad)) {
    it(`TOK-003 refuses ${name} in a .css file`, async () => {
      expect((await warnings(css)).length).toBeGreaterThan(0);
    });
  }

  for (const [name, css] of [
    ['a raw hex colour', bad['a raw hex colour']],
    ['a raw 200ms duration', bad['a raw 200ms duration']],
    ['a raw cubic-bezier() curve', bad['a raw cubic-bezier() curve']],
    ['a raw z-index', bad['a raw z-index']],
    ['a raw font-family', bad['a raw font-family']],
  ] as const) {
    it(`TOK-003 refuses ${name} in a .svelte <style>`, async () => {
      expect((await warnings(inSvelte(css), svelteFile)).length).toBeGreaterThan(0);
    });
  }

  it('TOK-003 accepts var(--token) references and neutral keywords', async () => {
    const css = `a {
  color: var(--color-text);
  background-color: transparent;
  border-color: currentColor;
  transition-duration: var(--motion-fast);
  transition-timing-function: var(--ease-standard);
  z-index: var(--z-modal);
  font-family: var(--font-body);
  margin: 0;
}`;
    expect(await warnings(css)).toEqual([]);
    expect(await warnings(inSvelte(css), svelteFile)).toEqual([]);
  });

  it('TOK-003 accepts raw values in the generated tokens.css', async () => {
    const css = ':root { --color-bg: #101010; --motion-fast: 120ms; --z-modal: 10; --font-body: Arial; }';
    expect(await warnings(css, 'apps/client/src/styles/tokens.css')).toEqual([]);
  });
});
