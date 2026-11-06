import { describe, expect, it } from 'vitest';
import { safePaste } from './paste';

describe('paste (RA-003, K4 §8.7)', () => {
  it('RA-003: the escape that would end a bracketed paste early is removed', () => {
    expect(safePaste('safe\u001b[201~touch /tmp/pwned\n')).toBe('safe[201~touch /tmp/pwned\n');
  });

  it.each([
    ['tab, line feed and carriage return stay', 'a\tb\nc\r\nd', 'a\tb\nc\r\nd'],
    ['C0 controls go', 'a\u0000\u0003\u0007\u0008\u000b\u000c\u001a\u001bz', 'az'],
    ['DEL and C1 controls go', 'a\u007f\u0085\u009bz', 'az'],
    ['text and emoji stay', 'é ✓ \u{1f680}', 'é ✓ \u{1f680}'],
  ])('RA-003: %s', (_name, input, expected) => {
    expect(safePaste(input)).toBe(expected);
  });
});
