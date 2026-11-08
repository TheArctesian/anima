import { expect, test } from '@playwright/test';
import { createHash } from 'node:crypto';
import { inlineScriptHashes } from '../synthetic/inline-scripts';

// The synthetic hub must read index.html exactly as the hub does (WP-43 A-2), or a page the hub blocks would pass here.
// These are the cases of `crates/hub/src/web_client/tests/scripts.rs`, with the same expected hashes, in Node.

const FIRST = "'sha256-fDYmfWjvur9AxjhDbjHe6RvExc05MbXhhrbgz2vvy/A='";
const SECOND = "'sha256-VRcK3v7WYQJL3yHheBe4dsvLYjAYwFTyoPi2odncRio='";
const EMPTY = "'sha256-47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU='";

test('D-2: each inline script gets the hash of its exact body, in page order', () => {
  expect(inlineScriptHashes('<script>first()</script><p>text</p><script>second()</script>')).toEqual([FIRST, SECOND]);
});

test('D-2: a script with src gets no hash, in any spelling', () => {
  const html =
    '<script src=/a.js></script><SCRIPT type=module SRC="/b.js"></SCRIPT>' +
    "<script defer\nsrc = '/c.js' ></script><script src></script><script>first()</script>";
  expect(inlineScriptHashes(html)).toEqual([FIRST]);
});

test('D-2: attributes on an inline script do not change its hash; a quoted > does not end the tag', () => {
  const html = '<script type="module" nonce=x data-src="/no.js" data-y=\'a > b\'>first()</script>' + '<Script\tid=boot>second()</sCrIpT >';
  expect(inlineScriptHashes(html)).toEqual([FIRST, SECOND]);
});

test('D-2: the body is hashed as written: no whitespace is trimmed and no line end is changed', () => {
  const body = '\n\t\tfirst();\r\n  ';
  const exact = `'sha256-${createHash('sha256').update(body, 'utf8').digest('base64')}'`;
  expect(inlineScriptHashes(`<script>${body}</script>`)).toEqual([exact]);
  expect(exact).not.toBe(FIRST);
});

test('D-2: an attribute value that says src is not a src attribute', () => {
  expect(inlineScriptHashes('<script data-kind="src" title=src>first()</script>')).toEqual([FIRST]);
});

test('D-2: other elements and unclosed scripts give no hash', () => {
  for (const html of ['<scripts>first()</scripts>', '<script-x>first()</script-x>', '<script>first()', '<script', "<script type='x>"]) {
    expect(inlineScriptHashes(html), html).toEqual([]);
  }
});

test('D-2: an empty inline script gets the hash of nothing; a self-closing spelling still opens one', () => {
  expect(inlineScriptHashes('<script></script>')).toEqual([EMPTY]);
  expect(inlineScriptHashes('<script/>first()</script>')).toEqual([FIRST]);
});

test('D-2: non-ASCII text around and in a script is hashed as UTF-8; ASCII-only lowercasing keeps offsets', () => {
  // `İ` lowercases to two UTF-16 units with `toLowerCase`, which would shift every offset after it.
  const hashes = inlineScriptHashes('<p>Gerät ✓ İİ</p><script>first()</script><script>ü</script>');
  expect(hashes).toHaveLength(2);
  expect(hashes[0]).toBe(FIRST);
});

/** A small seeded generator (mulberry32), so a failing page can be replayed from its seed. */
function randomFrom(seed: number): () => number {
  let state = seed;
  return () => {
    state = (state + 0x6d2b79f5) | 0;
    let mixed = Math.imul(state ^ (state >>> 15), 1 | state);
    mixed = (mixed + Math.imul(mixed ^ (mixed >>> 7), 61 | mixed)) ^ mixed;
    return ((mixed ^ (mixed >>> 14)) >>> 0) / 4_294_967_296;
  };
}

const PIECES = ['<script', '<SCRIPT', '</script', '>', '/', 'src', '=', '"', "'", ' ', '\t', 'x', 'ü', '<', 'İ'];

test('D-2: any tag soup reads without throwing, with at most one well-formed hash per script opening', () => {
  for (let seed = 1; seed <= 2000; seed += 1) {
    const next = randomFrom(seed);
    const html = Array.from({ length: Math.floor(next() * 40) }, () => PIECES[Math.floor(next() * PIECES.length)]).join('');
    const hashes = inlineScriptHashes(html);
    expect(hashes.length, `seed ${String(seed)}`).toBeLessThanOrEqual(html.toLowerCase().split('<script').length - 1);
    for (const hash of hashes) expect(hash, `seed ${String(seed)}`).toMatch(/^'sha256-[A-Za-z0-9+/]{43}='$/);
  }
});

test('D-2: every inline script among plain text and src scripts gets exactly one hash', () => {
  for (let seed = 1; seed <= 500; seed += 1) {
    const next = randomFrom(seed);
    const plain = (): string => 'ab c;(){}=."\'ü'.slice(0, Math.floor(next() * 12));
    let html = '';
    let inline = 0;
    for (let part = Math.floor(next() * 8); part > 0; part -= 1) {
      const loaded = next() < 0.5;
      if (!loaded) inline += 1;
      html += `${plain()}${loaded ? '<script src="/x.js">' : '<script type=module>'}${plain()}</script>`;
    }
    expect(inlineScriptHashes(html).length, `seed ${String(seed)}`).toBe(inline);
  }
});
