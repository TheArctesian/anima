import { describe, expect, it } from 'vitest';
import { webViewEventFrom } from './events';
import { displayFingerprint, fingerprintFrom } from './fingerprint';

const PIN = 'ab'.repeat(32);
const OTHER = 'cd'.repeat(32);

describe('fingerprint (D-5, H-4)', () => {
  it('D-5: takes exactly 64 lower-case hex digits and nothing else', () => {
    expect(fingerprintFrom(PIN)).toBe(PIN);
    for (const wrong of [[PIN], { toString: () => PIN }, PIN.toUpperCase(), PIN.slice(1), `${PIN}a`, PIN.replace(/(..)(?!$)/g, '$1:'), ` ${PIN}`, `g${PIN.slice(1)}`, 64, null]) {
      expect(fingerprintFrom(wrong)).toBeNull();
    }
  });

  it('D-5: shows colon-separated pairs in upper case', () => {
    const fingerprint = fingerprintFrom(`0a1b${'00'.repeat(28)}fe9d`);
    expect(fingerprint).not.toBeNull();
    if (fingerprint === null) return;
    const shown = displayFingerprint(fingerprint);
    expect(shown.startsWith('0A:1B:00:')).toBe(true);
    expect(shown.endsWith(':FE:9D')).toBe(true);
    expect(shown.split(':')).toHaveLength(32);
  });
});

describe('webViewEventFrom (WP-31 shell interface)', () => {
  it('reads each event the shell sends', () => {
    expect(webViewEventFrom({ tabId: 't-1', kind: 'LOADING', url: 'https://nas.lan:5001/' })).toEqual({ tabId: 't-1', kind: 'LOADING', url: 'https://nas.lan:5001/' });
    expect(webViewEventFrom({ tabId: 't-1', kind: 'LOADED', url: 'http://10.0.0.2/x' })).toEqual({ tabId: 't-1', kind: 'LOADED', url: 'http://10.0.0.2/x' });
    expect(webViewEventFrom({ tabId: 't-1', kind: 'UNTRUSTED', fingerprintSha256: PIN })).toEqual({ tabId: 't-1', kind: 'UNTRUSTED', fingerprintSha256: PIN });
    expect(webViewEventFrom({ tabId: 't-1', kind: 'MISMATCH', expected: PIN, presented: OTHER })).toEqual({ tabId: 't-1', kind: 'MISMATCH', expected: PIN, presented: OTHER });
    expect(webViewEventFrom({ tabId: 't-1', kind: 'FAILED', reason: 'unreachable' })).toEqual({ tabId: 't-1', kind: 'FAILED', reason: 'unreachable' });
  });

  it.each([
    ['not an object', 'LOADED'],
    ['an array', [{ tabId: 't', kind: 'LOADED', url: 'http://a/' }]],
    ['null', null],
    ['an unknown kind', { tabId: 't', kind: 'CLOSED' }],
    ['a kind from the prototype', { tabId: 't', kind: 'toString' }],
    ['a missing field', { tabId: 't', kind: 'LOADED' }],
    ['an extra field', { tabId: 't', kind: 'LOADED', url: 'http://a/', title: 'page text' }],
    ['another kind’s field', { tabId: 't', kind: 'UNTRUSTED', fingerprintSha256: PIN, url: 'https://a/' }],
    ['an empty tab ID', { tabId: '', kind: 'LOADED', url: 'http://a/' }],
    ['a tab ID with a slash', { tabId: 'a/b', kind: 'LOADED', url: 'http://a/' }],
    ['a tab ID that is not text', { tabId: 7, kind: 'LOADED', url: 'http://a/' }],
    ['a javascript URL', { tabId: 't', kind: 'LOADED', url: 'javascript:alert(1)' }],
    ['a file URL', { tabId: 't', kind: 'LOADING', url: 'file:///etc/passwd' }],
    ['a URL that does not parse', { tabId: 't', kind: 'LOADED', url: 'http://' }],
    ['a URL that is too long', { tabId: 't', kind: 'LOADED', url: `http://a/${'x'.repeat(8192)}` }],
    ['an upper-case fingerprint', { tabId: 't', kind: 'UNTRUSTED', fingerprintSha256: PIN.toUpperCase() }],
    ['a colon fingerprint', { tabId: 't', kind: 'UNTRUSTED', fingerprintSha256: PIN.replace(/(..)(?!$)/g, '$1:') }],
    ['an upper-case presented fingerprint', { tabId: 't', kind: 'MISMATCH', expected: PIN, presented: OTHER.toUpperCase() }],
    ['a presented fingerprint in a list', { tabId: 't', kind: 'MISMATCH', expected: PIN, presented: [OTHER] }],
    ['a short presented fingerprint', { tabId: 't', kind: 'MISMATCH', expected: PIN, presented: OTHER.slice(2) }],
    ['a reason with a URL', { tabId: 't', kind: 'FAILED', reason: 'could not load https://a/' }],
    ['a reason that is not text', { tabId: 't', kind: 'FAILED', reason: { code: 'x' } }],
  ])('drops %s', (_what, payload) => {
    expect(webViewEventFrom(payload)).toBeNull();
  });

  it('never throws and never passes a shape it did not check, for arbitrary payloads', () => {
    // A fixed-seed generator, so a failure is the same on every run.
    let seed = 0x5eed;
    const next = () => (seed = (seed * 1103515245 + 12345) % 2 ** 31) / 2 ** 31;
    const pick = <T>(choices: readonly T[]): T => choices[Math.floor(next() * choices.length)] as T;
    const values = [undefined, null, 0, -1, true, '', 'LOADED', 'FAILED', 't-1', PIN, 'http://a/', 'data:x', [], {}, 'x'.repeat(300)];
    const keys = ['tabId', 'kind', 'url', 'reason', 'expected', 'presented', 'fingerprintSha256', '__proto__', 'constructor'];
    for (let run = 0; run < 2000; run += 1) {
      const payload: Record<string, unknown> = {};
      for (let field = Math.floor(next() * 5); field > 0; field -= 1) payload[pick(keys)] = pick(values);
      const event = webViewEventFrom(payload);
      if (event === null) continue;
      expect(['LOADING', 'LOADED', 'UNTRUSTED', 'MISMATCH', 'FAILED']).toContain(event.kind);
      expect(Object.keys(event).sort()).toEqual(Object.keys(payload).sort());
    }
  });
});
