import { describe, expect, it, vi } from 'vitest';
import { firstDirection, scaledFade } from './path';

describe('scaledFade', () => {
  it('goes from gone at the scale it has when gone to in place at full strength', () => {
    const style = scaledFade(0.5, '10px 20px');
    expect(style(0)).toBe('opacity: 0; scale: 0.5; transform-origin: 10px 20px;');
    expect(style(1)).toBe('opacity: 1; scale: 1; transform-origin: 10px 20px;');
    expect(style(0.5)).toBe('opacity: 0.5; scale: 0.75; transform-origin: 10px 20px;');
  });

  it('is a fade only with no scale, and keeps the element’s own origin with no origin', () => {
    expect(scaledFade(undefined, '10px 20px')(0.25)).toBe('opacity: 0.25;');
    expect(scaledFade(0.98, undefined)(0)).toBe('opacity: 0; scale: 0.98;');
  });
});

describe('firstDirection', () => {
  it('MOT-002: a reversal keeps the path of the first direction, so the style at one progress is the same both ways', () => {
    const first = firstDirection<number | undefined>();
    const inward = scaledFade(first(() => 0.96), undefined);
    const outward = scaledFade(first(() => 0.98), undefined);
    for (const t of [0, 0.3, 0.7, 1]) expect(outward(t)).toBe(inward(t));
  });

  it('works out the value once, for the first direction only', () => {
    const first = firstDirection<string>();
    const own = vi.fn(() => 'measured');
    expect(first(own)).toBe('measured');
    expect(first(() => 'other')).toBe('measured');
    expect(own).toHaveBeenCalledTimes(1);
  });

  it('keeps a first value of nothing, so a fade-only start stays a fade', () => {
    const first = firstDirection<number | null>();
    expect(first(() => null)).toBeNull();
    expect(first(() => 0)).toBeNull();
  });
});
