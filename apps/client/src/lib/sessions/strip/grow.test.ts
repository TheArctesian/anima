// @vitest-environment jsdom
import { describe, expect, it } from 'vitest';
import { duration } from '$lib/motion/tokens';
import { grow } from './grow';

const moving = { reduced: false };
const reduced = { reduced: true };

function box(left: number, top: number, width: number, height: number): DOMRect {
  return new DOMRect(left, top, width, height);
}

describe('grow (Session tab open, 5.7.3)', () => {
  const tab = () => {
    const node = document.createElement('div');
    node.getBoundingClientRect = () => box(100, 0, 100, 36);
    return node;
  };
  const origin = { left: 400, top: 300, width: 50, height: 20 };

  it('MOT-005: the tab grows on spring-snappy with transform-origin at the control that opened it', () => {
    const config = grow(tab(), { origin, preference: moving })({ direction: 'in' });
    // Control centre (425, 310) less the tab's corner (100, 0).
    expect(config.css?.(0, 1)).toContain('transform-origin: 325px 310px;');
    expect(config.css?.(0, 1)).toContain('scale: 0.5;');
    expect(config.css?.(1, 0)).toContain('scale: 1;');
    expect(config.duration).toBeGreaterThan(0);
    expect(config.easing?.(1)).toBeCloseTo(1);
  });

  it('MOT-040: under reduced motion the tab fades in place over duration-base', () => {
    const config = grow(tab(), { origin, preference: reduced })({ direction: 'in' });
    expect(config.css?.(0.5, 0.5)).toBe('opacity: 0.5;');
    expect(config.duration).toBe(duration.base);
  });

  it('a tab with no known control fades in', () => {
    const config = grow(tab(), { origin: null, preference: moving })({ direction: 'in' });
    expect(config.css?.(0.5, 0.5)).toBe('opacity: 0.5;');
  });

  it('5.7.3: a closing tab closes to its own place over duration-fast; reduced motion only fades', () => {
    const exit = grow(tab(), { origin, preference: moving })({ direction: 'out' });
    expect(exit.css?.(0.5, 0.5)).toBe('opacity: 0.5; scale: 0.5;');
    expect(exit.css?.(0.5, 0.5)).not.toContain('transform-origin');
    expect(exit.duration).toBe(duration.fast);
    expect(grow(tab(), { origin, preference: reduced })({ direction: 'out' }).css?.(0.5, 0.5)).toBe('opacity: 0.5;');
  });
});
