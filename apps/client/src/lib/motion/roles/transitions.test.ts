// @vitest-environment jsdom
import { describe, expect, it } from 'vitest';
import type { TransitionConfig } from 'svelte/transition';
import { easeCurve, springCurve } from '../animate';
import { springMotion, staggerDelayMs, type SpringName } from '../options';
import { duration, reducedMotion } from '../tokens';
import { crossFade, listItem, modal, popover, type RoleTransition } from './transitions';

const normal = { preference: { reduced: false } };
const reduced = { preference: { reduced: true } };

function placed(left: number, top: number, width: number, height: number): Element {
  const node = document.createElement('div');
  node.getBoundingClientRect = () => new DOMRect(left, top, width, height);
  return node;
}

const box = () => placed(0, 0, 100, 100);
const entering = (role: RoleTransition) => role({ direction: 'in' });
const leaving = (role: RoleTransition) => role({ direction: 'out' });

function css(config: TransitionConfig, t: number): string {
  return config.css?.(t, 1 - t) ?? '';
}

function expectSpring(config: TransitionConfig, name: SpringName): void {
  const motion = springMotion(name, false);
  if (motion.type !== 'spring') throw new Error('expected a spring');
  const curve = springCurve(motion);
  expect(config.duration).toBeCloseTo(curve.durationMs, 6);
  expect(config.duration).toBeGreaterThan(duration.fast);
  expect(config.easing?.(0.5)).toBeCloseTo(curve.easing(0.5), 6);
}

describe('ease curves', () => {
  it('ease-enter decelerates and ease-exit accelerates', () => {
    expect(easeCurve('enter')(0.5)).toBeGreaterThan(0.5);
    expect(easeCurve('exit')(0.5)).toBeLessThan(0.5);
  });
});

describe('popover', () => {
  it('enters from 0.96 with a fade on spring-snappy', () => {
    const config = entering(popover(box(), normal));
    expect(css(config, 0)).toContain('opacity: 0;');
    expect(css(config, 0)).toContain('scale: 0.96;');
    expect(css(config, 1)).toContain('scale: 1;');
    expectSpring(config, 'snappy');
  });

  it('leaves with a fade and scale to 0.98 over duration-fast with ease-exit', () => {
    const config = leaving(popover(box(), normal));
    expect(config.duration).toBe(duration.fast);
    expect(config.easing?.(0.5)).toBeCloseTo(easeCurve('exit')(0.5), 6);
    expect(css(config, 0)).toContain('scale: 0.98;');
  });

  it.each([
    ['enters', entering],
    ['leaves', leaving],
  ] as const)('MOT-005: %s at the centre of the control that opened it', (_name, direction) => {
    const control = placed(10, 0, 40, 20);
    const config = direction(popover(placed(0, 30, 200, 100), { ...normal, from: control }));
    expect(css(config, 0.5)).toContain('transform-origin: 30px -20px;');
  });
});

describe('modal', () => {
  it('enters from 0.98 on spring-default and leaves the same way over duration-fast', () => {
    const enter = entering(modal(box(), normal));
    expect(css(enter, 0)).toContain('scale: 0.98;');
    expectSpring(enter, 'default');
    const leave = leaving(modal(box(), normal));
    expect(leave.duration).toBe(duration.fast);
    expect(css(leave, 0)).toContain('scale: 0.98;');
  });
});

describe('listItem', () => {
  it('5.7.3 reorder: a new item fades and scales from 0.98 after its stagger delay', () => {
    const config = entering(listItem(box(), { ...normal, index: 3 }));
    expect(config.delay).toBe(staggerDelayMs(3, false));
    expect(config.delay).toBeGreaterThan(0);
    expect(css(config, 0)).toContain('scale: 0.98;');
    expectSpring(config, 'move');
  });

  it('5.7.3 reorder: a leaving item fades in place over duration-fast', () => {
    const config = leaving(listItem(box(), { ...normal, index: 3 }));
    expect(config.duration).toBe(duration.fast);
    expect(config.delay).toBeUndefined();
    expect(css(config, 0.5)).toBe('opacity: 0.5;');
  });

  it('MOT-041: under reduced motion items fade in together', () => {
    const config = entering(listItem(box(), { ...reduced, index: 3 }));
    expect(config.delay).toBe(0);
    expect(css(config, 0.5)).toBe('opacity: 0.5;');
  });
});

describe('reduced motion', () => {
  const roles = [
    ['popover', popover],
    ['modal', modal],
    ['listItem', listItem],
  ] as const;

  it.each(roles)('MOT-040/041: %s is a fade only in both directions', (_name, role) => {
    for (const direction of [entering, leaving]) {
      expect(css(direction(role(box(), reduced)), 0.5)).toBe('opacity: 0.5;');
    }
  });

  it.each(roles)('MOT-041: %s fades in over the reduced duration-base', (_name, role) => {
    expect(entering(role(box(), reduced)).duration).toBe(reducedMotion.duration.base);
  });
});

describe('crossFade', () => {
  it('page change: duration-base in both modes, ease-enter in and ease-exit out', () => {
    expect(entering(crossFade(box(), normal)).duration).toBe(duration.base);
    expect(entering(crossFade(box(), reduced)).duration).toBe(reducedMotion.duration.base);
    expect(entering(crossFade(box(), normal)).easing?.(0.5)).toBeCloseTo(easeCurve('enter')(0.5), 6);
    expect(leaving(crossFade(box(), normal)).easing?.(0.5)).toBeCloseTo(easeCurve('exit')(0.5), 6);
    expect(css(leaving(crossFade(box(), reduced)), 0.25)).toBe('opacity: 0.25;');
  });
});
