import { expect, test } from '@playwright/test';
import { readdirSync, readFileSync } from 'node:fs';
import { join, relative } from 'node:path';
import { inlineScripts, inlineScriptHashes } from '../synthetic/inline-scripts';
import { BUILD_FOLDER } from '../synthetic/static-files';
import { builtIndex } from './k7-policy';

// The production build, read as files (E-6): what the browser specs cannot reach on every code path. The hub allows by
// hash only the inline scripts of index.html (A-2) and serves every other file under that same policy, and K7 has no
// 'unsafe-eval', no 'unsafe-inline' for scripts and no blob: for workers or frames. These rules keep the build inside it.

function filesUnder(folder: string): string[] {
  return readdirSync(folder, { withFileTypes: true }).flatMap((entry) =>
    entry.isDirectory() ? filesUnder(join(folder, entry.name)) : [join(folder, entry.name)],
  );
}

const FILES = filesUnder(BUILD_FOLDER);
const named = (extension: string): { name: string; text: string }[] =>
  FILES.filter((file) => file.endsWith(extension)).map((file) => ({ name: relative(BUILD_FOLDER, file), text: readFileSync(file, 'utf8') }));

/** Places in `text` that match `pattern`, with a little context, for the failure message. */
function places(text: string, pattern: RegExp): string[] {
  return [...text.matchAll(new RegExp(pattern.source, `${pattern.flags.replace('g', '')}g`))].map((found) =>
    text.slice(Math.max(0, found.index - 40), found.index + found[0].length + 40),
  );
}

/** Code K7 refuses at run time, in any script of the build. */
// `eval` and `Function` count bare and on a global object, with or without `?.`: the minifier writes
// `(globalThis as …)[['ev','al'].join('')]?.('1')` as `globalThis.eval?.(`. A member of anything else (`a.evaluate(`,
// `x.isFunction(`) is not the global.
const GLOBAL_CALL = String.raw`(?:^|[^\w$.]|\b(?:globalThis|window|self)\s*\.\s*)`;
const FORBIDDEN_CODE: Record<string, RegExp> = {
  eval: new RegExp(String.raw`${GLOBAL_CALL}eval\s*(?:\?\.\s*)?\(`),
  'Function constructor': new RegExp(String.raw`${GLOBAL_CALL}Function\s*(?:\?\.\s*)?\(`),
  'string timer': /(?<![\w$.])set(?:Timeout|Interval)\s*\(\s*["'`]/,
  'worker or shared worker': /new\s+(?:Shared)?Worker\s*\(/,
  'service worker registration': /serviceWorker\s*\.\s*register\s*\(/,
  'frame element': /createElement\(\s*["'`]i?frame["'`]|<i?frame[\s>]/i,
  'inline handler in markup': /<[a-z][\w-]*\s[^<>`]*\son[a-z]+\s*=/i,
  'inline handler set by script': /setAttribute\(\s*["'`]on[a-z]+["'`]/i,
  'javascript: URL': /["'`]javascript:/i,
};

test("SEC-001: SvelteKit's start script is the only inline script in index.html", () => {
  const scripts = inlineScripts(builtIndex());
  expect(scripts).toHaveLength(1);
  expect(scripts[0]).toMatch(/__sveltekit_\w+ = \{/);
  expect(scripts[0]).toMatch(/kit\.start\(app, element\)/);
  expect(scripts[0]).toMatch(/import\("\/_app\/immutable\/entry\/start\.[\w-]+\.js"\)/);
});

test('SEC-001: every HTML file in the build has only inline scripts that the index.html hashes allow', () => {
  const allowed = new Set(inlineScriptHashes(builtIndex()));
  for (const page of named('.html')) {
    for (const hash of inlineScriptHashes(page.text)) expect(allowed, page.name).toContain(hash);
  }
});

test('SEC-001: no HTML file in the build has an inline event handler or a javascript: URL', () => {
  for (const page of named('.html')) {
    expect(places(page.text, /<[a-z][\w-]*\s[^<>]*\son[a-z]+\s*=/i), page.name).toEqual([]);
    expect(places(page.text, /javascript:/i), page.name).toEqual([]);
  }
});

test('SEC-001: no script in the build evals, makes a worker or a frame, or writes an inline handler', () => {
  const scripts = named('.js');
  expect(scripts.length).toBeGreaterThan(0);
  for (const script of scripts) {
    for (const [rule, pattern] of Object.entries(FORBIDDEN_CODE)) {
      expect(places(script.text, pattern), `${rule} in ${script.name}`).toEqual([]);
    }
  }
});

test('SEC-001: the rules above find each breach they name', () => {
  // A rule that matches nothing proves nothing: each one must catch its own example.
  const examples: Record<string, readonly string[]> = {
    eval: ['x=eval("1")', 'eval?.("1")', 'globalThis.eval?.(`1`)', 'window.eval("1")', 'self . eval ("1")'],
    'Function constructor': ['new Function("return 1")', 'self.Function("")', 'globalThis.Function?.("")', 'window.Function("")'],
    'string timer': ['setTimeout(`go()`,0)'],
    'worker or shared worker': ['new Worker(URL.createObjectURL(b))'],
    'service worker registration': ['navigator.serviceWorker.register("/sw.js")'],
    'frame element': ['document.createElement(`iframe`)'],
    'inline handler in markup': ['`<img src="x.png" onerror="go()">`'],
    'inline handler set by script': ['e.setAttribute(`onclick`,"go()")'],
    'javascript: URL': ['a.href="javascript:go()"'],
  };
  for (const [rule, pattern] of Object.entries(FORBIDDEN_CODE)) {
    expect(examples[rule], rule).toBeDefined();
    for (const example of examples[rule] ?? []) expect(places(example, pattern), `${rule} in ${example}`).not.toEqual([]);
  }
  const nearMisses = ['a.evaluate(e)', 'isFunction(x)', 'x.isFunction(y)', 'a.eval(1)', 'typeof x.importScripts', 'getRegistration("/")', 'button.onclick=go'];
  for (const near of nearMisses) {
    for (const [rule, pattern] of Object.entries(FORBIDDEN_CODE)) expect(places(near, pattern), `${rule} on ${near}`).toEqual([]);
  }
});
