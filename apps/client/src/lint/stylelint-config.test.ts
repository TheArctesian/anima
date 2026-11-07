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
  'the linear keyword in the transition shorthand': 'a { transition: opacity var(--d) linear; }',
  'the linear keyword in the animation shorthand': 'a { animation: spin var(--d) linear; }',
  'a quoted string in a custom property': 'a { --label: "hi"; }',
  'a quoted string after a url() in a custom property': 'a { --m: url(x.svg) "y"; }',
  'a font-named custom property --f': 'a { --f: 12px; }',
  'a font-named custom property --font': 'a { --font: 12px; }',
  'a font-named custom property --font-x': 'a { --font-x: 12px; }',
  'a family-named custom property --x-family': 'a { --x-family: 12px; }',
  'a raw ease keyword in the animation shorthand': 'a { animation: spin var(--d) ease-in; }',
  'a raw z-index': 'a { z-index: 10; }',
  'a raw font-family': 'a { font-family: Arial; }',
  'a raw font family in the font shorthand': 'a { font: 12px Arial; }',
  'a raw border colour': 'a { border: 1px solid #fff; }',
  'an unlisted family in the font shorthand': 'a { font: 12px Roboto; }',
  'a quoted family with line-height in the font shorthand': 'a { font: 12px/1 "Fira Code"; }',
  'an unlisted family in font-family': 'a { font-family: Roboto, sans-serif; }',
  'an upper-case RGB() colour': 'a { color: RGB(1, 2, 3); }',
  'a raw number inside calc() for z-index': 'a { z-index: calc(1 + 2); }',
  'a raw duration in a custom property': 'a { --local: 200ms; }',
  'a raw z-index in a custom property': 'a { --z: 10; }',
  'a raw font family in a custom property': 'a { --f: Roboto; }',
  'a raw colour in a custom property': 'a { --c: #abc; }',
  'a raw easing curve in a custom property': 'a { --e: cubic-bezier(0, 0, 1, 1); }',
  'an unquoted unknown family in a font custom property': 'a { --f: Fira Code; }',
  'a font-named custom property holding a raw size': 'a { --font-size: 12px; }',
  'the linear easing keyword in a custom property': 'a { --e: linear; }',
  'the step-end easing keyword in a custom property': 'a { --e: step-end; }',
  'a calc() of literals in a layer custom property': 'a { --z: calc(1 + 2); }',
  'a primitive nord token in a colour': 'a { color: var(--nord-3); }',
  'a primitive nord token in a custom property': 'a { --surface-local: var(--nord-0); }',
  'a primitive nord token with a fallback': 'a { background-color: var(--nord-10, var(--bg)); }',
  'an upper-case 0.2S duration in transition': 'a { transition: opacity 0.2S var(--ease); }',
  'an upper-case EASE keyword in transition': 'a { transition: opacity var(--d) EASE; }',
  'a literal opacity': 'a { opacity: 0.5; }',
  'a literal fill-opacity': 'a { fill-opacity: 0.2; }',
  'a literal opacity in a percentage': 'a { opacity: 50%; }',
  'a @keyframes rule': '@keyframes spin { to { opacity: 0; } }',
  'a vendor @-webkit-keyframes rule': '@-webkit-keyframes spin { to { opacity: 0; } }',
};

const expectedRule: Record<string, string> = {
  'a raw border colour': 'color-no-hex',
  'a raw ease keyword in the animation shorthand': 'declaration-property-value-disallowed-list',
  'a quoted family with line-height in the font shorthand': 'declaration-property-value-allowed-list',
  'an unlisted family in font-family': 'declaration-property-value-allowed-list',
  'a raw colour in a custom property': 'color-no-hex',
  'a raw easing curve in a custom property': 'function-disallowed-list',
  'an upper-case 0.2S duration in transition': 'declaration-property-value-disallowed-list',
  'an upper-case EASE keyword in transition': 'declaration-property-value-disallowed-list',
  'a raw hex colour': 'color-no-hex',
  'a raw named colour': 'color-named',
  'a raw rgb() colour': 'function-disallowed-list',
  'an upper-case RGB() colour': 'function-disallowed-list',
  'a raw 200ms duration': 'scale-unlimited/declaration-strict-value',
  'a raw z-index': 'scale-unlimited/declaration-strict-value',
  'a raw number inside calc() for z-index': 'scale-unlimited/declaration-strict-value',
  'a raw cubic-bezier() curve': 'function-disallowed-list',
  'a raw duration in the transition shorthand': 'declaration-property-value-disallowed-list',
  'a raw font-family': 'declaration-property-value-allowed-list',
  'an unlisted family in the font shorthand': 'declaration-property-value-allowed-list',
  'a raw duration in a custom property': 'declaration-property-value-disallowed-list',
  'a raw z-index in a custom property': 'declaration-property-value-disallowed-list',
  'a raw font family in a custom property': 'declaration-property-value-disallowed-list',
  'an unquoted unknown family in a font custom property': 'declaration-property-value-allowed-list',
  'the linear easing keyword in a custom property': 'declaration-property-value-disallowed-list',
  'the step-end easing keyword in a custom property': 'declaration-property-value-disallowed-list',
  'a calc() of literals in a layer custom property': 'declaration-property-value-disallowed-list',
  'a primitive nord token in a colour': 'declaration-property-value-disallowed-list',
  'a primitive nord token in a custom property': 'declaration-property-value-disallowed-list',
  'a literal opacity': 'declaration-property-value-allowed-list',
  'a literal fill-opacity': 'declaration-property-value-allowed-list',
  'a literal opacity in a percentage': 'declaration-property-value-allowed-list',
  'a @keyframes rule': 'at-rule-disallowed-list',
  'a vendor @-webkit-keyframes rule': 'at-rule-disallowed-list',
};

describe('Stylelint token gate', () => {
  for (const [name, css] of Object.entries(bad)) {
    it(`TOK-003 refuses ${name} in a .css file`, async () => {
      const rules = await warnings(css);
      expect(rules.length).toBeGreaterThan(0);
      const expected = expectedRule[name];
      if (expected) expect(rules).toContain(expected);
    });
  }

  for (const [name, css] of [
    ['a raw hex colour', bad['a raw hex colour']],
    ['a raw 200ms duration', bad['a raw 200ms duration']],
    ['a raw cubic-bezier() curve', bad['a raw cubic-bezier() curve']],
    ['a raw z-index', bad['a raw z-index']],
    ['a raw font-family', bad['a raw font-family']],
    ['a @keyframes rule', bad['a @keyframes rule']],
  ] as const) {
    it(`TOK-003 refuses ${name} in a .svelte <style>`, async () => {
      expect((await warnings(inSvelte(css), svelteFile)).length).toBeGreaterThan(0);
    });
  }

  it('TOK-003 accepts var(--token) references and neutral keywords', async () => {
    const css = `a {
  color: var(--text);
  background-color: transparent;
  border-color: currentColor;
  transition-duration: var(--duration-fast);
  transition-timing-function: var(--ease-enter);
  z-index: var(--layer-modal);
  font-family: var(--font-ui);
  font: 12px/1.5 var(--font-mono);
  --local-gap: var(--space-2);
  --x: 0;
  --w: 50%;
  --font-local: var(--font-ui);
  --layer-local: var(--layer-modal);
  margin: 0;
}`;
    expect(await warnings(css)).toEqual([]);
    expect(await warnings(inSvelte(css), svelteFile)).toEqual([]);
  });

  it('TOK-003 accepts an opacity token, and 0 and 1 as the hidden and shown ends of a motion', async () => {
    const css = 'a { opacity: var(--opacity-disabled); fill-opacity: var(--opacity-band); } b { opacity: 0; } c { opacity: 1; }';
    expect(await warnings(css)).toEqual([]);
    expect(await warnings(inSvelte(css), svelteFile)).toEqual([]);
  });

  it('TOK-003 accepts one level of nested var() fallback in font and font-family', async () => {
    const css = 'a { font: 1rem var(--font-ui, var(--font-mono)); font-family: var(--font-ui, var(--font-mono)); }';
    expect(await warnings(css)).toEqual([]);
  });

  it('TOK-003 refuses calc() around var() in z-index on purpose: offsets come from tokens', async () => {
    expect(await warnings('a { z-index: calc(var(--z-base) + 1); }')).toContain(
      'scale-unlimited/declaration-strict-value',
    );
  });

});

describe('Stylelint token gate: vendored fonts and primitive tokens', () => {
  it('TOK-003 accepts quotes inside url() and non-font --f-* custom properties', async () => {
    expect(await warnings('a { --mask: url("x.svg"); --f-size: 12px; }')).toEqual([]);
  });

  it('TOK-003 accepts the vendored fonts.css with quoted @font-face families', async () => {
    const css = "@font-face { font-family: 'Fira Code'; src: url(x.woff2); }";
    expect(await warnings(css, 'apps/client/src/styles/fonts.css')).toEqual([]);
  });

  it('TOK-010 accepts primitive nord tokens inside tokens.css only', async () => {
    const css = ':root { --bg: var(--nord-0); }';
    expect(await warnings(css, 'apps/client/src/styles/tokens.css')).toEqual([]);
    expect(await warnings(css)).toContain('declaration-property-value-disallowed-list');
  });

  it('TOK-010 refuses a primitive nord token in a .svelte <style>', async () => {
    const rules = await warnings(inSvelte('a { color: var(--nord-3); }'), svelteFile);
    expect(rules).toContain('declaration-property-value-disallowed-list');
  });

  it('TOK-003 accepts raw values in the generated tokens.css', async () => {
    const css = ':root { --color-bg: #101010; --motion-fast: 120ms; --z-modal: 10; --font-body: Arial; }';
    expect(await warnings(css, 'apps/client/src/styles/tokens.css')).toEqual([]);
  });
});
