import { describe, expect, it } from 'vitest';
import { NO_LATCH, latched, toggled, withLatch } from './keys';

const bytes = (...codes: number[]) => Uint8Array.from(codes);
const typed = (value: string) => Uint8Array.from(new TextEncoder().encode(value));

describe('the Ctrl and Alt keys of an SSH session on a phone (RA-050, MOB-020)', () => {
  it('sends a typed key as it is when nothing is latched', () => {
    expect(withLatch(typed('c'), NO_LATCH)).toEqual(typed('c'));
  });

  it('Ctrl turns a letter of either case, or a space, into its control code', () => {
    const ctrl = toggled(NO_LATCH, 'ctrl');
    expect(withLatch(typed('c'), ctrl)).toEqual(bytes(0x03));
    expect(withLatch(typed('L'), ctrl)).toEqual(bytes(0x0c));
    expect(withLatch(typed(' '), ctrl)).toEqual(bytes(0x00));
    expect(withLatch(typed('1'), ctrl)).toEqual(typed('1'));
  });

  it('Alt puts Escape before the key, and Ctrl and Alt combine', () => {
    expect(withLatch(typed('b'), toggled(NO_LATCH, 'alt'))).toEqual(bytes(0x1b, 0x62));
    expect(withLatch(typed('c'), toggled(toggled(NO_LATCH, 'ctrl'), 'alt'))).toEqual(bytes(0x1b, 0x03));
  });

  it('leaves a paste of more than one byte as it is', () => {
    expect(withLatch(typed('ls -la'), toggled(NO_LATCH, 'ctrl'))).toEqual(typed('ls -la'));
  });

  it('a second press of a modifier lets it go', () => {
    const once = toggled(NO_LATCH, 'ctrl');
    expect(latched(once)).toBe(true);
    expect(latched(toggled(once, 'ctrl'))).toBe(false);
  });
});
