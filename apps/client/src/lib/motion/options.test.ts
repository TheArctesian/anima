import { describe, expect, it } from 'vitest';
import { modeTokens, springMotion, staggerDelayMs, tweenMotion, type SpringName } from './options';
import { duration, ease, reducedMotion, spring, stagger } from './tokens';

const names: SpringName[] = ['default', 'snappy', 'move', 'momentum'];

describe('springMotion', () => {
  it.each(names)('MOT-003/004: spring %s uses the K5 stiffness and damping with mass 1', (name) => {
    expect(springMotion(name, false)).toEqual({
      type: 'spring',
      stiffness: spring[name].stiffness,
      damping: spring[name].damping,
      mass: 1,
    });
  });

  it('MOT-004: only the momentum spring bounces', () => {
    const bouncing = names.filter((name) => spring[name].dampingRatio < 1);
    expect(bouncing).toEqual(['momentum']);
  });

  it('MOT-031: a release velocity is passed to the spring', () => {
    expect(springMotion('momentum', false, -840)).toMatchObject({ type: 'spring', velocity: -840 });
  });

  it.each(names)('MOT-041: under reduced motion spring %s is instant', (name) => {
    expect(springMotion(name, true, 500)).toEqual({ type: 'instant' });
  });
});

describe('tweenMotion', () => {
  it('MOT-003: an opacity tween takes the duration token and the ease curve', () => {
    expect(tweenMotion('fast', 'exit', false)).toEqual({ type: 'tween', durationMs: duration.fast, ease: ease.exit });
  });

  it('MOT-041: under reduced motion durations come from the reduced table', () => {
    expect(tweenMotion('base', 'enter', true)).toEqual({
      type: 'tween',
      durationMs: reducedMotion.duration.base,
      ease: ease.enter,
    });
    expect(tweenMotion('press', 'enter', true)).toEqual({ type: 'instant' });
  });

  it('a zero duration is instant', () => {
    expect(tweenMotion('instant', 'enter', false)).toEqual({ type: 'instant' });
  });
});

describe('staggerDelayMs', () => {
  it('MOT-040: list items come in one interval apart, up to the item cap', () => {
    const { intervalMs, maxItems } = stagger.list;
    expect(staggerDelayMs(0, false)).toBe(0);
    expect(staggerDelayMs(3, false)).toBe(3 * intervalMs);
    expect(staggerDelayMs(maxItems + 5, false)).toBe((maxItems - 1) * intervalMs);
  });

  it('MOT-041: under reduced motion list items come in together', () => {
    expect(staggerDelayMs(5, true)).toBe(0);
  });
});

describe('modeTokens', () => {
  it('MOT-021: thresholds and gesture constants do not change under reduced motion', () => {
    expect(modeTokens(true).threshold).toEqual(modeTokens(false).threshold);
    expect(modeTokens(true).gesture).toEqual(modeTokens(false).gesture);
  });
});
