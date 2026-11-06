// @vitest-environment jsdom
import { describe, expect, it, vi } from 'vitest';
import { LINK_MAX, checkLink, linkRefusalText, linkTarget, openLink, type LinkRefusal } from './links';

describe('which links may open (K4 §8.7)', () => {
  it.each([
    ['https://example.com/a?b=1#c', 'https://example.com/a?b=1#c'],
    ['http://example.com', 'http://example.com/'],
    ['HTTPS://Example.COM/Path', 'https://example.com/Path'],
    ['https://bücher.example/', 'https://xn--bcher-kva.example/'],
    ['http://localhost:8080/x', 'http://localhost:8080/x'],
    [`https://example.com/${'a'.repeat(LINK_MAX - 'https://example.com/'.length)}`, `https://example.com/${'a'.repeat(LINK_MAX - 'https://example.com/'.length)}`],
  ])('K4 §8.7: %s opens as %s', (raw, expected) => {
    expect(linkTarget(raw)).toBe(expected);
  });

  it.each([
    'javascript:alert(1)',
    'JavaScript:alert(1)',
    ' javascript:alert(1)',
    'data:text/html,<script>alert(1)</script>',
    'file:///etc/passwd',
    'ftp://example.com/',
    'ssh://host',
    'vbscript:x',
    'blob:https://example.com/id',
    'about:blank',
    'https://accounts.google.com:443@evil.example/login',
    'https://user:secret@example.com/',
    `https://example.com/${'a'.repeat(LINK_MAX)}`,
    `https://example.com/${'\u00e9'.repeat(LINK_MAX - 'https://example.com/'.length - 18)}`,
    '//example.com/',
    '/relative',
    'example.com',
    '',
  ])('K4 §8.7: %j is refused', (raw) => {
    expect(linkTarget(raw)).toBeUndefined();
  });

  it('K4 §8.7: a confirmed link opens in a new context with no opener and no referrer', () => {
    const open = vi.spyOn(window, 'open').mockReturnValue(null);
    openLink('https://example.com/');
    expect(open).toHaveBeenCalledExactlyOnceWith('https://example.com/', '_blank', 'noopener,noreferrer');
    open.mockRestore();
  });
});

describe('one fixed message per refusal reason (K4 §8.7)', () => {
  it.each([
    ['javascript:alert(1)', 'scheme'],
    ['not a link', 'scheme'],
    ['https://user:secret@example.com/', 'credentials'],
    [`https://example.com/${'a'.repeat(LINK_MAX)}`, 'too-long'],
  ])('K4 §8.7: %.40s is refused as %s, with a sentence of its own that does not repeat the link', (raw, reason) => {
    const check = checkLink(raw);
    expect(check).toEqual({ ok: false, reason });
    if (check.ok) return;
    expect(linkRefusalText(check.reason)).not.toContain(raw.slice(0, 12));
  });

  it('K4 §8.7: the three sentences differ', () => {
    expect(new Set(['scheme', 'credentials', 'too-long'].map((reason) => linkRefusalText(reason as LinkRefusal))).size).toBe(3);
  });
});
